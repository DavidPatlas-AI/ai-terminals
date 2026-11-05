// The Box — Phase 6: Codex provider adapter.
//
// Findings this module is built on (captured empirically 2026-07-31 against
// codex-cli 0.145.0, same method as claude_code.rs — real runs, not
// guesswork):
//
//   * `codex exec "<prompt>" --json --skip-git-repo-check --sandbox
//     workspace-write` in a given cwd (via `--cd`) prints one JSON object
//     per line: `thread.started` (carries `thread_id`, Codex's equivalent
//     of a session id), zero or more `item.started`/`item.completed`
//     objects, and a terminal `turn.completed` (success) or `turn.failed`
//     (failure, with an `error.message`). The process exit code is 0/1 to
//     match.
//   * Unlike Claude Code's stream-json, there is no single `is_error` flag
//     on the terminal line — success vs failure is which terminal *event
//     type* appears (`turn.completed` vs `turn.failed`), not a field on it.
//   * `item.completed` items carry a `type` of their own: `agent_message`
//     (text), `file_change` (a `changes: [{path, kind}]` array plus overall
//     `status`), `command_execution` (shell, with `command`/`exit_code`/
//     `aggregated_output`), `mcp_tool_call`, or `error` (non-fatal warning,
//     distinct from the fatal top-level `{"type":"error", ...}` event and
//     from `turn.failed`). Only `item.completed` (not `item.started`) is
//     acted on below, to avoid double-counting in-progress vs finished
//     work.
//   * This machine's Codex install has a broken Windows sandbox helper
//     (`codex-windows-sandbox-setup.exe` not found) — real runs here fall
//     back to an MCP `node_repl` tool to get file writes done. That's an
//     environment quirk, not something the adapter needs to special-case:
//     it still ends in a normal `file_change` item either way.

use super::{transition_turn, with_db, ProviderEvent, RunOutcome};
use crate::db::repository;
use rusqlite::Connection;
use serde::Serialize;
use serde_json::Value;
use std::io::{BufRead, Read};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

#[cfg(not(test))]
const CODEX_BINARY: &str = "codex";

#[cfg(not(test))]
fn binary_name() -> &'static str {
    CODEX_BINARY
}

#[cfg(test)]
fn binary_name() -> &'static str {
    "codex"
}

// ------------------------------------------------------------- detectInstalled

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DetectResult {
    pub installed: bool,
    pub version: Option<String>,
    pub path: Option<String>,
}

pub fn detect_installed() -> DetectResult {
    match Command::new("codex").arg("--version").output() {
        Ok(out) if out.status.success() => DetectResult {
            installed: true,
            version: Some(String::from_utf8_lossy(&out.stdout).trim().to_string()),
            path: which_codex(),
        },
        _ => DetectResult {
            installed: false,
            version: None,
            path: None,
        },
    }
}

#[cfg(windows)]
fn which_codex() -> Option<String> {
    let out = Command::new("where").arg("codex").output().ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .next()
        .map(|s| s.trim().to_string())
}

#[cfg(not(windows))]
fn which_codex() -> Option<String> {
    let out = Command::new("which").arg("codex").output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

// ------------------------------------------------------------ JSONL translation

/// Pure translation from one raw stdout line to zero or more
/// `ProviderEvent`s — same role as claude_code::parse_stream_json_line, unit
/// tested against literal fixture lines captured from real `codex exec
/// --json` runs instead of a mocked subprocess.
pub fn parse_json_line(line: &str) -> Vec<ProviderEvent> {
    let line = line.trim();
    if line.is_empty() {
        return vec![];
    }
    let v: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    match v.get("type").and_then(Value::as_str) {
        Some("thread.started") => parse_thread_started(&v),
        Some("item.completed") => parse_item_completed(&v),
        Some("turn.completed") => vec![ProviderEvent::Done {
            exit_status: "success".to_string(),
        }],
        Some("turn.failed") => parse_turn_failed(&v),
        Some("error") => parse_top_level_error(&v),
        // "turn.started", "item.started": no event — see module doc comment.
        _ => vec![],
    }
}

fn parse_thread_started(v: &Value) -> Vec<ProviderEvent> {
    match v.get("thread_id").and_then(Value::as_str) {
        Some(id) => vec![ProviderEvent::SessionId {
            provider_session_id: id.to_string(),
        }],
        None => vec![],
    }
}

fn parse_item_completed(v: &Value) -> Vec<ProviderEvent> {
    let Some(item) = v.get("item") else {
        return vec![];
    };
    match item.get("type").and_then(Value::as_str) {
        Some("agent_message") => match item.get("text").and_then(Value::as_str) {
            Some(text) if !text.is_empty() => vec![ProviderEvent::TextChunk {
                text: text.to_string(),
            }],
            _ => vec![],
        },
        Some("file_change") => parse_file_change_item(item),
        Some("command_execution") => parse_command_execution_item(item),
        Some("mcp_tool_call") => parse_mcp_tool_call_item(item),
        Some("error") => match item.get("message").and_then(Value::as_str) {
            Some(msg) => vec![ProviderEvent::Log {
                level: "warn".to_string(),
                message: msg.to_string(),
            }],
            None => vec![],
        },
        _ => vec![],
    }
}

fn parse_file_change_item(item: &Value) -> Vec<ProviderEvent> {
    let status = item.get("status").and_then(Value::as_str).unwrap_or("");
    let Some(changes) = item.get("changes").and_then(Value::as_array) else {
        return vec![];
    };
    let mut events = Vec::new();
    for change in changes {
        let Some(path) = change.get("path").and_then(Value::as_str) else {
            continue;
        };
        if status == "completed" {
            let change_type = match change.get("kind").and_then(Value::as_str) {
                Some("add") => "created",
                Some("delete") => "deleted",
                _ => "modified",
            };
            events.push(ProviderEvent::FileChanged {
                path: path.to_string(),
                change_type: change_type.to_string(),
            });
        } else if status == "failed" {
            events.push(ProviderEvent::Log {
                level: "warn".to_string(),
                message: format!("file_change failed: {path}"),
            });
        }
    }
    events
}

fn parse_command_execution_item(item: &Value) -> Vec<ProviderEvent> {
    let command = item
        .get("command")
        .and_then(Value::as_str)
        .unwrap_or("<unknown command>");
    let exit_code = item.get("exit_code").and_then(Value::as_i64);
    let mut events = vec![ProviderEvent::ActionRequested {
        action: command.to_string(),
        requires_approval: false,
    }];
    let level = match exit_code {
        Some(0) => "info",
        _ => "warn",
    };
    let output = item.get("aggregated_output").and_then(Value::as_str).unwrap_or("");
    events.push(ProviderEvent::Log {
        level: level.to_string(),
        message: format!("command exit_code={exit_code:?}: {command}\n{output}"),
    });
    events
}

fn parse_mcp_tool_call_item(item: &Value) -> Vec<ProviderEvent> {
    let server = item.get("server").and_then(Value::as_str).unwrap_or("?");
    let tool = item.get("tool").and_then(Value::as_str).unwrap_or("?");
    let has_error = item.get("error").map(|e| !e.is_null()).unwrap_or(false);
    vec![
        ProviderEvent::ActionRequested {
            action: format!("mcp:{server}/{tool}"),
            requires_approval: false,
        },
        ProviderEvent::Log {
            level: if has_error { "warn" } else { "info" }.to_string(),
            message: format!("mcp_tool_call {server}/{tool} completed"),
        },
    ]
}

fn parse_turn_failed(v: &Value) -> Vec<ProviderEvent> {
    let message = v
        .pointer("/error/message")
        .and_then(Value::as_str)
        .unwrap_or("codex turn failed")
        .to_string();
    vec![
        ProviderEvent::Log {
            level: "error".to_string(),
            message,
        },
        ProviderEvent::Done {
            exit_status: "error".to_string(),
        },
    ]
}

fn parse_top_level_error(v: &Value) -> Vec<ProviderEvent> {
    match v.get("message").and_then(Value::as_str) {
        Some(msg) => vec![ProviderEvent::Log {
            level: "error".to_string(),
            message: msg.to_string(),
        }],
        None => vec![],
    }
}

// ------------------------------------------------------------------ spawn + run

fn spawn_codex(
    binary: &str,
    project_path: &str,
    prompt: &str,
    resume_session_id: Option<&str>,
) -> std::io::Result<Child> {
    build_codex_command(binary, project_path, prompt, resume_session_id)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .spawn()
}

/// Split out from `spawn_codex` purely so the argv ordering can be
/// unit-tested via `Command`'s `Debug` output without spawning a real
/// process — see the `--` comment below for the bug this guards against.
fn build_codex_command(
    binary: &str,
    project_path: &str,
    prompt: &str,
    resume_session_id: Option<&str>,
) -> Command {
    let mut cmd = Command::new(binary);
    // Deliberately never pass --dangerously-bypass-approvals-and-sandbox:
    // per Codex's own help text that flag disables sandboxing entirely, not
    // just approval prompts — a materially bigger blast radius than
    // Claude's `bypassPermissions`. `workspace-write` is used for every
    // permission_mode value; this is a Codex-specific, intentionally
    // coarser safety choice than claude_code's per-tool
    // --disallowedTools blocklist (Codex's exec CLI has no equivalent flag
    // in this version).
    if let Some(sid) = resume_session_id {
        cmd.arg("exec").arg("resume");
        cmd.arg(sid); // session id: always a UUID, never dash-prefixed
    } else {
        cmd.arg("exec");
    }
    cmd.arg("--json")
        .arg("--skip-git-repo-check")
        .arg("--sandbox")
        .arg("workspace-write")
        .arg("--cd")
        .arg(project_path);
    // `--` immediately before the prompt, as the LAST argument, is
    // required, not decorative: codex's `exec` takes PROMPT as a bare
    // positional (unlike Claude Code's `-p <value>`), so a prompt that
    // happens to start with `-` — like a handoff summary opening with
    // "--- הקשר קודם ..." — gets misparsed as an unrecognized flag and
    // codex exits before printing a single line of JSON. It has to come
    // after every real flag above: anything placed after `--` is treated
    // as positional text too, so putting it earlier would swallow
    // `--json`/`--sandbox`/etc. as part of the prompt instead. Verified
    // empirically 2026-07-31 while debugging exactly that failure in the
    // real cross-provider handoff test.
    cmd.arg("--").arg(prompt);
    cmd
}

pub struct RunRequest<'a> {
    pub turn_id: &'a str,
    pub project_path: &'a str,
    pub prompt: &'a str,
    pub resume_session_id: Option<&'a str>,
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
    transition_turn(&db, req.turn_id, "running")?;

    let provider_run = with_db(&db, |conn| {
        repository::create_provider_run(conn, req.turn_id, "codex", None)
    })?;
    let run_id = provider_run.id.clone();

    let child = match spawn_codex(binary, req.project_path, req.prompt, req.resume_session_id) {
        Ok(c) => c,
        Err(e) => {
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

        for event in parse_json_line(&line) {
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
            with_db(&db, |conn| {
                repository::create_log(conn, turn_id, Some(&run_id), "info", "run cancelled by user")
            })?;
            "cancelled".to_string()
        }
        None => {
            let message = if stderr_output.trim().is_empty() {
                "codex process exited without a turn.completed/turn.failed line".to_string()
            } else {
                format!(
                    "codex process exited without a turn.completed/turn.failed line; stderr: {}",
                    stderr_output.trim()
                )
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

/// `start` + `drain` back to back, blocking until the whole turn finishes.
/// Only used by tests — see claude_code::run_with_binary for why the real
/// Tauri command doesn't use this shape.
#[cfg(test)]
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

    // ---- parse_json_line: fixtures captured verbatim from real `codex exec
    // --json` runs (see module doc comment) ----

    #[test]
    fn parses_thread_started_as_session_id() {
        let line = r#"{"type":"thread.started","thread_id":"019fb80e-040c-7d61-bd97-bde8b4c56a3a"}"#;
        assert_eq!(
            parse_json_line(line),
            vec![ProviderEvent::SessionId {
                provider_session_id: "019fb80e-040c-7d61-bd97-bde8b4c56a3a".to_string()
            }]
        );
    }

    #[test]
    fn ignores_turn_started() {
        assert_eq!(parse_json_line(r#"{"type":"turn.started"}"#), vec![]);
    }

    #[test]
    fn parses_agent_message_as_text_chunk() {
        let line = r#"{"type":"item.completed","item":{"id":"item_0","type":"agent_message","text":"pong"}}"#;
        assert_eq!(
            parse_json_line(line),
            vec![ProviderEvent::TextChunk { text: "pong".to_string() }]
        );
    }

    #[test]
    fn ignores_item_started_for_agent_message() {
        let line = r#"{"type":"item.started","item":{"id":"item_0","type":"agent_message","text":"pong"}}"#;
        assert_eq!(parse_json_line(line), vec![]);
    }

    #[test]
    fn parses_completed_file_change_as_file_changed() {
        let line = r#"{"type":"item.completed","item":{"id":"item_1","type":"file_change","changes":[{"path":"C:\\demo\\hello.txt","kind":"add"}],"status":"completed"}}"#;
        assert_eq!(
            parse_json_line(line),
            vec![ProviderEvent::FileChanged {
                path: "C:\\demo\\hello.txt".to_string(),
                change_type: "created".to_string(),
            }]
        );
    }

    #[test]
    fn parses_failed_file_change_as_warn_log_not_file_changed() {
        let line = r#"{"type":"item.completed","item":{"id":"item_1","type":"file_change","changes":[{"path":"C:\\demo\\hello.txt","kind":"add"}],"status":"failed"}}"#;
        assert_eq!(
            parse_json_line(line),
            vec![ProviderEvent::Log {
                level: "warn".to_string(),
                message: "file_change failed: C:\\demo\\hello.txt".to_string(),
            }]
        );
    }

    #[test]
    fn parses_command_execution_as_action_and_log() {
        let line = r#"{"type":"item.completed","item":{"id":"item_3","type":"command_execution","command":"echo hi","aggregated_output":"hi\n","exit_code":0,"status":"completed"}}"#;
        let events = parse_json_line(line);
        assert_eq!(events.len(), 2);
        assert_eq!(
            events[0],
            ProviderEvent::ActionRequested { action: "echo hi".to_string(), requires_approval: false }
        );
        assert!(matches!(&events[1], ProviderEvent::Log { level, .. } if level == "info"));
    }

    #[test]
    fn command_execution_nonzero_exit_logs_as_warn() {
        let line = r#"{"type":"item.completed","item":{"id":"item_3","type":"command_execution","command":"false","aggregated_output":"","exit_code":1,"status":"failed"}}"#;
        let events = parse_json_line(line);
        assert!(matches!(&events[1], ProviderEvent::Log { level, .. } if level == "warn"));
    }

    #[test]
    fn parses_turn_completed_as_success_done() {
        let line = r#"{"type":"turn.completed","usage":{"input_tokens":1}}"#;
        assert_eq!(
            parse_json_line(line),
            vec![ProviderEvent::Done { exit_status: "success".to_string() }]
        );
    }

    #[test]
    fn parses_turn_failed_as_error_log_then_done() {
        let line = r#"{"type":"turn.failed","error":{"message":"model not supported"}}"#;
        assert_eq!(
            parse_json_line(line),
            vec![
                ProviderEvent::Log { level: "error".to_string(), message: "model not supported".to_string() },
                ProviderEvent::Done { exit_status: "error".to_string() },
            ]
        );
    }

    #[test]
    fn parses_top_level_error_as_error_log() {
        let line = r#"{"type":"error","message":"boom"}"#;
        assert_eq!(
            parse_json_line(line),
            vec![ProviderEvent::Log { level: "error".to_string(), message: "boom".to_string() }]
        );
    }

    // ---- regression test for the real bug found 2026-07-31: a prompt
    // starting with '-' (a handoff summary opening with "---") was
    // misparsed by codex's positional-argument PROMPT as an unknown flag,
    // so codex exited with zero JSON output before doing anything. ----

    #[test]
    fn dash_prefixed_prompt_is_placed_after_a_bare_separator() {
        let cmd = build_codex_command("codex", "C:\\demo", "--- looks like a flag but isn't", None);
        let debug = format!("{cmd:?}");
        let sep_pos = debug.find("\"--\"").expect("command must contain a bare -- separator");
        let prompt_pos = debug
            .find("\"--- looks like a flag but isn't\"")
            .expect("prompt must appear as its own argument");
        assert!(sep_pos < prompt_pos, "-- must come immediately before the prompt: {debug}");
        // Nothing after the "--" *up to the prompt* may be another flag —
        // i.e. -- must be the argument directly preceding the prompt, not
        // just somewhere earlier in the command.
        assert_eq!(
            &debug[sep_pos..prompt_pos],
            "\"--\" ",
            "-- must be immediately followed by the prompt, not another flag: {debug}"
        );
    }

    #[test]
    fn resume_puts_session_id_before_flags_and_prompt_last() {
        let cmd = build_codex_command("codex", "C:\\demo", "continue please", Some("sess-123"));
        let debug = format!("{cmd:?}");
        let resume_pos = debug.find("\"resume\"").unwrap();
        let sid_pos = debug.find("\"sess-123\"").unwrap();
        let sep_pos = debug.find("\"--\"").unwrap();
        let prompt_pos = debug.find("\"continue please\"").unwrap();
        assert!(resume_pos < sid_pos, "session id must follow 'resume': {debug}");
        assert!(sid_pos < sep_pos, "-- must come after the session id: {debug}");
        assert!(sep_pos < prompt_pos, "prompt must be the last argument, after --: {debug}");
    }

    #[test]
    fn malformed_json_line_yields_no_events() {
        assert_eq!(parse_json_line("not json"), vec![]);
        assert_eq!(parse_json_line(""), vec![]);
    }

    // ---- spawn failure: real subprocess, deliberately nonexistent binary,
    // mirroring claude_code's equivalent test. ----

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
                resume_session_id: None,
            },
        );

        assert!(result.is_err());
        let conn = shared.lock().unwrap();
        let refreshed = repository::get_turn(&conn, &turn.id).unwrap().unwrap();
        assert_eq!(refreshed.status, "failed");
    }

    // ---- real end-to-end run, mirroring claude_code's. Spends real API
    // credits, so not part of the default `cargo test` run. ----

    #[test]
    #[ignore = "spends real API credits; run explicitly with `cargo test -- --ignored codex_real_end_to_end`"]
    fn codex_real_end_to_end_run_against_sample_project() {
        let sample_dir = r"C:\Users\DAVID\AppData\Local\Temp\the-box-e2e-sample";
        let shared = Arc::new(Mutex::new(init_in_memory().unwrap()));
        let turn = {
            let conn = shared.lock().unwrap();
            let project = repository::create_project(&conn, "E2E Sample", sample_dir, None).unwrap();
            let conversation = repository::create_conversation(&conn, &project.id, None).unwrap();
            repository::create_turn(&conn, &conversation.id, 1, "Reply with exactly the word: pong").unwrap()
        };

        let outcome = run_with_binary(
            "codex",
            shared.clone(),
            None,
            None,
            None,
            RunRequest {
                turn_id: &turn.id,
                project_path: sample_dir,
                prompt: "Reply with exactly the word: pong",
                resume_session_id: None,
            },
        )
        .expect("real codex run should succeed");

        assert_eq!(outcome.exit_status, "success");
        assert!(outcome.session_id.is_some());

        let conn = shared.lock().unwrap();
        let refreshed_turn = repository::get_turn(&conn, &turn.id).unwrap().unwrap();
        assert_eq!(refreshed_turn.status, "completed");
    }

    // ---- real cross-provider handoff (scenarios 12-13, doc 08): Claude
    // Code runs first and leaves a real, checkable trace (a file containing
    // a token nothing in this test hardcodes anywhere else); the handoff
    // summary is built from what actually happened in the DB, not fixture
    // data; Codex is then asked to report the file's contents *without
    // being told what the file is* — it can only succeed by having actually
    // used the handoff summary. Spends real API credits (2 calls), not part
    // of the default `cargo test` run. ----

    #[test]
    #[ignore = "spends real API credits (2 calls); run explicitly with `cargo test -- --ignored handoff_from_claude`"]
    fn handoff_from_claude_code_reaches_codex_with_real_context() {
        let sample_dir = r"C:\Users\DAVID\AppData\Local\Temp\the-box-handoff-sample";
        let shared = Arc::new(Mutex::new(init_in_memory().unwrap()));
        let turn = {
            let conn = shared.lock().unwrap();
            let project = repository::create_project(&conn, "Handoff Sample", sample_dir, None).unwrap();
            let conversation = repository::create_conversation(&conn, &project.id, None).unwrap();
            repository::create_turn(
                &conn,
                &conversation.id,
                1,
                "Create a file named NOTES.txt containing exactly this text and nothing else: SECRET-TOKEN-77",
            )
            .unwrap()
        };

        // Step 1: Claude Code does the actual work.
        let claude_started = crate::adapters::claude_code::start(
            shared.clone(),
            None,
            &crate::adapters::claude_code::RunRequest {
                turn_id: &turn.id,
                project_path: sample_dir,
                prompt: "Create a file named NOTES.txt containing exactly this text and nothing else: SECRET-TOKEN-77",
                permission_mode: "acceptEdits",
                resume_session_id: None,
            },
        )
        .expect("claude start should succeed");
        let claude_outcome = crate::adapters::claude_code::drain(
            claude_started,
            shared.clone(),
            None,
            None,
            None,
            &turn.id,
        )
        .expect("claude drain should succeed");
        assert_eq!(claude_outcome.exit_status, "success");
        {
            let conn = shared.lock().unwrap();
            assert_eq!(repository::get_turn(&conn, &turn.id).unwrap().unwrap().status, "completed");
        }

        // Step 2: build the handoff summary from real DB state left by step 1.
        let summary = {
            let conn = shared.lock().unwrap();
            crate::core::handoff::build_summary(&conn, &turn.id).unwrap()
        };
        assert!(summary.contains("NOTES.txt"), "handoff summary must mention the file Claude actually created");
        assert!(
            summary.contains("SECRET-TOKEN-77") || summary.contains("Create a file named NOTES.txt"),
            "handoff summary must carry the original intent forward"
        );

        // Step 3: Codex continues the SAME turn. This only works if
        // completed -> running is a legal transition (Phase 6 finding, see
        // core::turn_status).
        let codex_prompt = format!(
            "{summary}\nRead the file mentioned above (do not create a new one) and reply with exactly its contents, nothing else."
        );
        let outcome = run_with_binary(
            "codex",
            shared.clone(),
            None,
            None,
            None,
            RunRequest {
                turn_id: &turn.id,
                project_path: sample_dir,
                prompt: &codex_prompt,
                resume_session_id: None,
            },
        )
        .expect("codex handoff run should succeed");
        assert_eq!(outcome.exit_status, "success");

        let conn = shared.lock().unwrap();
        assert_eq!(repository::get_turn(&conn, &turn.id).unwrap().unwrap().status, "completed");

        let logs = repository::list_logs_for_turn(&conn, &turn.id).unwrap();
        let codex_saw_the_secret = logs.iter().any(|l| l.message.contains("SECRET-TOKEN-77"));
        assert!(
            codex_saw_the_secret,
            "Codex's reply should contain the token from the file Claude Code created — proof it actually used the handoff context, not a generic reply. Logs: {logs:#?}"
        );
    }
}
