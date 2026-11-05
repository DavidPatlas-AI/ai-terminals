// The Box — Phase 8: Git Checkpoint Service.
//
// Verified empirically 2026-07-31 before writing any Rust: checkpointing
// uses a *shadow* git directory per project — `git --git-dir=<appdata>/
// checkpoints/<project_id> --work-tree=<project_path> ...` — instead of the
// project's own `.git`. Two things this buys, both confirmed by hand:
//
//   1. It works whether or not the project is already a git repo, and
//      never touches that repo's own history/branches/stash — git refuses
//      to `add` a path literally named `.git`, so a real nested `.git/`
//      is automatically excluded from what the shadow repo tracks, and
//      the shadow repo's own HEAD/refs live entirely outside the project
//      directory.
//   2. Restore is `reset --hard <checkpoint>` + `clean -fd`: reset alone
//      only fixes tracked files (leaves newly-created untracked files
//      behind), so `clean -fd` is required to actually remove files the
//      provider created after the checkpoint — verified by hand: a
//      modified file, a deleted file, and a brand-new file were all
//      correctly restored/removed, while a file explicitly kept untouched
//      stayed untouched.

use crate::db::models::GitCheckpoint;
use crate::db::repository;
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::process::Command;

fn shadow_git_dir(app_data_dir: &Path, project_id: &str) -> PathBuf {
    app_data_dir.join("checkpoints").join(project_id)
}

fn run_git(git_dir: &Path, work_tree: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("--git-dir")
        .arg(git_dir)
        .arg("--work-tree")
        .arg(work_tree)
        .args(args)
        .output()
        .map_err(|e| format!("failed to launch git: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn ensure_shadow_repo(git_dir: &Path) -> Result<(), String> {
    if git_dir.join("HEAD").exists() {
        return Ok(());
    }
    std::fs::create_dir_all(git_dir).map_err(|e| e.to_string())?;
    let output = Command::new("git")
        .arg("--git-dir")
        .arg(git_dir)
        .arg("init")
        .arg("-q")
        .output()
        .map_err(|e| format!("failed to launch git init: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "git init failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

/// Snapshots `project_path` as it is right now and records it in
/// `git_checkpoints`. Safe to call even if nothing changed since the last
/// checkpoint (`--allow-empty`) and even if the project has never been
/// checkpointed before (initializes the shadow repo on first use).
pub fn create_checkpoint(
    conn: &Connection,
    app_data_dir: &Path,
    project_id: &str,
    project_path: &str,
    turn_id: &str,
) -> Result<GitCheckpoint, String> {
    let git_dir = shadow_git_dir(app_data_dir, project_id);
    ensure_shadow_repo(&git_dir)?;
    let work_tree = Path::new(project_path);

    run_git(&git_dir, work_tree, &["-c", "user.email=the-box@local", "-c", "user.name=The Box", "add", "-A"])?;
    let msg = format!("checkpoint before turn {turn_id}");
    run_git(
        &git_dir,
        work_tree,
        &[
            "-c", "user.email=the-box@local", "-c", "user.name=The Box",
            "commit", "--allow-empty", "-q", "-m", &msg,
        ],
    )?;

    let hash = run_git(&git_dir, work_tree, &["rev-parse", "HEAD"])?;
    repository::create_git_checkpoint(conn, turn_id, project_id, &hash).map_err(|e| e.to_string())
}

/// Resets `project_path` to exactly match a previously-created checkpoint:
/// tracked-file changes are reverted, and files created after the
/// checkpoint (untracked at that commit) are removed. Marks the checkpoint
/// row as restored.
pub fn restore_checkpoint(
    conn: &Connection,
    app_data_dir: &Path,
    checkpoint_id: &str,
) -> Result<(), String> {
    let checkpoint = repository::get_git_checkpoint(conn, checkpoint_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("checkpoint {checkpoint_id} not found"))?;
    let project = repository::get_project(conn, &checkpoint.project_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("project {} not found", checkpoint.project_id))?;

    let git_dir = shadow_git_dir(app_data_dir, &checkpoint.project_id);
    let work_tree = Path::new(&project.path);

    run_git(&git_dir, work_tree, &["reset", "--hard", "-q", &checkpoint.commit_or_stash_ref])?;
    run_git(&git_dir, work_tree, &["clean", "-fd", "-q"])?;

    repository::mark_git_checkpoint_restored(conn, checkpoint_id).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_in_memory;

    struct ScratchDir(PathBuf);
    impl ScratchDir {
        fn new(label: &str) -> Self {
            let path = std::env::temp_dir().join(format!("the-box-checkpoint-test-{label}-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&path).unwrap();
            ScratchDir(path)
        }
    }
    impl Drop for ScratchDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn setup_turn(conn: &Connection, project_path: &str) -> (String, String) {
        let project = repository::create_project(conn, "Test", project_path, None).unwrap();
        let conversation = repository::create_conversation(conn, &project.id, None).unwrap();
        let turn = repository::create_turn(conn, &conversation.id, 1, "x").unwrap();
        (project.id, turn.id)
    }

    #[test]
    fn checkpoint_create_and_restore_roundtrip() {
        let work = ScratchDir::new("work");
        let app_data = ScratchDir::new("appdata");
        let work_path = work.0.to_str().unwrap();

        std::fs::write(work.0.join("file1.txt"), "original content").unwrap();
        std::fs::write(work.0.join("file2.txt"), "keep me").unwrap();
        std::fs::create_dir_all(work.0.join("subdir")).unwrap();
        std::fs::write(work.0.join("subdir/file3.txt"), "nested").unwrap();

        let conn = init_in_memory().unwrap();
        let (project_id, turn_id) = setup_turn(&conn, work_path);

        let checkpoint = create_checkpoint(&conn, &app_data.0, &project_id, work_path, &turn_id).unwrap();
        assert!(!checkpoint.commit_or_stash_ref.is_empty());
        assert!(checkpoint.restored_at.is_none());

        // Simulate a provider turn: modify, delete, and add files.
        std::fs::write(work.0.join("file1.txt"), "MODIFIED by AI").unwrap();
        std::fs::remove_file(work.0.join("file2.txt")).unwrap();
        std::fs::write(work.0.join("file4.txt"), "new file from AI").unwrap();
        std::fs::write(work.0.join("subdir/file3.txt"), "modified nested").unwrap();

        restore_checkpoint(&conn, &app_data.0, &checkpoint.id).unwrap();

        assert_eq!(std::fs::read_to_string(work.0.join("file1.txt")).unwrap(), "original content");
        assert_eq!(std::fs::read_to_string(work.0.join("file2.txt")).unwrap(), "keep me");
        assert_eq!(std::fs::read_to_string(work.0.join("subdir/file3.txt")).unwrap(), "nested");
        assert!(!work.0.join("file4.txt").exists(), "file created after the checkpoint must be removed by restore");

        let refreshed = repository::get_git_checkpoint(&conn, &checkpoint.id).unwrap().unwrap();
        assert!(refreshed.restored_at.is_some());
    }

    #[test]
    fn checkpoint_never_touches_the_projects_own_real_git_repo() {
        let work = ScratchDir::new("work-realgit");
        let app_data = ScratchDir::new("appdata-realgit");
        let work_path = work.0.to_str().unwrap();

        // The project is ALSO a real git repo with its own commit.
        run_git_plain(&work.0, &["init", "-q"]);
        std::fs::write(work.0.join("readme.md"), "real project file").unwrap();
        run_git_plain(&work.0, &["add", "readme.md"]);
        run_git_plain(&work.0, &["-c", "user.email=real@user", "-c", "user.name=RealUser", "commit", "-q", "-m", "real user's own commit"]);
        let real_head_before = run_git_plain_output(&work.0, &["rev-parse", "HEAD"]);

        let conn = init_in_memory().unwrap();
        let (project_id, turn_id) = setup_turn(&conn, work_path);
        create_checkpoint(&conn, &app_data.0, &project_id, work_path, &turn_id).unwrap();

        let real_head_after = run_git_plain_output(&work.0, &["rev-parse", "HEAD"]);
        assert_eq!(real_head_before, real_head_after, "the project's own git history must be untouched");

        // The shadow repo must not have tracked the nested .git directory.
        let shadow_git_dir = shadow_git_dir(&app_data.0, &project_id);
        let tracked = run_git_output(&shadow_git_dir, &["ls-tree", "-r", "--name-only", "HEAD"]);
        assert!(!tracked.contains(".git"), "shadow checkpoint must never track the project's own .git: {tracked}");
        assert!(tracked.contains("readme.md"));
    }

    fn run_git_plain(dir: &Path, args: &[&str]) {
        let status = Command::new("git").current_dir(dir).args(args).status().unwrap();
        assert!(status.success(), "git {args:?} failed");
    }

    fn run_git_plain_output(dir: &Path, args: &[&str]) -> String {
        let out = Command::new("git").current_dir(dir).args(args).output().unwrap();
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    fn run_git_output(git_dir: &Path, args: &[&str]) -> String {
        let out = Command::new("git").arg("--git-dir").arg(git_dir).args(args).output().unwrap();
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }
}
