// The Box — Phase 4 provider adapters.
//
// Interface and event shape follow THE_BOX_AUDIT/06_PROVIDER_ADAPTER_SPEC_HE.md
// ("ProviderAdapter" / "ProviderEvent"). Claude Code is the only adapter
// built in Phase 4 (Codex/Gemini/Ollama are Phase 6/10); the module is
// structured so those can be added as siblings later without touching this
// file's event enum.

pub mod connected;
pub mod claude_code;
pub mod codex;
pub mod gemini;
pub mod ollama;

use rusqlite::Connection;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

/// Shared by every adapter's `drain`: the outcome of one provider_run.
pub struct RunOutcome {
    pub provider_run_id: String,
    pub exit_status: String,
    pub session_id: Option<String>,
}

fn with_db<T>(
    db: &Arc<Mutex<Connection>>,
    f: impl FnOnce(&Connection) -> rusqlite::Result<T>,
) -> Result<T, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    f(&conn).map_err(|e| e.to_string())
}

/// Fetches the turn's current status and only writes `to` if
/// `core::turn_status::is_valid_transition` allows it — see that module for
/// why the DB layer itself deliberately does not enforce this. Shared by
/// every adapter, since the transition rules don't depend on which
/// provider is running.
fn transition_turn(db: &Arc<Mutex<Connection>>, turn_id: &str, to: &str) -> Result<(), String> {
    let current = with_db(db, |conn| crate::db::repository::get_turn(conn, turn_id))?
        .ok_or_else(|| format!("turn {turn_id} not found"))?
        .status;
    if !crate::core::turn_status::is_valid_transition(&current, to) {
        return Err(format!("illegal turn status transition: {current} -> {to}"));
    }
    with_db(db, |conn| {
        crate::db::repository::update_turn_status(conn, turn_id, to)
    })
}

/// Phase 5 — tracks in-flight adapter subprocesses so `cancel_run` can kill
/// one by PID, and records which run_ids were *deliberately* cancelled so
/// the reader thread (see claude_code::run) can tell "the user cancelled
/// this" apart from "the process died on its own" when the stdout stream
/// ends without ever seeing a `result` line — those two cases must not both
/// collapse into a generic "error".
#[derive(Clone, Default)]
pub struct RunControl {
    pids: Arc<Mutex<HashMap<String, u32>>>,
    cancelled: Arc<Mutex<HashSet<String>>>,
}

impl RunControl {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn active_runs(&self) -> Vec<(String,u32)> {
        self.pids.lock().unwrap().iter().map(|(id,pid)|(id.clone(),*pid)).collect()
    }

    pub fn register(&self, run_id: &str, pid: u32) {
        self.pids.lock().unwrap().insert(run_id.to_string(), pid);
    }

    pub fn unregister(&self, run_id: &str) {
        self.pids.lock().unwrap().remove(run_id);
    }

    pub fn pid_of(&self, run_id: &str) -> Option<u32> {
        self.pids.lock().unwrap().get(run_id).copied()
    }

    pub fn mark_cancelled(&self, run_id: &str) {
        self.cancelled.lock().unwrap().insert(run_id.to_string());
    }

    pub fn was_cancelled(&self, run_id: &str) -> bool {
        self.cancelled.lock().unwrap().contains(run_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_control_tracks_and_forgets_pids() {
        let rc = RunControl::new();
        assert_eq!(rc.pid_of("r1"), None);
        rc.register("r1", 1234);
        assert_eq!(rc.pid_of("r1"), Some(1234));
        assert_eq!(rc.active_runs(), vec![("r1".to_string(),1234)]);
        rc.unregister("r1");
        assert_eq!(rc.pid_of("r1"), None);
        assert!(rc.active_runs().is_empty());
    }

    #[test]
    fn run_control_marks_and_reports_cancellation() {
        let rc = RunControl::new();
        assert!(!rc.was_cancelled("r1"));
        rc.mark_cancelled("r1");
        assert!(rc.was_cancelled("r1"));
        assert!(!rc.was_cancelled("r2"));
    }
}

/// One `Log` variant beyond the six in doc 06's proposal: it exists purely
/// to populate the `logs` table (thinking blocks, permission-denial notices)
/// without overloading `TextChunk`, which is reserved for text actually
/// meant to be shown to the user as the assistant's reply.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProviderEvent {
    TextChunk { text: String },
    ActionRequested { action: String, requires_approval: bool },
    FileChanged { path: String, change_type: String },
    SessionId { provider_session_id: String },
    Log { level: String, message: String },
    Error { message: String, fatal: bool },
    Done { exit_status: String },
}
