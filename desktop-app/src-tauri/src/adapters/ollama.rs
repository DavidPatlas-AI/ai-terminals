// The Box — Phase 10: Ollama provider adapter.
//
// Findings this module is built on (captured empirically 2026-08-04):
//
//   * The `ollama run <model>` CLI is built for interactive terminal use —
//     even when its stdin/stdout are piped (as a subprocess must), it still
//     emits a stream of ANSI cursor/spinner control codes mixed into
//     stdout, with no clean "headless" flag equivalent to Claude Code's
//     `-p` or Codex's `exec --json`. It's not usable for programmatic
//     parsing.
//   * Ollama's actual programmatic interface is its local REST API
//     (`http://localhost:11434/api/generate`), which the desktop app
//     already runs in the background once installed. POSTing
//     `{"model":..., "prompt":..., "stream":true}` returns clean
//     newline-delimited JSON: one `{"response":"<token>","done":false}`
//     line per token, and a final `{"done":true,"done_reason":"stop",...}`
//     line — or `{"error":"..."}` for a real failure (e.g. unknown model).
//   * `curl` is used as the subprocess (not a new Rust HTTP dependency) to
//     stay consistent with claude_code.rs/codex.rs's architecture — the
//     request body is written to curl's own stdin (`--data-binary @-`)
//     rather than passed as a CLI argument, both to avoid any argument
//     length/escaping issues with arbitrary prompt content and because
//     `curl -N` (no-buffer) reliably streams the NDJSON response line by
//     line as it arrives, verified by hand with a multi-sentence prompt.
//   * Ollama has no file-editing or shell-tool capability in this mode —
//     it's a plain chat completion, not an agentic coding tool like Claude
//     Code/Codex. So this adapter only ever emits TextChunk and Done —
//     never FileChanged or a meaningful ActionRequested. There is also no
//     provider-side session/thread id to capture (Ollama's own
//     multi-turn `context` array is a very different mechanism, not
//     analogous to Claude's/Codex's session ids, and resume isn't
//     implemented for it in this phase).

use super::{transition_turn, with_db, ProviderEvent, RunOutcome};
use crate::db::repository;
use rusqlite::Connection;
use serde::Serialize;
use serde_json::Value;
use std::io::{BufRead, Read, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

const OLLAMA_API_URL: &str = "http://localhost:11434/api/generate";
const DEFAULT_MODEL: &str = "llama3.2";

#[cfg(not(test))]
fn binary_name() -> &'static str {
    "curl"
}

#[cfg(test)]
fn binary_name() -> &'static str {
    "curl"
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
    match Command::new("ollama").arg("--version").output() {
        Ok(out) if out.status.success() => DetectResult {
            installed: true,
            version: Some(String::from_utf8_lossy(&out.stdout).trim().to_string()),
            path: which_ollama(),
        },
        _ => DetectResult {
            installed: false,
            version: None,
            path: None,
        },
    }
}

#[cfg(windows)]
fn which_ollama() -> Option<String> {
    let out = Command::new("where").arg("ollama").output().ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .next()
        .map(|s| s.trim().to_string())
}

#[cfg(not(windows))]
fn which_ollama() -> Option<String> {
    let out = Command::new("which").arg("ollama").output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

// ------------------------------------------------------------ NDJSON translation

/// Pure translation from one raw stdout line to zero or more
/// `ProviderEvent`s — same role as the other adapters' line parsers, unit
/// tested against literal fixture lines captured from a real
/// `/api/generate` streaming response.
pub fn parse_ndjson_line(line: &str) -> Vec<ProviderEvent> {
    let line = line.trim();
    if line.is_empty() {
        return vec![];
    }
    let v: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    if let Some(err) = v.get("error").and_then(Value::as_str) {
        return vec![
            ProviderEvent::Log {
                level: "error".to_string(),
                message: err.to_string(),
            },
            ProviderEvent::Done {
                exit_status: "error".to_string(),
            },
        ];
    }

    let mut events = Vec::new();
    if let Some(text) = v.get("response").and_then(Value::as_str) {
        if !text.is_empty() {
            events.push(ProviderEvent::TextChunk {
                text: text.to_string(),
            });
        }
    }
    if v.get("done").and_then(Value::as_bool) == Some(true) {
        events.push(ProviderEvent::Done {
            exit_status: "success".to_string(),
        });
    }
    events
}

// ------------------------------------------------------------------ spawn + run

fn spawn_ollama(binary: &str, model: &str, prompt: &str) -> std::io::Result<Child> {
    let mut child = Command::new(binary)
        .arg("-s") // silent: no progress meter mixed into stdout
        .arg("-f") // fail on HTTP error status instead of printing the error body as a "success"
        .arg("-N") // no-buffer: stream NDJSON lines as they arrive
        .arg("-X")
        .arg("POST")
        .arg(OLLAMA_API_URL)
        .arg("-H")
        .arg("Content-Type: application/json")
        .arg("--data-binary")
        .arg("@-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // The request body goes on curl's stdin rather than as a CLI argument —
    // sidesteps both shell-argument-length limits and the exact class of
    // "argument starts with a special character" bug found in the Codex
    // adapter, and serde_json handles escaping arbitrary prompt content
    // (quotes, newlines, Hebrew text) correctly regardless.
    let body = serde_json::json!({ "model": model, "prompt": prompt, "stream": true }).to_string();
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(body.as_bytes());
        // stdin is dropped here, closing it — curl needs EOF on stdin to
        // know the request body is complete.
    }

    Ok(child)
}

pub struct RunRequest<'a> {
    pub turn_id: &'a str,
    pub prompt: &'a str,
    pub model: Option<&'a str>,
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
        repository::create_provider_run(conn, req.turn_id, "ollama", None)
    })?;
    let run_id = provider_run.id.clone();

    let model = req.model.unwrap_or(DEFAULT_MODEL);
    let child = match spawn_ollama(binary, model, req.prompt) {
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

    let mut exit_status: Option<String> = None;

    let reader = std::io::BufReader::new(stdout);
    for line in reader.lines() {
        let Ok(line) = line else { break };
        if let Some(path) = &log_file {
            use std::io::Write as _;
            if let Ok(mut f) = std::fs::OpenOptions::new().append(true).open(path) {
                let _ = writeln!(f, "{line}");
            }
        }

        for event in parse_ndjson_line(&line) {
            match &event {
                ProviderEvent::TextChunk { text } => {
                    let _ = with_db(&db, |conn| {
                        repository::create_log(conn, turn_id, Some(&run_id), "info", text)
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
                // Never produced by this adapter, kept exhaustive so a
                // future ProviderEvent variant doesn't silently go
                // unhandled here.
                ProviderEvent::FileChanged { .. }
                | ProviderEvent::ActionRequested { .. }
                | ProviderEvent::SessionId { .. } => {}
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
                "ollama request exited without a done:true line — is the Ollama app/service running?".to_string()
            } else {
                format!(
                    "ollama request exited without a done:true line; stderr: {}",
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
        session_id: None,
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

    // ---- parse_ndjson_line: fixtures captured verbatim from a real
    // Ollama /api/generate streaming response (see module doc comment) ----

    #[test]
    fn parses_a_token_chunk() {
        let line = r#"{"model":"llama3.2","created_at":"2026-08-04T17:41:22.2353739Z","response":"P","done":false}"#;
        assert_eq!(
            parse_ndjson_line(line),
            vec![ProviderEvent::TextChunk { text: "P".to_string() }]
        );
    }

    #[test]
    fn skips_empty_response_chunks() {
        let line = r#"{"model":"llama3.2","response":"","done":false}"#;
        assert_eq!(parse_ndjson_line(line), vec![]);
    }

    #[test]
    fn parses_final_done_line_as_success() {
        let line = r#"{"model":"llama3.2","response":"","done":true,"done_reason":"stop","total_duration":1264199700}"#;
        assert_eq!(
            parse_ndjson_line(line),
            vec![ProviderEvent::Done { exit_status: "success".to_string() }]
        );
    }

    #[test]
    fn final_chunk_with_trailing_text_emits_both_text_and_done() {
        let line = r#"{"model":"llama3.2","response":"!","done":true}"#;
        assert_eq!(
            parse_ndjson_line(line),
            vec![
                ProviderEvent::TextChunk { text: "!".to_string() },
                ProviderEvent::Done { exit_status: "success".to_string() },
            ]
        );
    }

    #[test]
    fn parses_error_body_as_error_log_then_done() {
        let line = r#"{"error":"model 'this-model-does-not-exist' not found"}"#;
        assert_eq!(
            parse_ndjson_line(line),
            vec![
                ProviderEvent::Log { level: "error".to_string(), message: "model 'this-model-does-not-exist' not found".to_string() },
                ProviderEvent::Done { exit_status: "error".to_string() },
            ]
        );
    }

    #[test]
    fn malformed_json_line_yields_no_events() {
        assert_eq!(parse_ndjson_line("not json"), vec![]);
        assert_eq!(parse_ndjson_line(""), vec![]);
    }

    // ---- spawn failure: real subprocess, deliberately nonexistent binary,
    // mirroring the other adapters' equivalent test. ----

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
                prompt: "hello",
                model: None,
            },
        );

        assert!(result.is_err());
        let conn = shared.lock().unwrap();
        let refreshed = repository::get_turn(&conn, &turn.id).unwrap().unwrap();
        assert_eq!(refreshed.status, "failed");
    }

    // ---- real end-to-end run against the actual local Ollama service.
    // Free (no API cost, purely local), but still needs Ollama actually
    // running with the llama3.2 model pulled, so kept #[ignore] to avoid
    // breaking `cargo test` on a machine without Ollama set up. ----

    #[test]
    #[ignore = "needs a real local Ollama service with llama3.2 pulled; run explicitly with `cargo test -- --ignored ollama_real_end_to_end`"]
    fn ollama_real_end_to_end_run() {
        let shared = Arc::new(Mutex::new(init_in_memory().unwrap()));
        let turn = {
            let conn = shared.lock().unwrap();
            let project = repository::create_project(&conn, "Demo", "C:\\demo", None).unwrap();
            let conversation = repository::create_conversation(&conn, &project.id, None).unwrap();
            repository::create_turn(&conn, &conversation.id, 1, "Reply with exactly the word: pong").unwrap()
        };

        let outcome = run_with_binary(
            "curl",
            shared.clone(),
            None,
            None,
            None,
            RunRequest {
                turn_id: &turn.id,
                prompt: "Reply with exactly the word: pong",
                model: None,
            },
        )
        .expect("real ollama run should succeed");

        assert_eq!(outcome.exit_status, "success");

        let conn = shared.lock().unwrap();
        let refreshed_turn = repository::get_turn(&conn, &turn.id).unwrap().unwrap();
        assert_eq!(refreshed_turn.status, "completed");

        let logs = repository::list_logs_for_turn(&conn, &turn.id).unwrap();
        assert!(!logs.is_empty(), "streamed text chunks should have been persisted as log entries");
    }

    #[test]
    #[ignore = "needs a real local Ollama service; run explicitly with `cargo test -- --ignored ollama_real_error`"]
    fn ollama_real_error_for_unknown_model() {
        let shared = Arc::new(Mutex::new(init_in_memory().unwrap()));
        let turn = {
            let conn = shared.lock().unwrap();
            let project = repository::create_project(&conn, "Demo", "C:\\demo", None).unwrap();
            let conversation = repository::create_conversation(&conn, &project.id, None).unwrap();
            repository::create_turn(&conn, &conversation.id, 1, "hi").unwrap()
        };

        let outcome = run_with_binary(
            "curl",
            shared.clone(),
            None,
            None,
            None,
            RunRequest {
                turn_id: &turn.id,
                prompt: "hi",
                model: Some("this-model-does-not-exist-xyz123"),
            },
        )
        .expect("run_with_binary itself should not Err — the failure is reported via exit_status");

        assert_eq!(outcome.exit_status, "error", "an unknown model must be a real failure, never a faked success");

        let conn = shared.lock().unwrap();
        let refreshed_turn = repository::get_turn(&conn, &turn.id).unwrap().unwrap();
        assert_eq!(refreshed_turn.status, "failed");
    }
}
