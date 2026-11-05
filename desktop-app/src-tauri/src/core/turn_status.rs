// The Box — Phase 5: valid `turns.status` transitions.
//
// SQLite's CHECK constraint on the column only restricts which *values* are
// legal, not which *transitions* are — nothing stops a naive UPDATE from
// jumping straight from 'completed' back to 'running'. This is the single
// place that decides which turn -> turn transitions are legal; every
// command that changes a turn's status calls `is_valid_transition` first
// (see adapters::claude_code::run and the Phase 5 Tauri commands in
// lib.rs) and refuses to write the new status if it returns false.

// Only `cancelled` is truly terminal (the user explicitly gave up).
//
// `completed` and `failed` were originally both terminal too, but Phase 6's
// handoff scenario (doc 08 #12: task starts in Claude Code, moves to
// Codex, mid-conversation) broke that assumption for real — a turn can
// hold several provider_runs (per THE_BOX_AUDIT/05's data model), and
// handing off after a provider finishes (successfully *or* not) means
// starting a fresh provider_run on the *same* turn, which means the turn
// has to be able to go back to `running`. `completed` doesn't mean "this
// turn object is frozen"; it means "the last provider_run that touched it
// finished cleanly" — the conversation can still continue. `failed`
// re-opening the same way covers retrying with a different provider after
// a real failure (scenario 14 is about honest reporting at the time, not
// about failure being unrecoverable).
const TERMINAL: &[&str] = &["cancelled"];

pub fn is_valid_transition(from: &str, to: &str) -> bool {
    if from == to {
        return true; // idempotent no-op writes are fine
    }
    if TERMINAL.contains(&from) {
        return false; // terminal states never leave
    }
    match (from, to) {
        ("planning", "awaiting_approval") => true,
        ("planning", "cancelled") => true,
        // Simple mode goes planning -> awaiting_approval -> running (a plan
        // is generated and approved first). Pro mode / the debug screen can
        // run a turn directly without ever generating a plan, so
        // planning -> running has to stay legal too.
        ("planning", "running") => true,
        ("awaiting_approval", "running") => true,
        ("awaiting_approval", "cancelled") => true,
        ("running", "completed") => true,
        ("running", "failed") => true,
        ("running", "cancelled") => true,
        ("running", "blocked") => true,
        ("blocked", "running") => true,
        ("blocked", "cancelled") => true,
        // Handoff / retry: start a new provider_run on a turn whose last
        // one already finished. See the module doc comment above.
        ("completed", "running") => true,
        ("failed", "running") => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_STATUSES: &[&str] = &[
        "planning",
        "awaiting_approval",
        "running",
        "blocked",
        "completed",
        "failed",
        "cancelled",
    ];

    #[test]
    fn same_state_is_always_a_valid_no_op() {
        for s in ALL_STATUSES {
            assert!(is_valid_transition(s, s), "{s} -> {s} should be valid");
        }
    }

    #[test]
    fn terminal_states_never_leave() {
        for terminal in TERMINAL {
            for target in ALL_STATUSES {
                if terminal == target {
                    continue;
                }
                assert!(
                    !is_valid_transition(terminal, target),
                    "{terminal} -> {target} must be rejected, terminal states are final"
                );
            }
        }
    }

    #[test]
    fn happy_path_is_fully_reachable() {
        assert!(is_valid_transition("planning", "awaiting_approval"));
        assert!(is_valid_transition("awaiting_approval", "running"));
        assert!(is_valid_transition("running", "completed"));
    }

    #[test]
    fn cancellation_reachable_from_every_non_terminal_state() {
        for s in ["planning", "awaiting_approval", "running", "blocked"] {
            assert!(is_valid_transition(s, "cancelled"), "{s} -> cancelled should be allowed");
        }
    }

    #[test]
    fn direct_run_without_a_plan_is_allowed_for_pro_mode_and_debug() {
        assert!(is_valid_transition("planning", "running"));
    }

    #[test]
    fn cannot_skip_planning_straight_to_completion() {
        assert!(!is_valid_transition("planning", "completed"));
        assert!(!is_valid_transition("awaiting_approval", "completed"));
    }

    #[test]
    fn blocked_can_resume_or_cancel_but_not_jump_to_completion() {
        assert!(is_valid_transition("blocked", "running"));
        assert!(is_valid_transition("blocked", "cancelled"));
        assert!(!is_valid_transition("blocked", "completed"));
        assert!(!is_valid_transition("blocked", "failed"));
    }

    #[test]
    fn completed_and_failed_can_reopen_for_handoff_or_retry_but_only_to_running() {
        assert!(is_valid_transition("completed", "running"));
        assert!(is_valid_transition("failed", "running"));
        for to in ["awaiting_approval", "planning", "blocked", "cancelled", "failed"] {
            assert!(!is_valid_transition("completed", to), "completed -> {to} should stay rejected");
        }
        for to in ["awaiting_approval", "planning", "blocked", "cancelled", "completed"] {
            assert!(!is_valid_transition("failed", to), "failed -> {to} should stay rejected");
        }
    }

    #[test]
    fn cancelled_is_the_only_true_terminal_state() {
        for target in ALL_STATUSES {
            if *target == "cancelled" {
                continue;
            }
            assert!(!is_valid_transition("cancelled", target), "cancelled -> {target} must stay rejected");
        }
    }

    #[test]
    fn exhaustive_matrix_matches_hand_written_allow_list() {
        let allowed: &[(&str, &str)] = &[
            ("planning", "awaiting_approval"),
            ("planning", "cancelled"),
            ("planning", "running"),
            ("awaiting_approval", "running"),
            ("awaiting_approval", "cancelled"),
            ("running", "completed"),
            ("running", "failed"),
            ("running", "cancelled"),
            ("running", "blocked"),
            ("blocked", "running"),
            ("blocked", "cancelled"),
            ("completed", "running"),
            ("failed", "running"),
        ];
        for from in ALL_STATUSES {
            for to in ALL_STATUSES {
                let expected = from == to || allowed.contains(&(*from, *to));
                assert_eq!(
                    is_valid_transition(from, to),
                    expected,
                    "{from} -> {to} mismatch"
                );
            }
        }
    }
}
