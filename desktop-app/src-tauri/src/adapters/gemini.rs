// The Box — Phase 10: Gemini provider adapter.
//
// ⚠️ UNLIKE every other adapter in this codebase, the event-shape parsing
// below is NOT independently verified against real output. `gemini
// --help` was inspected for real on 2026-08-04 (the CLI flags referenced
// here — `-p`, `--output-format stream-json`, `--approval-mode`,
// `--session-id`/`--resume`, `-s`/`--sandbox` — are genuinely what this
// installed CLI (gemini 0.51.0) exposes), but actually *running* it
// requires an auth method (`GEMINI_API_KEY` or Google Cloud auth) that
// isn't configured in this environment, so the real JSON payload shape of
// `--output-format stream-json` could not be captured the way it was for
// claude_code.rs/codex.rs/ollama.rs. The parser below is a best-effort
// guess modeled on Claude Code's stream-json shape, since Gemini's CLI
// uses the identical output-format name and Google's docs describe a
// similarly Claude-Code-style agentic loop (system/init, assistant
// messages with content blocks, tool calls, a terminal result). Treat
// every assumption below as unverified until someone with real Gemini
// credentials runs it once and compares. Do not extend this file's
// confidence level to "verified" in commit messages or comments the way
// the other adapters' docs do.
//
// Given that uncertainty, this adapter is intentionally conservative:
// unrecognized line shapes are ignored (return no events) rather than
// guessed at further, and `run` treats "no explicit fatal signal" the
// same defensive way the other adapters do (a stream that ends without an
// explicit terminal event is a real error, never a silent success).

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
fn binary_name() -> &'static str {
    "gemini"
}

#[cfg(test)]
fn binary_name() -> &'static str {
    "gemini"
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
    match Command::new("gemini").arg("--version").output() {
        Ok(out) if out.status.success() => DetectResult {
            installed: true,
            version: Some(String::from_utf8_lossy(&out.stdout).trim().to_string()),
            path: which_gemini(),
        },
        _ => DetectResult {
            installed: false,
            version: None,
            path: None,
        },
    }
}

#[cfg(windows)]
fn which_gemini() -> Option<String> {
    let out = Command::new("where").arg("gemini").output().ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .next()
        .map(|s| s.trim().to_string())
}

#[cfg(not(windows))]
fn which_gemini() -> Option<String> {
    let out = Command::new("which").arg("gemini").output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

// ------------------------------------------------------------ JSON translation
// UNVERIFIED — see module doc comment.

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
        Some("result") => parse_result(&v),
        Some("error") => parse_error(&v),
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
        // Some SDKs put plain text directly on the object as "text"
        // instead of a Claude-style content-block array — handle both
        // shapes defensively since this is unverified territory.
        if let Some(text) = v.get("text").and_then(Value::as_str) {
            if !text.is_empty() {
                events.push(ProviderEvent::TextChunk { text: text.to_string() });
            }
        }
        return events;
    };
    for block in blocks {
        match block.get("type").and_then(Value::as_str) {
            Some("text") => {
                if let Some(text) = block.get("text").and_then(Value::as_str) {
                    if !text.is_empty() {
                        events.push(ProviderEvent::TextChunk { text: text.to_string() });
                    }
                }
            }
            Some("tool_use") | Some("tool_call") | Some("function_call") => {
                let name = block
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string();
                events.push(ProviderEvent::ActionRequested {
                    action: name,
                    requires_approval: false,
                });
            }
            _ => {}
        }
    }
    events
}

fn parse_result(v: &Value) -> Vec<ProviderEvent> {
    let is_error = v.get("is_error").and_then(Value::as_bool).unwrap_or(false);
    vec![ProviderEvent::Done {
        exit_status: if is_error { "error" } else { "success" }.to_string(),
    }]
}

fn parse_error(v: &Value) -> Vec<ProviderEvent> {
    let message = v
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("gemini reported an error")
        .to_string();
    vec![
        ProviderEvent::Log { level: "error".to_string(), message },
        ProviderEvent::Done { exit_status: "error".to_string() },
    ]
}

// ------------------------------------------------------------------ spawn + run

fn spawn_gemini(
    binary: &str,
    project_path: &str,
    prompt: &str,
    resume_session_id: Option<&str>,
) -> std::io::Result<Child> {
    let mut cmd = Command::new(binary);
    cmd.current_dir(project_path)
        .arg("-p")
        .arg(prompt)
        .arg("--output-format")
        .arg("stream-json")
        .arg("--skip-trust")
        // auto_edit, not yolo: mirrors claude_code's acceptEdits default —
        // file edits proceed, but this deliberately does not grant blanket
        // tool approval the way --approval-mode yolo would.
        .arg("--approval-mode")
        .arg("auto_edit")
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
        repository::create_provider_run(conn, req.turn_id, "gemini", None)
    })?;
    let run_id = provider_run.id.clone();

    let child = match spawn_gemini(binary, req.project_path, req.prompt, req.resume_session_id) {
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
            with_db(&db, |conn| {
                repository::create_log(conn, turn_id, Some(&run_id), "info", "run cancelled by user")
            })?;
            "cancelled".to_string()
        }
        None => {
            let message = if stderr_output.trim().is_empty() {
                "gemini process exited without a recognized terminal event — the stream-json shape this adapter expects is unverified (see module doc comment); this could be a real failure or a parser mismatch".to_string()
            } else {
                format!(
                    "gemini process exited without a recognized terminal event; stderr: {}",
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
//
// These are ALL either pure-function tests against *guessed* fixture
// shapes (documented as such, not "captured from a real run" like the
// other adapters) or real-subprocess tests that don't require successful
// auth (spawn-failure path). There is deliberately no real end-to-end test
// here, unlike claude_code/codex/ollama — that requires real Gemini
// credentials this environment doesn't have. Do not mark this adapter as
// fully verified until that test exists and passes for real.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{init_in_memory, repository};

    #[test]
    fn parses_assumed_session_init_shape() {
        let line = r#"{"type":"system","subtype":"init","session_id":"abc-123"}"#;
        assert_eq!(
            parse_stream_json_line(line),
            vec![ProviderEvent::SessionId { provider_session_id: "abc-123".to_string() }]
        );
    }

    #[test]
    fn parses_assumed_assistant_text_shape() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"pong"}]}}"#;
        assert_eq!(
            parse_stream_json_line(line),
            vec![ProviderEvent::TextChunk { text: "pong".to_string() }]
        );
    }

    #[test]
    fn parses_flat_text_field_as_fallback_shape() {
        let line = r#"{"type":"assistant","text":"pong"}"#;
        assert_eq!(
            parse_stream_json_line(line),
            vec![ProviderEvent::TextChunk { text: "pong".to_string() }]
        );
    }

    #[test]
    fn parses_assumed_success_result_shape() {
        let line = r#"{"type":"result","is_error":false}"#;
        assert_eq!(
            parse_stream_json_line(line),
            vec![ProviderEvent::Done { exit_status: "success".to_string() }]
        );
    }

    #[test]
    fn parses_assumed_error_shape() {
        let line = r#"{"type":"error","message":"boom"}"#;
        assert_eq!(
            parse_stream_json_line(line),
            vec![
                ProviderEvent::Log { level: "error".to_string(), message: "boom".to_string() },
                ProviderEvent::Done { exit_status: "error".to_string() },
            ]
        );
    }

    #[test]
    fn malformed_json_line_yields_no_events() {
        assert_eq!(parse_stream_json_line("not json"), vec![]);
        assert_eq!(parse_stream_json_line(""), vec![]);
    }

    // ---- spawn failure: real subprocess, deliberately nonexistent binary.
    // Doesn't need auth, so this one IS a real verified test. ----

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
}
