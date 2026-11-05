// The Box — Phase 4: Claude Code provider adapter.
//
// Findings this module is built on (captured empirically 2026-07-31, see
// THE_BOX_AUDIT/06_PROVIDER_ADAPTER_SPEC_HE.md for the target interface):
//
//   * `claude -p "<prompt>" --output-format stream-json --verbose` in a given
//     cwd prints one newline-delimited JSON object per line: a `system/init`
//     line carrying `session_id`, zero or more `assistant`/`user` lines for
//     the turn's content and tool results, and a final `result` line with
//     `is_error` + `permission_denials`.
//   * File writes/edits surface as a `user` line whose `tool_use_result.type`
//     is `create` | `update` | `delete` and whose `tool_use_result.filePath`
//     is the touched file — that is the only reliable source for
//     `file_changed` events; there is no separate structured event for it.
//   * In headless (`-p`) mode there is no TTY to answer a permission prompt.
//     A denied tool_use does NOT hang the process — it comes back as an
//     immediate `tool_result` with `is_error: true`, and the run still
//     finishes and reports the denial in the final line's
//     `permission_denials` array. So `startRun` never blocks waiting for a
//     mid-run approval; per-action approval has to be decided in advance
//     (via `--permission-mode` / `--allowedTools`) or brokered by Phase 5 as
//     a *replanning* step after a denied run, not as an in-flight prompt.

use super::ProviderEvent;
use crate::db::repository;
use rusqlite::Connection;
use serde::Serialize;
use serde_json::Value;
use std::io::{BufRead, Read};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

#[cfg(not(test))]
const CLAUDE_BINARY: &str = "claude";

// ------------------------------------------------------------- detectInstalled

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DetectResult {
    pub installed: bool,
    pub version: Option<String>,
    pub path: Option<String>,
}

pub fn detect_installed() -> DetectResult {
    match Command::new("claude").arg("--version").output() {
        Ok(out) if out.status.success() => DetectResult {
            installed: true,
            version: Some(String::from_utf8_lossy(&out.stdout).trim().to_string()),
            path: which_claude(),
        },
        _ => DetectResult {
            installed: false,
            version: None,
            path: None,
        },
    }
}

#[cfg(windows)]
fn which_claude() -> Option<String> {
    let out = Command::new("where").arg("claude").output().ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .next()
        .map(|s| s.trim().to_string())
}

#[cfg(not(windows))]
fn which_claude() -> Option<String> {
    let out = Command::new("which").arg("claude").output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

// -------------------------------------------------------------------- planning
//
// Phase 5: task-plan generation reuses the same `claude` binary in
// `--permission-mode plan` (no tool executes anything in that mode) with
// `--json-schema` to force a structured response — verified for real
// 2026-07-31: the result line's `structured_output` field comes back
// already parsed against the schema, no free-text parsing needed.

const PLAN_JSON_SCHEMA: &str = r#"{"type":"object","properties":{"steps":{"type":"array","items":{"type":"string"}},"risk_level":{"type":"string","enum":["low","medium","high"]}},"required":["steps","risk_level"]}"#;

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedPlan {
    pub steps: Vec<String>,
    pub risk_level: String,
}

pub fn generate_plan(project_path: &str, user_intent: &str) -> Result<GeneratedPlan, String> {
    generate_plan_with_binary(binary_name(), project_path, user_intent)
}

fn generate_plan_with_binary(
    binary: &str,
    project_path: &str,
    user_intent: &str,
) -> Result<GeneratedPlan, String> {
    let output = Command::new(binary)
        .current_dir(project_path)
        .arg("-p")
        .arg(user_intent)
        .arg("--permission-mode")
        .arg("plan")
        .arg("--output-format")
        .arg("json")
        .arg("--json-schema")
        .arg(PLAN_JSON_SCHEMA)
        .arg("--no-session-persistence")
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("failed to launch '{binary}' for planning: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "claude planning run exited with a failure status: {}",
            stderr.trim()
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: Value = serde_json::from_str(stdout.trim())
        .map_err(|e| format!("could not parse planning output as JSON: {e}"))?;

    if v.get("is_error").and_then(Value::as_bool).unwrap_or(false) {
        let msg = v
            .get("result")
            .and_then(Value::as_str)
            .unwrap_or("unknown planning error");
        return Err(format!("claude reported an error while planning: {msg}"));
    }

    let structured = v
        .get("structured_output")
        .ok_or_else(|| "planning result is missing structured_output".to_string())?;

    let steps: Vec<String> = structured
        .get("steps")
        .and_then(Value::as_array)
        .ok_or_else(|| "planning result is missing a steps array".to_string())?
        .iter()
        .filter_map(|s| s.as_str().map(str::to_string))
        .collect();
    if steps.is_empty() {
        return Err("planning produced an empty step list".to_string());
    }

    let risk_level = structured
        .get("risk_level")
        .and_then(Value::as_str)
        .unwrap_or("medium")
        .to_string();

    Ok(GeneratedPlan { steps, risk_level })
}

// ------------------------------------------------------- stream-json translation

/// Pure translation from one raw stdout line to zero or more `ProviderEvent`s.
/// No I/O — this is what the "mock subprocess" tests exercise, feeding it
/// literal JSON lines captured from real `claude -p ... --output-format
/// stream-json` runs instead of spawning a process.
pub fn parse_stream_json_line(line: &str) -> Vec<ProviderEvent> {
    let line = line.trim();
    if line.is_empty() {
        return vec![];
    }
    let v: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    match v.get("type").and_then(Value::as_str) {
        Some("system") => parse_system(&v),
        Some("assistant") => parse_assistant(&v),
        Some("user") => parse_user(&v),
        Some("result") => parse_result(&v),
        _ => vec![],
    }
}

fn parse_system(v: &Value) -> Vec<ProviderEvent> {
    if v.get("subtype").and_then(Value::as_str) != Some("init") {
        return vec![];
    }
    match v.get("session_id").and_then(Value::as_str) {
        Some(sid) => vec![ProviderEvent::SessionId {
            provider_session_id: sid.to_string(),
        }],
        None => vec![],
    }
}

fn parse_assistant(v: &Value) -> Vec<ProviderEvent> {
    let mut events = Vec::new();
    let Some(blocks) = v.pointer("/message/content").and_then(Value::as_array) else {
        return events;
    };
    for block in blocks {
        match block.get("type").and_then(Value::as_str) {
            Some("text") => {
                if let Some(text) = block.get("text").and_then(Value::as_str) {
                    if !text.is_empty() {
                        events.push(ProviderEvent::TextChunk {
                            text: text.to_string(),
                        });
                    }
                }
            }
            Some("tool_use") => {
                let name = block
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string();
                events.push(ProviderEvent::ActionRequested {
                    action: name,
                    // Headless mode already resolved approval via
                    // --permission-mode before this line was ever printed —
                    // see module doc comment. By the time we observe this
                    // event it is informational, not a live decision point.
                    requires_approval: false,
                });
            }
            _ => {}
        }
    }
    events
}

fn parse_user(v: &Value) -> Vec<ProviderEvent> {
    let mut events = Vec::new();
    let Some(tur) = v.get("tool_use_result") else {
        return events;
    };

    if let Some(kind) = tur.get("type").and_then(Value::as_str) {
        let change_type = match kind {
            "create" => Some("created"),
            "update" => Some("modified"),
            "delete" => Some("deleted"),
            _ => None,
        };
        if let (Some(ct), Some(path)) = (change_type, tur.get("filePath").and_then(Value::as_str))
        {
            events.push(ProviderEvent::FileChanged {
                path: path.to_string(),
                change_type: ct.to_string(),
            });
        }
    } else if let Some(s) = tur.as_str() {
        // Permission denials and other tool errors arrive as a bare string
        // here (e.g. "Error: Claude requested permissions to write to
        // ..., but you haven't granted it yet.").
        events.push(ProviderEvent::Log {
            level: "warn".to_string(),
            message: s.to_string(),
        });
    }
    events
}

fn parse_result(v: &Value) -> Vec<ProviderEvent> {
    let mut events = Vec::new();
    if let Some(denials) = v.get("permission_denials").and_then(Value::as_array) {
        for d in denials {
            if let Some(tool) = d.get("tool_name").and_then(Value::as_str) {
                events.push(ProviderEvent::Log {
                    level: "warn".to_string(),
                    message: format!("permission denied for tool: {tool}"),
                });
            }
        }
    }
    let is_error = v.get("is_error").and_then(Value::as_bool).unwrap_or(false);
    events.push(ProviderEvent::Done {
        exit_status: if is_error { "error" } else { "success" }.to_string(),
    });
    events
}

// ------------------------------------------------------------------ spawn + run

/// Scenario 15 ("dangerous action requires approval") in a form that's
/// actually enforceable headlessly: since Phase 4's finding is that
/// headless mode can't pause mid-run for an interactive approval, these
/// patterns are hard-blocked via `--disallowedTools` on *every* run
/// regardless of the plan's risk_level, rather than merely "asked about".
const ALWAYS_DISALLOWED_TOOLS: &[&str] = &[
    "Bash(git push:*)",
    "Bash(rm -rf:*)",
    "Bash(git reset --hard:*)",
];

fn spawn_claude(
    binary: &str,
    project_path: &str,
    prompt: &str,
    permission_mode: &str,
    resume_session_id: Option<&str>,
) -> std::io::Result<Child> {
    let mut cmd = Command::new(binary);
    cmd.current_dir(project_path)
        .arg("-p")
        .arg(prompt)
        .arg("--output-format")
        .arg("stream-json")
        .arg("--verbose")
        .arg("--permission-mode")
        .arg(permission_mode)
        .arg("--no-session-persistence")
        .arg("--disallowedTools")
        .args(ALWAYS_DISALLOWED_TOOLS)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    if let Some(sid) = resume_session_id {
        cmd.arg("--resume").arg(sid);
    }
    cmd.spawn()
}

pub struct RunRequest<'a> {
    pub turn_id: &'a str,
    pub project_path: &'a str,
    pub prompt: &'a str,
    pub permission_mode: &'a str,
    pub resume_session_id: Option<&'a str>,
}

use super::{transition_turn, with_db, RunOutcome};

#[cfg(not(test))]
fn binary_name() -> &'static str {
    CLAUDE_BINARY
}

#[cfg(test)]
fn binary_name() -> &'static str {
    "claude"
}

pub struct StartedRun {
    run_id: String,
    child: Child,
}

impl StartedRun {
    pub fn run_id(&self) -> &str {
        &self.run_id
    }
}

/// Validates the turn is in a startable state, creates the `provider_runs`
/// row, spawns the subprocess, and registers its PID with `control` — all
/// the parts that have to happen before the caller can hand a run_id back
/// to the frontend. Does not touch stdout/stderr yet; that's `drain`'s job.
pub fn start(
    db: Arc<Mutex<Connection>>,
    control: Option<&super::RunControl>,
    req: &RunRequest,
) -> Result<StartedRun, String> {
    start_with_binary(binary_name(), db, control, req)
}

fn start_with_binary(
    binary: &str,
    db: Arc<Mutex<Connection>>,
    control: Option<&super::RunControl>,
    req: &RunRequest,
) -> Result<StartedRun, String> {
    // Reject up front (no provider_run row, no side effects) if the turn
    // isn't actually in a state a run can start from — e.g. re-running an
    // already-`completed` turn by mistake.
    transition_turn(&db, req.turn_id, "running")?;

    let provider_run = with_db(&db, |conn| {
        repository::create_provider_run(conn, req.turn_id, "claude_code", None)
    })?;
    let run_id = provider_run.id.clone();

    let child = match spawn_claude(
        binary,
        req.project_path,
        req.prompt,
        req.permission_mode,
        req.resume_session_id,
    ) {
        Ok(c) => c,
        Err(e) => {
            // Real failure -> real error, never a faked success (doc 06,
            // binding principle 4). This is also the "command doesn't
            // exist" scenario from the Phase 4 manual test plan.
            let message = format!("failed to launch '{binary}': {e}");
            with_db(&db, |conn| {
                repository::create_log(conn, req.turn_id, Some(&run_id), "error", &message)
            })?;
            with_db(&db, |conn| {
                repository::finish_provider_run(conn, &run_id, "error")
            })?;
            transition_turn(&db, req.turn_id, "failed")?;
            return Err(message);
        }
    };

    if let Some(ctrl) = control {
        ctrl.register(&run_id, child.id());
    }

    Ok(StartedRun { run_id, child })
}

/// `start` + `drain` back to back, blocking until the whole turn finishes.
/// Only used by tests, which don't need a cancel button mid-run; the real
/// `run_claude_code` Tauri command (lib.rs) calls `start` and `drain`
/// separately instead, on its own thread, so it can hand back a run_id the
/// moment the process starts rather than after it's already done.
fn run_with_binary(
    binary: &str,
    db: Arc<Mutex<Connection>>,
    app: Option<&tauri::AppHandle>,
    log_dir: Option<&Path>,
    control: Option<&super::RunControl>,
    req: RunRequest,
) -> Result<RunOutcome, String> {
    let started = start_with_binary(binary, db.clone(), control, &req)?;
    drain(started, db, app, log_dir, control, req.turn_id)
}

/// Reads the started subprocess's stdout/stderr to completion, persisting
/// events and the final turn/provider_run status. `turn_id` is passed
/// separately (not as part of a borrowed `RunRequest`) so this can run on
/// its own thread after `start` returns, without needing the original
/// request's borrowed strings to outlive that thread.
pub fn drain(
    started: StartedRun,
    db: Arc<Mutex<Connection>>,
    app: Option<&tauri::AppHandle>,
    log_dir: Option<&Path>,
    control: Option<&super::RunControl>,
    turn_id: &str,
) -> Result<RunOutcome, String> {
    let StartedRun { run_id, mut child } = started;

    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");

    let stderr_handle = std::thread::spawn(move || {
        let mut s = String::new();
        let mut stderr = stderr;
        let _ = stderr.read_to_string(&mut s);
        s
    });

    let mut log_file = log_dir.map(|dir| {
        let _ = std::fs::create_dir_all(dir);
        dir.join(format!("{run_id}.log"))
    });
    // Only keep the path if we can actually create the file.
    if let Some(path) = &log_file {
        if std::fs::File::create(path).is_err() {
            log_file = None;
        }
    }

    let mut session_id: Option<String> = None;
    let mut exit_status: Option<String> = None;

    let reader = std::io::BufReader::new(stdout);
    for line in reader.lines() {
        let Ok(line) = line else { break };
        if let Some(path) = &log_file {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new().append(true).open(path) {
                let _ = writeln!(f, "{line}");
            }
        }

        for event in parse_stream_json_line(&line) {
            match &event {
                ProviderEvent::SessionId { provider_session_id } => {
                    session_id = Some(provider_session_id.clone());
                    let _ = with_db(&db, |conn| {
                        repository::set_provider_run_session(conn, &run_id, provider_session_id)
                    });
                }
                ProviderEvent::FileChanged { path, change_type } => {
                    let _ = with_db(&db, |conn| {
                        repository::create_file_change(
                            conn,
                            turn_id,
                            Some(&run_id),
                            path,
                            change_type,
                            None,
                            None,
                        )
                    });
                }
                ProviderEvent::TextChunk { text } => {
                    let _ = with_db(&db, |conn| {
                        repository::create_log(conn, turn_id, Some(&run_id), "info", text)
                    });
                }
                ProviderEvent::ActionRequested { action, .. } => {
                    let _ = with_db(&db, |conn| {
                        repository::create_log(
                            conn,
                            turn_id,
                            Some(&run_id),
                            "info",
                            &format!("tool_use: {action}"),
                        )
                    });
                }
                ProviderEvent::Log { level, message } => {
                    let _ = with_db(&db, |conn| {
                        repository::create_log(conn, turn_id, Some(&run_id), level, message)
                    });
                }
                ProviderEvent::Error { message, .. } => {
                    let _ = with_db(&db, |conn| {
                        repository::create_log(conn, turn_id, Some(&run_id), "error", message)
                    });
                }
                ProviderEvent::Done { exit_status: es } => {
                    exit_status = Some(es.clone());
                }
            }

            if let Some(app) = app {
                emit_event(app, &run_id, turn_id, &event);
            }
        }
    }

    let _ = child.wait();
    let stderr_output = stderr_handle.join().unwrap_or_default();
    if let Some(ctrl) = control {
        ctrl.unregister(&run_id);
    }

    let final_status = match exit_status {
        Some(s) => s,
        None if control.map(|c| c.was_cancelled(&run_id)).unwrap_or(false) => {
            // The stream ending without a `result` line is expected here —
            // cancel_run killed the process on purpose. Distinguishing this
            // from a real crash (below) is exactly what RunControl's
            // cancelled-set exists for; otherwise both would collapse into
            // an indistinguishable "error".
            with_db(&db, |conn| {
                repository::create_log(conn, turn_id, Some(&run_id), "info", "run cancelled by user")
            })?;
            "cancelled".to_string()
        }
        None => {
            // Stream ended with no `result` line and it wasn't a deliberate
            // cancel — the process died on its own. Real error, not a
            // silent success.
            let message = if stderr_output.trim().is_empty() {
                "claude process exited without a result line".to_string()
            } else {
                format!("claude process exited without a result line; stderr: {}", stderr_output.trim())
            };
            with_db(&db, |conn| {
                repository::create_log(conn, turn_id, Some(&run_id), "error", &message)
            })?;
            if let Some(app) = app {
                emit_event(
                    app,
                    &run_id,
                    turn_id,
                    &ProviderEvent::Error { message, fatal: true },
                );
            }
            "error".to_string()
        }
    };

    with_db(&db, |conn| {
        repository::finish_provider_run(conn, &run_id, &final_status)
    })?;
    transition_turn(
        &db,
        turn_id,
        match final_status.as_str() {
            "success" => "completed",
            "cancelled" => "cancelled",
            _ => "failed",
        },
    )?;

    Ok(RunOutcome {
        provider_run_id: run_id,
        exit_status: final_status,
        session_id,
    })
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct TaggedEvent<'a> {
    run_id: &'a str,
    turn_id: &'a str,
    event: &'a ProviderEvent,
}

fn emit_event(app: &tauri::AppHandle, run_id: &str, turn_id: &str, event: &ProviderEvent) {
    use tauri::Emitter;
    let _ = app.emit(
        "provider_event",
        TaggedEvent {
            run_id,
            turn_id,
            event,
        },
    );
}

// ---------------------------------------------------------------------- tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{init_in_memory, repository};

    // ---- parse_stream_json_line: fixtures captured verbatim from real runs
    // (see module doc comment) ----

    #[test]
    fn parses_session_init() {
        let line = r#"{"type":"system","subtype":"init","cwd":"C:\\demo","session_id":"b07ea9ec-8272-4c76-846f-a9ea1d5772c0","tools":[],"model":"claude-sonnet-5"}"#;
        assert_eq!(
            parse_stream_json_line(line),
            vec![ProviderEvent::SessionId {
                provider_session_id: "b07ea9ec-8272-4c76-846f-a9ea1d5772c0".to_string()
            }]
        );
    }

    #[test]
    fn ignores_non_init_system_lines() {
        let line = r#"{"type":"system","subtype":"thinking_tokens","estimated_tokens":50}"#;
        assert_eq!(parse_stream_json_line(line), vec![]);
    }

    #[test]
    fn parses_assistant_text_chunk() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"pong"}]}}"#;
        assert_eq!(
            parse_stream_json_line(line),
            vec![ProviderEvent::TextChunk { text: "pong".to_string() }]
        );
    }

    #[test]
    fn skips_empty_text_blocks() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"text","text":""}]}}"#;
        assert_eq!(parse_stream_json_line(line), vec![]);
    }

    #[test]
    fn parses_assistant_tool_use() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"toolu_1","name":"Write","input":{"file_path":"x"}}]}}"#;
        assert_eq!(
            parse_stream_json_line(line),
            vec![ProviderEvent::ActionRequested {
                action: "Write".to_string(),
                requires_approval: false,
            }]
        );
    }

    #[test]
    fn ignores_thinking_blocks() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"thinking","thinking":"...","signature":"x"}]}}"#;
        assert_eq!(parse_stream_json_line(line), vec![]);
    }

    #[test]
    fn parses_file_created() {
        let line = r#"{"type":"user","message":{"role":"user","content":[{"tool_use_id":"t1","type":"tool_result","content":"File created successfully at: C:\\demo\\hello.txt"}]},"tool_use_result":{"type":"create","filePath":"C:\\demo\\hello.txt","content":"hi"}}"#;
        assert_eq!(
            parse_stream_json_line(line),
            vec![ProviderEvent::FileChanged {
                path: "C:\\demo\\hello.txt".to_string(),
                change_type: "created".to_string(),
            }]
        );
    }

    #[test]
    fn parses_file_modified() {
        let line = r#"{"type":"user","tool_use_result":{"type":"update","filePath":"C:\\demo\\a.txt"}}"#;
        assert_eq!(
            parse_stream_json_line(line),
            vec![ProviderEvent::FileChanged {
                path: "C:\\demo\\a.txt".to_string(),
                change_type: "modified".to_string(),
            }]
        );
    }

    #[test]
    fn parses_permission_denial_as_warn_log() {
        let line = r#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result","content":"...","is_error":true,"tool_use_id":"t1"}]},"tool_use_result":"Error: Claude requested permissions to write to C:\\demo\\x.txt, but you haven't granted it yet."}"#;
        assert_eq!(
            parse_stream_json_line(line),
            vec![ProviderEvent::Log {
                level: "warn".to_string(),
                message: "Error: Claude requested permissions to write to C:\\demo\\x.txt, but you haven't granted it yet.".to_string(),
            }]
        );
    }

    #[test]
    fn parses_successful_result_as_done() {
        let line = r#"{"is_error":false,"subtype":"success","result":"pong","type":"result","permission_denials":[]}"#;
        assert_eq!(
            parse_stream_json_line(line),
            vec![ProviderEvent::Done { exit_status: "success".to_string() }]
        );
    }

    #[test]
    fn parses_error_result_with_permission_denials() {
        let line = r#"{"is_error":false,"type":"result","permission_denials":[{"tool_name":"Write","tool_use_id":"t1","tool_input":{}}]}"#;
        assert_eq!(
            parse_stream_json_line(line),
            vec![
                ProviderEvent::Log {
                    level: "warn".to_string(),
                    message: "permission denied for tool: Write".to_string(),
                },
                ProviderEvent::Done { exit_status: "success".to_string() },
            ]
        );
    }

    #[test]
    fn malformed_json_line_yields_no_events() {
        assert_eq!(parse_stream_json_line("not json at all"), vec![]);
        assert_eq!(parse_stream_json_line(""), vec![]);
        assert_eq!(parse_stream_json_line("   "), vec![]);
    }

    // ---- spawn failure: real subprocess, deliberately nonexistent binary.
    // This is the "command doesn't exist" scenario from the Phase 4 manual
    // test plan (THE_BOX_AUDIT/07), exercised for real rather than mocked. ----

    #[test]
    fn run_reports_real_error_when_binary_missing() {
        let shared = Arc::new(Mutex::new(init_in_memory().unwrap()));
        let turn = {
            let conn = shared.lock().unwrap();
            let project = repository::create_project(&conn, "Demo", "C:\\demo", None).unwrap();
            let conversation = repository::create_conversation(&conn, &project.id, None).unwrap();
            repository::create_turn(&conn, &conversation.id, 1, "hello").unwrap()
        };

        let result = run_with_binary(
            "this-binary-does-not-exist-xyz123",
            shared.clone(),
            None,
            None,
            None,
            RunRequest {
                turn_id: &turn.id,
                project_path: "C:\\demo",
                prompt: "hello",
                permission_mode: "acceptEdits",
                resume_session_id: None,
            },
        );

        assert!(result.is_err(), "spawning a nonexistent binary must be a real Err, never a faked success");

        let conn = shared.lock().unwrap();
        let refreshed = repository::get_turn(&conn, &turn.id).unwrap().unwrap();
        assert_eq!(refreshed.status, "failed");
    }

    // ---- real end-to-end run against an actual sample project directory,
    // per the Phase 4 manual test plan. Spends real API credits, so it is
    // not part of the default `cargo test` run — see 2026-07-31 session
    // notes / THE_BOX_AUDIT for the one-off verified result. ----

    #[test]
    #[ignore = "spends real API credits; run explicitly with `cargo test -- --ignored real_end_to_end`"]
    fn real_end_to_end_run_against_sample_project() {
        let sample_dir = r"C:\Users\DAVID\AppData\Local\Temp\the-box-e2e-sample";
        let shared = Arc::new(Mutex::new(init_in_memory().unwrap()));
        let turn = {
            let conn = shared.lock().unwrap();
            let project = repository::create_project(&conn, "E2E Sample", sample_dir, None).unwrap();
            let conversation = repository::create_conversation(&conn, &project.id, None).unwrap();
            repository::create_turn(&conn, &conversation.id, 1, "Reply with exactly the word: pong").unwrap()
        };

        let outcome = run_with_binary(
            "claude",
            shared.clone(),
            None,
            None,
            None,
            RunRequest {
                turn_id: &turn.id,
                project_path: sample_dir,
                prompt: "Reply with exactly the word: pong",
                permission_mode: "acceptEdits",
                resume_session_id: None,
            },
        )
        .expect("real claude run should succeed");

        assert_eq!(outcome.exit_status, "success");
        assert!(outcome.session_id.is_some(), "session_id should be captured from the system/init line");

        let conn = shared.lock().unwrap();
        let refreshed_turn = repository::get_turn(&conn, &turn.id).unwrap().unwrap();
        assert_eq!(refreshed_turn.status, "completed");

        let run = repository::get_provider_run(&conn, &outcome.provider_run_id).unwrap().unwrap();
        assert_eq!(run.exit_status.as_deref(), Some("success"));
        assert!(run.ended_at.is_some());
        assert!(run.provider_session_id.is_some());

        let logs = repository::list_logs_for_turn(&conn, &turn.id).unwrap();
        assert!(!logs.is_empty(), "the assistant's text reply should have been persisted as a log entry");
    }

    // ---- real cancel-mid-run: proves cancel_run's PID-kill mechanism
    // actually stops work in flight, not just that it marks the DB row.
    // The prompt sleeps for 20s via the Bash tool (bypassPermissions, only
    // for this isolated disposable test) and only writes done.txt
    // afterward; the test asserts that file never appears. ----

    #[test]
    #[ignore = "spends real API credits and takes several seconds; run explicitly with `cargo test -- --ignored cancel_mid_run`"]
    fn cancel_mid_run_kills_process_and_marks_turn_cancelled() {
        let sample_dir = r"C:\Users\DAVID\AppData\Local\Temp\the-box-cancel-sample";
        let done_marker = std::path::Path::new(sample_dir).join("done.txt");
        let _ = std::fs::remove_file(&done_marker);

        let shared = Arc::new(Mutex::new(init_in_memory().unwrap()));
        let turn_id = {
            let conn = shared.lock().unwrap();
            let project = repository::create_project(&conn, "Cancel Sample", sample_dir, None).unwrap();
            let conversation = repository::create_conversation(&conn, &project.id, None).unwrap();
            repository::create_turn(&conn, &conversation.id, 1, "long task").unwrap().id
        };

        let control = crate::adapters::RunControl::new();
        let control_for_thread = control.clone();
        let db_for_thread = shared.clone();
        let turn_id_for_thread = turn_id.clone();

        let handle = std::thread::spawn(move || {
            run_with_binary(
                "claude",
                db_for_thread,
                None,
                None,
                Some(&control_for_thread),
                RunRequest {
                    turn_id: &turn_id_for_thread,
                    project_path: sample_dir,
                    prompt: "Use the Bash tool to run exactly this command and wait for it to finish: \
                             powershell -Command \"Start-Sleep -Seconds 20\" \
                             — only after that command completes, write a file done.txt containing the word done.",
                    permission_mode: "bypassPermissions",
                    resume_session_id: None,
                },
            )
        });

        // Poll for the provider_run row (created before spawn) so we can
        // find its id without a dedicated "list runs for turn" API.
        let mut run_id: Option<String> = None;
        for _ in 0..50 {
            std::thread::sleep(std::time::Duration::from_millis(200));
            let conn = shared.lock().unwrap();
            let found: Option<String> = conn
                .query_row(
                    "SELECT id FROM provider_runs WHERE turn_id = ?1 LIMIT 1",
                    rusqlite::params![turn_id],
                    |r| r.get(0),
                )
                .ok();
            if found.is_some() {
                run_id = found;
                break;
            }
        }
        let run_id = run_id.expect("provider_run row should appear shortly after the run starts");

        let mut pid: Option<u32> = None;
        for _ in 0..25 {
            if let Some(p) = control.pid_of(&run_id) {
                pid = Some(p);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
        let pid = pid.expect("pid should be registered shortly after spawn succeeds");

        control.mark_cancelled(&run_id);
        let kill = std::process::Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .output()
            .expect("taskkill should run");
        assert!(
            kill.status.success(),
            "taskkill failed: {}",
            String::from_utf8_lossy(&kill.stderr)
        );

        let outcome = handle
            .join()
            .expect("run thread should not panic")
            .expect("run_with_binary should return Ok even when the process was killed");
        assert_eq!(outcome.exit_status, "cancelled");
        assert!(
            !done_marker.exists(),
            "done.txt must never be written — its presence would mean the kill happened after the sleep, not during it"
        );

        let conn = shared.lock().unwrap();
        let refreshed = repository::get_turn(&conn, &turn_id).unwrap().unwrap();
        assert_eq!(refreshed.status, "cancelled");

        let run = repository::get_provider_run(&conn, &outcome.provider_run_id).unwrap().unwrap();
        assert_eq!(run.exit_status.as_deref(), Some("cancelled"));
    }
}
