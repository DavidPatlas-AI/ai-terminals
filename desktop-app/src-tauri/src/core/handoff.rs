// The Box — Phase 6: handoff summary builder.
//
// Per THE_BOX_AUDIT/05_DATABASE_AND_CONVERSATION_MODEL_HE.md "תמצית
// handoff": when a turn moves from one provider to another, the next
// provider's initial prompt must include what happened so far — not just
// the original request. This assembles that from the same tables Phase 3-5
// already populate (turns, task_plans, approvals, file_changes, logs);
// nothing here is provider-specific.
//
// Scenario 13 (doc 08) requires this land in the *next provider's initial
// prompt*, not a side channel the CLI might ignore — so the caller
// (lib.rs's handoff_to_codex) prepends this text directly to the prompt it
// hands the adapter.

use crate::db::repository;
use rusqlite::Connection;

pub fn build_summary(conn: &Connection, turn_id: &str) -> Result<String, String> {
    let turn = repository::get_turn(conn, turn_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("turn {turn_id} not found"))?;

    let mut out = String::new();
    out.push_str("--- הקשר קודם (הועבר מספק AI אחר, ראו לוגים לפרטים מלאים) ---\n\n");
    out.push_str("בקשת המשתמש המקורית:\n");
    out.push_str(&turn.user_intent);
    out.push_str("\n\n");

    if let Some(plan_id) = &turn.task_plan_id {
        if let Some(plan) = repository::get_task_plan(conn, plan_id).map_err(|e| e.to_string())? {
            out.push_str(&format!(
                "תוכנית עבודה שנוצרה (רמת סיכון: {}):\n",
                plan.risk_level.as_deref().unwrap_or("לא ידוע")
            ));
            let steps: Vec<String> = serde_json::from_str(&plan.steps_json).unwrap_or_default();
            for (i, step) in steps.iter().enumerate() {
                out.push_str(&format!("{}. {}\n", i + 1, step));
            }
            out.push('\n');
        }
    }

    let approvals = repository::list_approvals_for_turn(conn, turn_id).map_err(|e| e.to_string())?;
    if !approvals.is_empty() {
        out.push_str("אישורים שהתקבלו:\n");
        for a in &approvals {
            out.push_str(&format!("- {}: {}\n", a.scope, a.decision));
        }
        out.push('\n');
    }

    let file_changes = repository::list_file_changes_for_turn(conn, turn_id).map_err(|e| e.to_string())?;
    let unreviewed: Vec<_> = file_changes.iter().filter(|f| f.reviewed_at.is_none()).collect();
    if !unreviewed.is_empty() {
        out.push_str("קבצים ששונו עד כה (טרם נסקרו):\n");
        for f in &unreviewed {
            out.push_str(&format!("- {}: {}\n", f.change_type, f.file_path));
        }
        out.push('\n');
    }

    let open_questions = repository::list_open_questions_for_turn(conn, turn_id).map_err(|e| e.to_string())?;
    let unresolved: Vec<_> = open_questions.iter().filter(|q| q.resolved_at.is_none()).collect();
    if !unresolved.is_empty() {
        out.push_str("שאלות פתוחות שטרם נענו:\n");
        for q in &unresolved {
            out.push_str(&format!("- {}\n", q.question));
        }
        out.push('\n');
    }

    let logs = repository::list_logs_for_turn(conn, turn_id).map_err(|e| e.to_string())?;
    if !logs.is_empty() {
        out.push_str("תמצית פעילות/שגיאות אחרונות:\n");
        let tail_start = logs.len().saturating_sub(10);
        for log in &logs[tail_start..] {
            out.push_str(&format!("[{}] {}\n", log.level, truncate(&log.message, 200)));
        }
        out.push('\n');
    }

    out.push_str("--- סוף ההקשר הקודם. אנא המשך את המשימה מכאן. ---\n");
    Ok(out)
}

fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{truncated}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_in_memory;

    #[test]
    fn summary_includes_intent_plan_approvals_files_and_logs() {
        let conn = init_in_memory().unwrap();
        let project = repository::create_project(&conn, "Demo", "C:\\demo", None).unwrap();
        let conversation = repository::create_conversation(&conn, &project.id, None).unwrap();
        let turn = repository::create_turn(&conn, &conversation.id, 1, "תבנה לי מסך התחברות").unwrap();

        let plan = repository::create_task_plan(
            &conn,
            &turn.id,
            r#"["ליצור קובץ login.html","להוסיף בדיקות"]"#,
            Some("medium"),
        )
        .unwrap();
        repository::set_turn_task_plan(&conn, &turn.id, &plan.id).unwrap();
        repository::create_approval(&conn, &turn.id, "full_plan", Some(&plan.id), "approved").unwrap();
        repository::create_file_change(&conn, &turn.id, None, "login.html", "created", None, None).unwrap();
        repository::create_log(&conn, &turn.id, None, "info", "started working").unwrap();
        repository::create_log(&conn, &turn.id, None, "error", "compile error on line 5").unwrap();

        let summary = build_summary(&conn, &turn.id).unwrap();

        assert!(summary.contains("תבנה לי מסך התחברות"), "should include original user_intent");
        assert!(summary.contains("ליצור קובץ login.html"), "should include plan steps");
        assert!(summary.contains("medium"), "should include risk level");
        assert!(summary.contains("approved"), "should include approval decision");
        assert!(summary.contains("login.html"), "should include unreviewed file change");
        assert!(summary.contains("compile error on line 5"), "should include recent error log");
    }

    #[test]
    fn summary_omits_empty_sections_gracefully() {
        let conn = init_in_memory().unwrap();
        let project = repository::create_project(&conn, "Demo", "C:\\demo", None).unwrap();
        let conversation = repository::create_conversation(&conn, &project.id, None).unwrap();
        let turn = repository::create_turn(&conn, &conversation.id, 1, "בקשה פשוטה").unwrap();

        let summary = build_summary(&conn, &turn.id).unwrap();
        assert!(summary.contains("בקשה פשוטה"));
        assert!(!summary.contains("תוכנית עבודה"), "no plan was created, section should be skipped");
    }

    #[test]
    fn reviewed_file_changes_are_excluded() {
        let conn = init_in_memory().unwrap();
        let project = repository::create_project(&conn, "Demo", "C:\\demo", None).unwrap();
        let conversation = repository::create_conversation(&conn, &project.id, None).unwrap();
        let turn = repository::create_turn(&conn, &conversation.id, 1, "x").unwrap();
        let fc = repository::create_file_change(&conn, &turn.id, None, "a.txt", "created", None, None).unwrap();
        conn.execute(
            "UPDATE file_changes SET reviewed_at = ?2, review_decision = 'kept' WHERE id = ?1",
            rusqlite::params![fc.id, "2026-07-31T00:00:00Z"],
        )
        .unwrap();

        let summary = build_summary(&conn, &turn.id).unwrap();
        assert!(!summary.contains("a.txt"), "reviewed file changes should not appear as pending review");
    }

    #[test]
    fn unknown_turn_is_a_clear_error() {
        let conn = init_in_memory().unwrap();
        let err = build_summary(&conn, "does-not-exist").unwrap_err();
        assert!(err.contains("not found"));
    }
}
