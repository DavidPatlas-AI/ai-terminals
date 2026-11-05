// The Box — Phase 3 database layer.
//
// One SQLite connection per app, held behind a Mutex in Tauri's managed
// state (see lib.rs). Foreign keys are enforced explicitly — SQLite has
// them off by default per-connection.

pub mod models;
pub mod repository;

use rusqlite::Connection;
use std::sync::{Arc, Mutex};

// Arc so long-running adapter calls (Phase 4) can hold a clone across a
// spawned thread and lock only for the brief duration of each write,
// instead of holding Tauri's managed-state borrow for an entire subprocess
// run.
pub struct DbState(pub Arc<Mutex<Connection>>);

const SCHEMA: &str = include_str!("schema.sql");
const INDEXES: &str = include_str!("indexes.sql");

/// Opens (creating if needed) the SQLite file at `path` and applies the
/// schema. Safe to call on an existing database — every statement is
/// `CREATE TABLE IF NOT EXISTS`.
pub fn init(path: &std::path::Path) -> rusqlite::Result<Connection> {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "foreign_keys", true)?;
    conn.execute_batch(SCHEMA)?;
    migrate_providers(&conn)?;
    conn.execute_batch(INDEXES)?;
    Ok(conn)
}

/// In-memory database for tests — same schema, isolated per call.
#[cfg(test)]
pub fn init_in_memory() -> rusqlite::Result<Connection> {
    let conn = Connection::open_in_memory()?;
    conn.pragma_update(None, "foreign_keys", true)?;
    conn.execute_batch(SCHEMA)?;
    conn.execute_batch(INDEXES)?;
    Ok(conn)
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

// Expand the provider CHECK for existing installations, preserving all references.
fn migrate_providers(conn: &Connection) -> rusqlite::Result<()> {
    let sql: String = conn.query_row("SELECT sql FROM sqlite_master WHERE name='provider_runs'",[],|r|r.get(0))?;
    if sql.contains("'grok'") { return Ok(()); }
    conn.pragma_update(None,"foreign_keys",false)?;
    let result = conn.execute_batch("BEGIN IMMEDIATE;
        CREATE TABLE provider_runs_expanded (
          id TEXT PRIMARY KEY, turn_id TEXT NOT NULL REFERENCES turns(id),
          provider TEXT NOT NULL CHECK(provider IN ('claude_code','codex','gemini','ollama','grok','deepseek')),
          provider_session_id TEXT, started_at TEXT NOT NULL, ended_at TEXT,
          exit_status TEXT CHECK(exit_status IN ('success','error','cancelled',NULL)), raw_log_ref TEXT);
        INSERT INTO provider_runs_expanded SELECT * FROM provider_runs;
        DROP TABLE provider_runs;
        ALTER TABLE provider_runs_expanded RENAME TO provider_runs;
        COMMIT;");
    if result.is_err() { let _=conn.execute_batch("ROLLBACK;"); }
    conn.pragma_update(None,"foreign_keys",true)?;
    result
}

#[cfg(test)]
mod migration_tests {
    use super::*;
    #[test]
    fn expands_old_provider_constraint_without_losing_log_references() {
        let conn=Connection::open_in_memory().unwrap();
        let old=SCHEMA.replace("'ollama','grok','deepseek'", "'ollama'");
        conn.execute_batch(&old).unwrap();
        conn.pragma_update(None,"foreign_keys",true).unwrap();
        let p=repository::create_project(&conn,"Test","C:/test",None).unwrap();
        let c=repository::create_conversation(&conn,&p.id,None).unwrap();
        let t=repository::create_turn(&conn,&c.id,1,"Hi").unwrap();
        let r=repository::create_provider_run(&conn,&t.id,"claude_code",None).unwrap();
        repository::create_log(&conn,&t.id,Some(&r.id),"info","Saved reply").unwrap();
        migrate_providers(&conn).unwrap();
        migrate_providers(&conn).unwrap();
        repository::create_provider_run(&conn,&t.id,"grok",None).unwrap();
        assert_eq!(repository::list_logs_for_turn(&conn,&t.id).unwrap()[0].message,"Saved reply");
        let violations:i64=conn.query_row("SELECT count(*) FROM pragma_foreign_key_check",[],|r|r.get(0)).unwrap();
        assert_eq!(violations,0);
    }
}
