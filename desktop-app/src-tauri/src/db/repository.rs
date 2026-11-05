// The Box — Phase 3 repository layer: one create + one get (+ list where the
// debug screen needs it) per table. No business logic here — that belongs to
// later phases (approval engine, adapters, ...).

use super::models::*;
use super::{new_id, now};
use rusqlite::{params, Connection, OptionalExtension, Result};

// ---------------------------------------------------------------- projects

pub fn create_project(
    conn: &Connection,
    name: &str,
    path: &str,
    icon: Option<&str>,
) -> Result<Project> {
    let id = new_id();
    let created_at = now();
    conn.execute(
        "INSERT INTO projects (id, name, path, icon, created_at, last_opened_at)
         VALUES (?1, ?2, ?3, ?4, ?5, NULL)",
        params![id, name, path, icon, created_at],
    )?;
    Ok(Project {
        id,
        name: name.to_string(),
        path: path.to_string(),
        icon: icon.map(str::to_string),
        created_at,
        last_opened_at: None,
    })
}

pub fn get_project(conn: &Connection, id: &str) -> Result<Option<Project>> {
    conn.query_row(
        "SELECT id, name, path, icon, created_at, last_opened_at FROM projects WHERE id = ?1",
        params![id],
        |r| {
            Ok(Project {
                id: r.get(0)?,
                name: r.get(1)?,
                path: r.get(2)?,
                icon: r.get(3)?,
                created_at: r.get(4)?,
                last_opened_at: r.get(5)?,
            })
        },
    )
    .optional()
}

pub fn list_projects(conn: &Connection) -> Result<Vec<Project>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, path, icon, created_at, last_opened_at FROM projects ORDER BY created_at",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(Project {
            id: r.get(0)?,
            name: r.get(1)?,
            path: r.get(2)?,
            icon: r.get(3)?,
            created_at: r.get(4)?,
            last_opened_at: r.get(5)?,
        })
    })?;
    rows.collect()
}

// ----------------------------------------------------------- conversations

pub fn create_conversation(
    conn: &Connection,
    project_id: &str,
    title: Option<&str>,
) -> Result<Conversation> {
    let id = new_id();
    let ts = now();
    conn.execute(
        "INSERT INTO conversations (id, project_id, title, created_at, updated_at, status)
         VALUES (?1, ?2, ?3, ?4, ?4, 'active')",
        params![id, project_id, title, ts],
    )?;
    Ok(Conversation {
        id,
        project_id: project_id.to_string(),
        title: title.map(str::to_string),
        created_at: ts.clone(),
        updated_at: ts,
        status: "active".to_string(),
    })
}

pub fn get_conversation(conn: &Connection, id: &str) -> Result<Option<Conversation>> {
    conn.query_row(
        "SELECT id, project_id, title, created_at, updated_at, status FROM conversations WHERE id = ?1",
        params![id],
        map_conversation,
    )
    .optional()
}

pub fn list_conversations_for_project(
    conn: &Connection,
    project_id: &str,
) -> Result<Vec<Conversation>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, title, created_at, updated_at, status
         FROM conversations WHERE project_id = ?1 ORDER BY created_at",
    )?;
    let rows = stmt.query_map(params![project_id], map_conversation)?;
    rows.collect()
}

fn map_conversation(r: &rusqlite::Row) -> Result<Conversation> {
    Ok(Conversation {
        id: r.get(0)?,
        project_id: r.get(1)?,
        title: r.get(2)?,
        created_at: r.get(3)?,
        updated_at: r.get(4)?,
        status: r.get(5)?,
    })
}

// ------------------------------------------------------------------ turns

pub fn create_turn(
    conn: &Connection,
    conversation_id: &str,
    seq: i64,
    user_intent: &str,
) -> Result<Turn> {
    let id = new_id();
    let ts = now();
    conn.execute(
        "INSERT INTO turns (id, conversation_id, seq, user_intent, task_plan_id, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, NULL, 'planning', ?5, ?5)",
        params![id, conversation_id, seq, user_intent, ts],
    )?;
    Ok(Turn {
        id,
        conversation_id: conversation_id.to_string(),
        seq,
        user_intent: user_intent.to_string(),
        task_plan_id: None,
        status: "planning".to_string(),
        created_at: ts.clone(),
        updated_at: ts,
    })
}

pub fn get_turn(conn: &Connection, id: &str) -> Result<Option<Turn>> {
    conn.query_row(
        "SELECT id, conversation_id, seq, user_intent, task_plan_id, status, created_at, updated_at
         FROM turns WHERE id = ?1",
        params![id],
        map_turn,
    )
    .optional()
}

pub fn list_turns_for_conversation(conn: &Connection, conversation_id: &str) -> Result<Vec<Turn>> {
    let mut stmt = conn.prepare(
        "SELECT id, conversation_id, seq, user_intent, task_plan_id, status, created_at, updated_at
         FROM turns WHERE conversation_id = ?1 ORDER BY seq",
    )?;
    let rows = stmt.query_map(params![conversation_id], map_turn)?;
    rows.collect()
}

pub fn set_turn_task_plan(conn: &Connection, turn_id: &str, task_plan_id: &str) -> Result<()> {
    conn.execute(
        "UPDATE turns SET task_plan_id = ?2, updated_at = ?3 WHERE id = ?1",
        params![turn_id, task_plan_id, now()],
    )?;
    Ok(())
}

pub fn update_turn_status(conn: &Connection, turn_id: &str, status: &str) -> Result<()> {
    conn.execute(
        "UPDATE turns SET status = ?2, updated_at = ?3 WHERE id = ?1",
        params![turn_id, status, now()],
    )?;
    Ok(())
}

fn map_turn(r: &rusqlite::Row) -> Result<Turn> {
    Ok(Turn {
        id: r.get(0)?,
        conversation_id: r.get(1)?,
        seq: r.get(2)?,
        user_intent: r.get(3)?,
        task_plan_id: r.get(4)?,
        status: r.get(5)?,
        created_at: r.get(6)?,
        updated_at: r.get(7)?,
    })
}

// ------------------------------------------------------------ provider_runs

pub fn create_provider_run(
    conn: &Connection,
    turn_id: &str,
    provider: &str,
    provider_session_id: Option<&str>,
) -> Result<ProviderRun> {
    let id = new_id();
    let started_at = now();
    conn.execute(
        "INSERT INTO provider_runs (id, turn_id, provider, provider_session_id, started_at, ended_at, exit_status, raw_log_ref)
         VALUES (?1, ?2, ?3, ?4, ?5, NULL, NULL, NULL)",
        params![id, turn_id, provider, provider_session_id, started_at],
    )?;
    Ok(ProviderRun {
        id,
        turn_id: turn_id.to_string(),
        provider: provider.to_string(),
        provider_session_id: provider_session_id.map(str::to_string),
        started_at,
        ended_at: None,
        exit_status: None,
        raw_log_ref: None,
    })
}

pub fn get_provider_run(conn: &Connection, id: &str) -> Result<Option<ProviderRun>> {
    conn.query_row(
        "SELECT id, turn_id, provider, provider_session_id, started_at, ended_at, exit_status, raw_log_ref
         FROM provider_runs WHERE id = ?1",
        params![id],
        |r| {
            Ok(ProviderRun {
                id: r.get(0)?,
                turn_id: r.get(1)?,
                provider: r.get(2)?,
                provider_session_id: r.get(3)?,
                started_at: r.get(4)?,
                ended_at: r.get(5)?,
                exit_status: r.get(6)?,
                raw_log_ref: r.get(7)?,
            })
        },
    )
    .optional()
}

pub fn set_provider_run_session(conn: &Connection, id: &str, provider_session_id: &str) -> Result<()> {
    conn.execute(
        "UPDATE provider_runs SET provider_session_id = ?2 WHERE id = ?1",
        params![id, provider_session_id],
    )?;
    Ok(())
}

pub fn finish_provider_run(conn: &Connection, id: &str, exit_status: &str) -> Result<()> {
    conn.execute(
        "UPDATE provider_runs SET ended_at = ?2, exit_status = ?3 WHERE id = ?1",
        params![id, now(), exit_status],
    )?;
    Ok(())
}

// ---------------------------------------------------------------- task_plans

pub fn create_task_plan(
    conn: &Connection,
    turn_id: &str,
    steps_json: &str,
    risk_level: Option<&str>,
) -> Result<TaskPlan> {
    let id = new_id();
    let created_at = now();
    conn.execute(
        "INSERT INTO task_plans (id, turn_id, steps_json, risk_level, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, turn_id, steps_json, risk_level, created_at],
    )?;
    Ok(TaskPlan {
        id,
        turn_id: turn_id.to_string(),
        steps_json: steps_json.to_string(),
        risk_level: risk_level.map(str::to_string),
        created_at,
    })
}

pub fn get_task_plan(conn: &Connection, id: &str) -> Result<Option<TaskPlan>> {
    conn.query_row(
        "SELECT id, turn_id, steps_json, risk_level, created_at FROM task_plans WHERE id = ?1",
        params![id],
        |r| {
            Ok(TaskPlan {
                id: r.get(0)?,
                turn_id: r.get(1)?,
                steps_json: r.get(2)?,
                risk_level: r.get(3)?,
                created_at: r.get(4)?,
            })
        },
    )
    .optional()
}

// ----------------------------------------------------------------- approvals

pub fn create_approval(
    conn: &Connection,
    turn_id: &str,
    scope: &str,
    action_ref: Option<&str>,
    decision: &str,
) -> Result<Approval> {
    let id = new_id();
    let decided_at = now();
    conn.execute(
        "INSERT INTO approvals (id, turn_id, scope, action_ref, decision, decided_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, turn_id, scope, action_ref, decision, decided_at],
    )?;
    Ok(Approval {
        id,
        turn_id: turn_id.to_string(),
        scope: scope.to_string(),
        action_ref: action_ref.map(str::to_string),
        decision: decision.to_string(),
        decided_at,
    })
}

pub fn get_approval(conn: &Connection, id: &str) -> Result<Option<Approval>> {
    conn.query_row(
        "SELECT id, turn_id, scope, action_ref, decision, decided_at FROM approvals WHERE id = ?1",
        params![id],
        |r| {
            Ok(Approval {
                id: r.get(0)?,
                turn_id: r.get(1)?,
                scope: r.get(2)?,
                action_ref: r.get(3)?,
                decision: r.get(4)?,
                decided_at: r.get(5)?,
            })
        },
    )
    .optional()
}

pub fn list_approvals_for_turn(conn: &Connection, turn_id: &str) -> Result<Vec<Approval>> {
    let mut stmt = conn.prepare(
        "SELECT id, turn_id, scope, action_ref, decision, decided_at
         FROM approvals WHERE turn_id = ?1 ORDER BY decided_at",
    )?;
    let rows = stmt.query_map(params![turn_id], |r| {
        Ok(Approval {
            id: r.get(0)?,
            turn_id: r.get(1)?,
            scope: r.get(2)?,
            action_ref: r.get(3)?,
            decision: r.get(4)?,
            decided_at: r.get(5)?,
        })
    })?;
    rows.collect()
}

// -------------------------------------------------------------- file_changes

#[allow(clippy::too_many_arguments)]
pub fn create_file_change(
    conn: &Connection,
    turn_id: &str,
    provider_run_id: Option<&str>,
    file_path: &str,
    change_type: &str,
    diff_ref: Option<&str>,
    git_checkpoint_id: Option<&str>,
) -> Result<FileChange> {
    let id = new_id();
    conn.execute(
        "INSERT INTO file_changes (id, turn_id, provider_run_id, file_path, change_type, diff_ref, git_checkpoint_id, reviewed_at, review_decision)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, NULL)",
        params![id, turn_id, provider_run_id, file_path, change_type, diff_ref, git_checkpoint_id],
    )?;
    Ok(FileChange {
        id,
        turn_id: turn_id.to_string(),
        provider_run_id: provider_run_id.map(str::to_string),
        file_path: file_path.to_string(),
        change_type: change_type.to_string(),
        diff_ref: diff_ref.map(str::to_string),
        git_checkpoint_id: git_checkpoint_id.map(str::to_string),
        reviewed_at: None,
        review_decision: None,
    })
}

pub fn get_file_change(conn: &Connection, id: &str) -> Result<Option<FileChange>> {
    conn.query_row(
        "SELECT id, turn_id, provider_run_id, file_path, change_type, diff_ref, git_checkpoint_id, reviewed_at, review_decision
         FROM file_changes WHERE id = ?1",
        params![id],
        |r| {
            Ok(FileChange {
                id: r.get(0)?,
                turn_id: r.get(1)?,
                provider_run_id: r.get(2)?,
                file_path: r.get(3)?,
                change_type: r.get(4)?,
                diff_ref: r.get(5)?,
                git_checkpoint_id: r.get(6)?,
                reviewed_at: r.get(7)?,
                review_decision: r.get(8)?,
            })
        },
    )
    .optional()
}

pub fn list_file_changes_for_turn(conn: &Connection, turn_id: &str) -> Result<Vec<FileChange>> {
    let mut stmt = conn.prepare(
        "SELECT id, turn_id, provider_run_id, file_path, change_type, diff_ref, git_checkpoint_id, reviewed_at, review_decision
         FROM file_changes WHERE turn_id = ?1 ORDER BY rowid",
    )?;
    let rows = stmt.query_map(params![turn_id], |r| {
        Ok(FileChange {
            id: r.get(0)?,
            turn_id: r.get(1)?,
            provider_run_id: r.get(2)?,
            file_path: r.get(3)?,
            change_type: r.get(4)?,
            diff_ref: r.get(5)?,
            git_checkpoint_id: r.get(6)?,
            reviewed_at: r.get(7)?,
            review_decision: r.get(8)?,
        })
    })?;
    rows.collect()
}

// ---------------------------------------------------------------- artifacts

pub fn create_artifact(
    conn: &Connection,
    turn_id: &str,
    kind: &str,
    content_ref: &str,
) -> Result<Artifact> {
    let id = new_id();
    let created_at = now();
    conn.execute(
        "INSERT INTO artifacts (id, turn_id, kind, content_ref, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, turn_id, kind, content_ref, created_at],
    )?;
    Ok(Artifact {
        id,
        turn_id: turn_id.to_string(),
        kind: kind.to_string(),
        content_ref: content_ref.to_string(),
        created_at,
    })
}

pub fn get_artifact(conn: &Connection, id: &str) -> Result<Option<Artifact>> {
    conn.query_row(
        "SELECT id, turn_id, kind, content_ref, created_at FROM artifacts WHERE id = ?1",
        params![id],
        |r| {
            Ok(Artifact {
                id: r.get(0)?,
                turn_id: r.get(1)?,
                kind: r.get(2)?,
                content_ref: r.get(3)?,
                created_at: r.get(4)?,
            })
        },
    )
    .optional()
}

// ----------------------------------------------------------- git_checkpoints

pub fn create_git_checkpoint(
    conn: &Connection,
    turn_id: &str,
    project_id: &str,
    commit_or_stash_ref: &str,
) -> Result<GitCheckpoint> {
    let id = new_id();
    let created_at = now();
    conn.execute(
        "INSERT INTO git_checkpoints (id, turn_id, project_id, commit_or_stash_ref, created_at, restored_at)
         VALUES (?1, ?2, ?3, ?4, ?5, NULL)",
        params![id, turn_id, project_id, commit_or_stash_ref, created_at],
    )?;
    Ok(GitCheckpoint {
        id,
        turn_id: turn_id.to_string(),
        project_id: project_id.to_string(),
        commit_or_stash_ref: commit_or_stash_ref.to_string(),
        created_at,
        restored_at: None,
    })
}

pub fn get_git_checkpoint(conn: &Connection, id: &str) -> Result<Option<GitCheckpoint>> {
    conn.query_row(
        "SELECT id, turn_id, project_id, commit_or_stash_ref, created_at, restored_at
         FROM git_checkpoints WHERE id = ?1",
        params![id],
        |r| {
            Ok(GitCheckpoint {
                id: r.get(0)?,
                turn_id: r.get(1)?,
                project_id: r.get(2)?,
                commit_or_stash_ref: r.get(3)?,
                created_at: r.get(4)?,
                restored_at: r.get(5)?,
            })
        },
    )
    .optional()
}

pub fn get_latest_git_checkpoint_for_turn(conn: &Connection, turn_id: &str) -> Result<Option<GitCheckpoint>> {
    conn.query_row(
        "SELECT id, turn_id, project_id, commit_or_stash_ref, created_at, restored_at
         FROM git_checkpoints WHERE turn_id = ?1 ORDER BY created_at DESC LIMIT 1",
        params![turn_id],
        |r| {
            Ok(GitCheckpoint {
                id: r.get(0)?,
                turn_id: r.get(1)?,
                project_id: r.get(2)?,
                commit_or_stash_ref: r.get(3)?,
                created_at: r.get(4)?,
                restored_at: r.get(5)?,
            })
        },
    )
    .optional()
}

pub fn mark_git_checkpoint_restored(conn: &Connection, id: &str) -> Result<()> {
    conn.execute(
        "UPDATE git_checkpoints SET restored_at = ?2 WHERE id = ?1",
        params![id, now()],
    )?;
    Ok(())
}

// ----------------------------------------------------------- open_questions

pub fn create_open_question(
    conn: &Connection,
    turn_id: &str,
    question: &str,
    raised_by_provider_run_id: Option<&str>,
) -> Result<OpenQuestion> {
    let id = new_id();
    conn.execute(
        "INSERT INTO open_questions (id, turn_id, question, raised_by_provider_run_id, resolved_at, resolution)
         VALUES (?1, ?2, ?3, ?4, NULL, NULL)",
        params![id, turn_id, question, raised_by_provider_run_id],
    )?;
    Ok(OpenQuestion {
        id,
        turn_id: turn_id.to_string(),
        question: question.to_string(),
        raised_by_provider_run_id: raised_by_provider_run_id.map(str::to_string),
        resolved_at: None,
        resolution: None,
    })
}

pub fn get_open_question(conn: &Connection, id: &str) -> Result<Option<OpenQuestion>> {
    conn.query_row(
        "SELECT id, turn_id, question, raised_by_provider_run_id, resolved_at, resolution
         FROM open_questions WHERE id = ?1",
        params![id],
        |r| {
            Ok(OpenQuestion {
                id: r.get(0)?,
                turn_id: r.get(1)?,
                question: r.get(2)?,
                raised_by_provider_run_id: r.get(3)?,
                resolved_at: r.get(4)?,
                resolution: r.get(5)?,
            })
        },
    )
    .optional()
}

pub fn list_open_questions_for_turn(conn: &Connection, turn_id: &str) -> Result<Vec<OpenQuestion>> {
    let mut stmt = conn.prepare(
        "SELECT id, turn_id, question, raised_by_provider_run_id, resolved_at, resolution
         FROM open_questions WHERE turn_id = ?1 ORDER BY rowid",
    )?;
    let rows = stmt.query_map(params![turn_id], |r| {
        Ok(OpenQuestion {
            id: r.get(0)?,
            turn_id: r.get(1)?,
            question: r.get(2)?,
            raised_by_provider_run_id: r.get(3)?,
            resolved_at: r.get(4)?,
            resolution: r.get(5)?,
        })
    })?;
    rows.collect()
}

// -------------------------------------------------------------------- logs

pub fn create_log(
    conn: &Connection,
    turn_id: &str,
    provider_run_id: Option<&str>,
    level: &str,
    message: &str,
) -> Result<LogEntry> {
    let id = new_id();
    let created_at = now();
    conn.execute(
        "INSERT INTO logs (id, turn_id, provider_run_id, level, message, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, turn_id, provider_run_id, level, message, created_at],
    )?;
    Ok(LogEntry {
        id,
        turn_id: turn_id.to_string(),
        provider_run_id: provider_run_id.map(str::to_string),
        level: level.to_string(),
        message: message.to_string(),
        created_at,
    })
}

pub fn list_logs_for_turn(conn: &Connection, turn_id: &str) -> Result<Vec<LogEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, turn_id, provider_run_id, level, message, created_at
         FROM logs WHERE turn_id = ?1 ORDER BY created_at",
    )?;
    let rows = stmt.query_map(params![turn_id], |r| {
        Ok(LogEntry {
            id: r.get(0)?,
            turn_id: r.get(1)?,
            provider_run_id: r.get(2)?,
            level: r.get(3)?,
            message: r.get(4)?,
            created_at: r.get(5)?,
        })
    })?;
    rows.collect()
}

// ----------------------------------------------------------------- settings

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<Setting> {
    let updated_at = now();
    conn.execute(
        "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        params![key, value, updated_at],
    )?;
    Ok(Setting {
        key: key.to_string(),
        value: value.to_string(),
        updated_at,
    })
}

pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<Setting>> {
    conn.query_row(
        "SELECT key, value, updated_at FROM settings WHERE key = ?1",
        params![key],
        |r| {
            Ok(Setting {
                key: r.get(0)?,
                value: r.get(1)?,
                updated_at: r.get(2)?,
            })
        },
    )
    .optional()
}

// ---------------------------------------------------------------------- tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_in_memory;

    fn setup() -> Connection {
        init_in_memory().expect("schema should apply cleanly")
    }

    #[test]
    fn project_create_and_get_roundtrip() {
        let conn = setup();
        let p = create_project(&conn, "Demo", "C:\\demo", Some("📁")).unwrap();
        let fetched = get_project(&conn, &p.id).unwrap().unwrap();
        assert_eq!(fetched.name, "Demo");
        assert_eq!(fetched.path, "C:\\demo");
        assert_eq!(fetched.icon.as_deref(), Some("📁"));
    }

    #[test]
    fn get_project_missing_returns_none() {
        let conn = setup();
        assert!(get_project(&conn, "does-not-exist").unwrap().is_none());
    }

    #[test]
    fn conversation_requires_existing_project() {
        let conn = setup();
        let err = create_conversation(&conn, "missing-project", Some("t")).unwrap_err();
        assert!(matches!(err, rusqlite::Error::SqliteFailure(_, _)));
    }

    #[test]
    fn conversation_create_and_list_for_project() {
        let conn = setup();
        let p = create_project(&conn, "Demo", "C:\\demo", None).unwrap();
        let c1 = create_conversation(&conn, &p.id, Some("first")).unwrap();
        let c2 = create_conversation(&conn, &p.id, Some("second")).unwrap();
        let list = list_conversations_for_project(&conn, &p.id).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, c1.id);
        assert_eq!(list[1].id, c2.id);
        assert_eq!(get_conversation(&conn, &c1.id).unwrap().unwrap().status, "active");
    }

    #[test]
    fn turn_requires_existing_conversation() {
        let conn = setup();
        let err = create_turn(&conn, "missing-conversation", 1, "hello").unwrap_err();
        assert!(matches!(err, rusqlite::Error::SqliteFailure(_, _)));
    }

    #[test]
    fn turn_lifecycle_with_task_plan() {
        let conn = setup();
        let p = create_project(&conn, "Demo", "C:\\demo", None).unwrap();
        let c = create_conversation(&conn, &p.id, None).unwrap();
        let t = create_turn(&conn, &c.id, 1, "תעשה לי X").unwrap();
        assert_eq!(t.status, "planning");
        assert!(t.task_plan_id.is_none());

        let plan = create_task_plan(&conn, &t.id, "[\"step1\"]", Some("low")).unwrap();
        set_turn_task_plan(&conn, &t.id, &plan.id).unwrap();
        update_turn_status(&conn, &t.id, "awaiting_approval").unwrap();

        let refreshed = get_turn(&conn, &t.id).unwrap().unwrap();
        assert_eq!(refreshed.task_plan_id.as_deref(), Some(plan.id.as_str()));
        assert_eq!(refreshed.status, "awaiting_approval");

        let fetched_plan = get_task_plan(&conn, &plan.id).unwrap().unwrap();
        assert_eq!(fetched_plan.steps_json, "[\"step1\"]");
    }

    #[test]
    fn turn_status_rejects_invalid_value() {
        let conn = setup();
        let p = create_project(&conn, "Demo", "C:\\demo", None).unwrap();
        let c = create_conversation(&conn, &p.id, None).unwrap();
        let t = create_turn(&conn, &c.id, 1, "x").unwrap();
        let err = update_turn_status(&conn, &t.id, "not-a-real-status").unwrap_err();
        assert!(matches!(err, rusqlite::Error::SqliteFailure(_, _)));
    }

    #[test]
    fn provider_run_create_finish_and_get() {
        let conn = setup();
        let p = create_project(&conn, "Demo", "C:\\demo", None).unwrap();
        let c = create_conversation(&conn, &p.id, None).unwrap();
        let t = create_turn(&conn, &c.id, 1, "x").unwrap();
        let run = create_provider_run(&conn, &t.id, "claude_code", Some("sess-1")).unwrap();
        assert!(run.ended_at.is_none());

        finish_provider_run(&conn, &run.id, "success").unwrap();
        let fetched = get_provider_run(&conn, &run.id).unwrap().unwrap();
        assert_eq!(fetched.exit_status.as_deref(), Some("success"));
        assert!(fetched.ended_at.is_some());
    }

    #[test]
    fn provider_run_session_id_can_be_set_after_creation() {
        let conn = setup();
        let p = create_project(&conn, "Demo", "C:\\demo", None).unwrap();
        let c = create_conversation(&conn, &p.id, None).unwrap();
        let t = create_turn(&conn, &c.id, 1, "x").unwrap();
        let run = create_provider_run(&conn, &t.id, "claude_code", None).unwrap();
        assert!(run.provider_session_id.is_none());
        set_provider_run_session(&conn, &run.id, "sess-abc").unwrap();
        let fetched = get_provider_run(&conn, &run.id).unwrap().unwrap();
        assert_eq!(fetched.provider_session_id.as_deref(), Some("sess-abc"));
    }

    #[test]
    fn provider_run_rejects_unknown_provider() {
        let conn = setup();
        let p = create_project(&conn, "Demo", "C:\\demo", None).unwrap();
        let c = create_conversation(&conn, &p.id, None).unwrap();
        let t = create_turn(&conn, &c.id, 1, "x").unwrap();
        let err = create_provider_run(&conn, &t.id, "not_a_provider", None).unwrap_err();
        assert!(matches!(err, rusqlite::Error::SqliteFailure(_, _)));
    }

    #[test]
    fn approval_create_and_get() {
        let conn = setup();
        let p = create_project(&conn, "Demo", "C:\\demo", None).unwrap();
        let c = create_conversation(&conn, &p.id, None).unwrap();
        let t = create_turn(&conn, &c.id, 1, "x").unwrap();
        let a = create_approval(&conn, &t.id, "full_plan", None, "approved").unwrap();
        assert_eq!(get_approval(&conn, &a.id).unwrap().unwrap().decision, "approved");
    }

    #[test]
    fn file_change_create_and_get() {
        let conn = setup();
        let p = create_project(&conn, "Demo", "C:\\demo", None).unwrap();
        let c = create_conversation(&conn, &p.id, None).unwrap();
        let t = create_turn(&conn, &c.id, 1, "x").unwrap();
        let fc = create_file_change(&conn, &t.id, None, "src/main.rs", "modified", None, None).unwrap();
        let fetched = get_file_change(&conn, &fc.id).unwrap().unwrap();
        assert_eq!(fetched.change_type, "modified");
        assert!(fetched.review_decision.is_none());
    }

    #[test]
    fn artifact_create_and_get() {
        let conn = setup();
        let p = create_project(&conn, "Demo", "C:\\demo", None).unwrap();
        let c = create_conversation(&conn, &p.id, None).unwrap();
        let t = create_turn(&conn, &c.id, 1, "x").unwrap();
        let a = create_artifact(&conn, &t.id, "test_result", "ref://1").unwrap();
        assert_eq!(get_artifact(&conn, &a.id).unwrap().unwrap().kind, "test_result");
    }

    #[test]
    fn git_checkpoint_create_and_get() {
        let conn = setup();
        let p = create_project(&conn, "Demo", "C:\\demo", None).unwrap();
        let c = create_conversation(&conn, &p.id, None).unwrap();
        let t = create_turn(&conn, &c.id, 1, "x").unwrap();
        let gc = create_git_checkpoint(&conn, &t.id, &p.id, "abc123").unwrap();
        assert_eq!(
            get_git_checkpoint(&conn, &gc.id).unwrap().unwrap().commit_or_stash_ref,
            "abc123"
        );
    }

    #[test]
    fn open_question_create_and_get() {
        let conn = setup();
        let p = create_project(&conn, "Demo", "C:\\demo", None).unwrap();
        let c = create_conversation(&conn, &p.id, None).unwrap();
        let t = create_turn(&conn, &c.id, 1, "x").unwrap();
        let q = create_open_question(&conn, &t.id, "מה גודל ה-DB?", None).unwrap();
        assert!(get_open_question(&conn, &q.id).unwrap().unwrap().resolved_at.is_none());
    }

    #[test]
    fn log_create_and_list_for_turn() {
        let conn = setup();
        let p = create_project(&conn, "Demo", "C:\\demo", None).unwrap();
        let c = create_conversation(&conn, &p.id, None).unwrap();
        let t = create_turn(&conn, &c.id, 1, "x").unwrap();
        create_log(&conn, &t.id, None, "info", "started").unwrap();
        create_log(&conn, &t.id, None, "error", "boom").unwrap();
        let logs = list_logs_for_turn(&conn, &t.id).unwrap();
        assert_eq!(logs.len(), 2);
    }

    #[test]
    fn log_rejects_invalid_level() {
        let conn = setup();
        let p = create_project(&conn, "Demo", "C:\\demo", None).unwrap();
        let c = create_conversation(&conn, &p.id, None).unwrap();
        let t = create_turn(&conn, &c.id, 1, "x").unwrap();
        let err = create_log(&conn, &t.id, None, "critical", "boom").unwrap_err();
        assert!(matches!(err, rusqlite::Error::SqliteFailure(_, _)));
    }

    #[test]
    fn setting_set_is_upsert() {
        let conn = setup();
        set_setting(&conn, "theme", "dark").unwrap();
        set_setting(&conn, "theme", "light").unwrap();
        assert_eq!(get_setting(&conn, "theme").unwrap().unwrap().value, "light");
    }

    #[test]
    fn project_path_must_be_unique() {
        let conn = setup();
        create_project(&conn, "A", "C:\\same", None).unwrap();
        let err = create_project(&conn, "B", "C:\\same", None).unwrap_err();
        assert!(matches!(err, rusqlite::Error::SqliteFailure(_, _)));
    }
}
