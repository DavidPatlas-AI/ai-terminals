// Phase 2 — minimal Tauri backend for the AI Command Hub desktop shell.
//
// Exactly four commands are exposed to the frontend, all narrow and
// single-purpose. There is no generic shell-execution command and no
// unrestricted filesystem command: `check_project_folder` only ever reads
// the existence of a handful of well-known marker files inside the path the
// user explicitly picked via the native OS folder dialog, and
// `open_project_folder` only ever launches `explorer.exe` on that same path.

mod adapters;
mod core;
mod db;
mod platform;
mod worker;
pub fn run_worker() -> i32 { worker::run() }

use adapters::{claude_code, codex, gemini, ollama, RunControl};
use db::{models::*, repository, DbState};
use serde::Serialize;
use std::sync::{Arc, Mutex};
use tauri::{Manager, State, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_dialog::DialogExt;

/// Fetches the turn's current status and rejects `to` if
/// `core::turn_status::is_valid_transition` doesn't allow it — same rule
/// the adapter itself enforces for run-related transitions (see
/// adapters::claude_code::transition_turn); this is the Phase 5-command
/// side of the same guarantee.
fn validated_status_transition(
    conn: &rusqlite::Connection,
    turn_id: &str,
    to: &str,
) -> Result<(), String> {
    let current = repository::get_turn(conn, turn_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("turn {turn_id} not found"))?
        .status;
    if !core::turn_status::is_valid_transition(&current, to) {
        return Err(format!("illegal turn status transition: {current} -> {to}"));
    }
    Ok(())
}

/// Phase 8: every provider run is required to have a checkpoint first — a
/// checkpoint failure (e.g. git isn't installed) blocks the run rather than
/// silently skipping the safety net, since the whole point of Phase 8 is
/// that a provider's changes are always undoable.
fn checkpoint_before_run(
    app: &tauri::AppHandle,
    db: &Arc<Mutex<rusqlite::Connection>>,
    project_id: &str,
    project_path: &str,
    turn_id: &str,
) -> Result<(), String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("could not resolve app data dir for checkpointing: {e}"))?;
    let conn = db.lock().map_err(|e| e.to_string())?;
    core::git_checkpoint::create_checkpoint(&conn, &app_data_dir, project_id, project_path, turn_id)
        .map(|_| ())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    name: String,
    version: String,
    os: String,
    running_under_tauri: bool,
}

#[tauri::command]
fn app_info(app: tauri::AppHandle) -> AppInfo {
    let info = app.package_info();
    AppInfo {
        name: info.name.clone(),
        version: info.version.to_string(),
        os: std::env::consts::OS.to_string(),
        running_under_tauri: true,
    }
}

#[tauri::command]
fn select_project_folder(app: tauri::AppHandle) -> Option<String> {
    app.dialog()
        .file()
        .blocking_pick_folder()
        .map(|p| p.to_string())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectFolderInfo {
    exists: bool,
    is_directory: bool,
    folder_name: String,
    has_git: bool,
    has_package_json: bool,
    has_cargo_toml: bool,
    has_python_project: bool,
    detected_project_type: String,
}

#[tauri::command]
fn check_project_folder(path: String) -> ProjectFolderInfo {
    let p = std::path::Path::new(&path);

    let exists = p.exists();
    let is_directory = p.is_dir();
    let folder_name = p
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let has_git = p.join(".git").exists();
    let has_package_json = p.join("package.json").exists();
    let has_cargo_toml = p.join("Cargo.toml").exists();
    let has_python_project = p.join("requirements.txt").exists()
        || p.join("pyproject.toml").exists()
        || p.join("setup.py").exists();

    let detected_project_type = if has_cargo_toml {
        "rust"
    } else if has_package_json {
        "node"
    } else if has_python_project {
        "python"
    } else if has_git {
        "git-repo"
    } else {
        "unknown"
    }
    .to_string();

    ProjectFolderInfo {
        exists,
        is_directory,
        folder_name,
        has_git,
        has_package_json,
        has_cargo_toml,
        has_python_project,
        detected_project_type,
    }
}

#[tauri::command]
fn open_project_folder(path: String) -> Result<(), String> {
    let p = std::path::Path::new(&path);
    if !p.exists() || !p.is_dir() {
        return Err("Folder does not exist".to_string());
    }
    platform::open_folder(p)
}

// ---------------------------------------------------------------------------
// Phase 3 — debug-screen commands over the DB repository layer. These are
// intentionally thin: no validation beyond what SQLite's CHECK/FK constraints
// already enforce, since this is a debug screen, not the real approval flow
// (that's Phase 5).

#[tauri::command]
fn db_list_projects(state: State<DbState>) -> Result<Vec<Project>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    repository::list_projects(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn db_create_project(
    state: State<DbState>,
    name: String,
    path: String,
    icon: Option<String>,
) -> Result<Project, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    repository::create_project(&conn, &name, &path, icon.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
fn db_list_conversations(state: State<DbState>, project_id: String) -> Result<Vec<Conversation>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    repository::list_conversations_for_project(&conn, &project_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn db_create_conversation(
    state: State<DbState>,
    project_id: String,
    title: Option<String>,
) -> Result<Conversation, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    repository::create_conversation(&conn, &project_id, title.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
fn db_list_turns(state: State<DbState>, conversation_id: String) -> Result<Vec<Turn>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    repository::list_turns_for_conversation(&conn, &conversation_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn db_create_turn(
    state: State<DbState>,
    conversation_id: String,
    user_intent: String,
) -> Result<Turn, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let next_seq = repository::list_turns_for_conversation(&conn, &conversation_id)
        .map_err(|e| e.to_string())?
        .len() as i64
        + 1;
    repository::create_turn(&conn, &conversation_id, next_seq, &user_intent)
        .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Phase 4 — Claude Code provider adapter commands.

#[tauri::command]
fn detect_claude_code() -> claude_code::DetectResult {
    claude_code::detect_installed()
}

/// Starts the subprocess synchronously (so a real spawn failure surfaces
/// immediately as an Err) and returns the run_id right away, then drains
/// its output on a background thread. Returning before the run finishes is
/// what makes `cancel_run` useful — a version that blocked until completion
/// (like `claude_code::run`, used by tests) would only hand back a run_id
/// after there was nothing left to cancel.
#[tauri::command]
fn run_claude_code(
    app: tauri::AppHandle,
    state: State<DbState>,
    control: State<RunControl>,
    turn_id: String,
    project_id: String,
    project_path: String,
    prompt: String,
    permission_mode: Option<String>,
) -> Result<String, String> {
    let db = state.0.clone();
    checkpoint_before_run(&app, &db, &project_id, &project_path, &turn_id)?;
    let control_owned = control.inner().clone();
    let started = claude_code::start(
        db.clone(),
        Some(&control_owned),
        &claude_code::RunRequest {
            turn_id: &turn_id,
            project_path: &project_path,
            prompt: &prompt,
            permission_mode: permission_mode.as_deref().unwrap_or("acceptEdits"),
            resume_session_id: None,
        },
    )?;
    let run_id = started.run_id().to_string();

    let log_dir = app.path().app_data_dir().ok().map(|d| d.join("logs"));
    let app_owned = app.clone();
    std::thread::spawn(move || {
        let _ = claude_code::drain(
            started,
            db,
            Some(&app_owned),
            log_dir.as_deref(),
            Some(&control_owned),
            &turn_id,
        );
    });

    Ok(run_id)
}

// ---------------------------------------------------------------------------
// Phase 5 — Policy & Approval Engine: plan generation, approve/reject, cancel.

#[tauri::command]
fn generate_task_plan(
    state: State<DbState>,
    turn_id: String,
    project_path: String,
    user_intent: String,
) -> Result<TaskPlan, String> {
    let plan = claude_code::generate_plan(&project_path, &user_intent)?;
    let steps_json = serde_json::to_string(&plan.steps).map_err(|e| e.to_string())?;

    let conn = state.0.lock().map_err(|e| e.to_string())?;
    validated_status_transition(&conn, &turn_id, "awaiting_approval")?;
    let task_plan = repository::create_task_plan(&conn, &turn_id, &steps_json, Some(&plan.risk_level))
        .map_err(|e| e.to_string())?;
    repository::set_turn_task_plan(&conn, &turn_id, &task_plan.id).map_err(|e| e.to_string())?;
    repository::update_turn_status(&conn, &turn_id, "awaiting_approval").map_err(|e| e.to_string())?;
    Ok(task_plan)
}

#[tauri::command]
fn decide_task_plan(
    state: State<DbState>,
    turn_id: String,
    task_plan_id: String,
    decision: String,
) -> Result<Approval, String> {
    if decision != "approved" && decision != "rejected" {
        return Err(format!("decision must be 'approved' or 'rejected', got '{decision}'"));
    }
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let approval = repository::create_approval(&conn, &turn_id, "full_plan", Some(&task_plan_id), &decision)
        .map_err(|e| e.to_string())?;
    if decision == "rejected" {
        validated_status_transition(&conn, &turn_id, "cancelled")?;
        repository::update_turn_status(&conn, &turn_id, "cancelled").map_err(|e| e.to_string())?;
    }
    // On "approved" the status deliberately stays `awaiting_approval` —
    // actually starting the run (which flips it to `running`) is a
    // separate explicit run_claude_code call, so approving a plan never
    // silently starts executing it.
    Ok(approval)
}

#[tauri::command]
fn cancel_run(control: State<RunControl>, run_id: String) -> Result<(), String> {
    let pid = control
        .pid_of(&run_id)
        .ok_or_else(|| "no active run with that id".to_string())?;
    // Mark cancelled before killing: the adapter's reader thread checks this
    // flag to tell "user cancelled" apart from "process crashed" once the
    // stdout stream ends (see adapters::claude_code::run).
    control.mark_cancelled(&run_id);
    platform::terminate(pid)
}

// ---------------------------------------------------------------------------
// Phase 6 — Codex provider adapter + handoff from Claude Code.

#[tauri::command]
fn detect_codex() -> codex::DetectResult {
    codex::detect_installed()
}

/// Same start-then-drain-on-a-thread shape as run_claude_code, for the same
/// reason: the frontend needs a run_id back immediately, not after the
/// whole turn finishes, or the cancel button would be useless.
#[tauri::command]
fn run_codex(
    app: tauri::AppHandle,
    state: State<DbState>,
    control: State<RunControl>,
    turn_id: String,
    project_id: String,
    project_path: String,
    prompt: String,
) -> Result<String, String> {
    let db = state.0.clone();
    checkpoint_before_run(&app, &db, &project_id, &project_path, &turn_id)?;
    let control_owned = control.inner().clone();
    let started = codex::start(
        db.clone(),
        Some(&control_owned),
        &codex::RunRequest {
            turn_id: &turn_id,
            project_path: &project_path,
            prompt: &prompt,
            resume_session_id: None,
        },
    )?;
    let run_id = started.run_id().to_string();

    let log_dir = app.path().app_data_dir().ok().map(|d| d.join("logs"));
    let app_owned = app.clone();
    std::thread::spawn(move || {
        let _ = codex::drain(
            started,
            db,
            Some(&app_owned),
            log_dir.as_deref(),
            Some(&control_owned),
            &turn_id,
        );
    });

    Ok(run_id)
}

/// Scenario 12-13 (doc 08): builds the handoff summary from everything
/// Claude Code (or whichever provider ran before) left in the DB for this
/// turn, and starts a Codex run whose *initial prompt* is that summary —
/// not just the original request — so Codex actually knows what happened.
#[tauri::command]
fn handoff_to_codex(
    app: tauri::AppHandle,
    state: State<DbState>,
    control: State<RunControl>,
    turn_id: String,
    project_id: String,
    project_path: String,
) -> Result<String, String> {
    let db = state.0.clone();
    checkpoint_before_run(&app, &db, &project_id, &project_path, &turn_id)?;
    let summary = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        core::handoff::build_summary(&conn, &turn_id)?
    };

    let control_owned = control.inner().clone();
    let started = codex::start(
        db.clone(),
        Some(&control_owned),
        &codex::RunRequest {
            turn_id: &turn_id,
            project_path: &project_path,
            prompt: &summary,
            resume_session_id: None,
        },
    )?;
    let run_id = started.run_id().to_string();

    let log_dir = app.path().app_data_dir().ok().map(|d| d.join("logs"));
    let app_owned = app.clone();
    std::thread::spawn(move || {
        let _ = codex::drain(
            started,
            db,
            Some(&app_owned),
            log_dir.as_deref(),
            Some(&control_owned),
            &turn_id,
        );
    });

    Ok(run_id)
}

// ---------------------------------------------------------------------------
// Phase 10 — Ollama provider adapter. No checkpoint_before_run here (unlike
// Claude Code/Codex): Ollama in this phase is a plain local chat
// completion with no file-editing or shell-tool capability, so there is no
// project directory being modified to checkpoint.

#[tauri::command]
fn detect_ollama() -> ollama::DetectResult {
    ollama::detect_installed()
}

#[tauri::command]
fn run_ollama(
    app: tauri::AppHandle,
    state: State<DbState>,
    control: State<RunControl>,
    turn_id: String,
    prompt: String,
    model: Option<String>,
) -> Result<String, String> {
    let db = state.0.clone();
    let control_owned = control.inner().clone();
    let started = ollama::start(
        db.clone(),
        Some(&control_owned),
        &ollama::RunRequest {
            turn_id: &turn_id,
            prompt: &prompt,
            model: model.as_deref(),
        },
    )?;
    let run_id = started.run_id().to_string();

    let log_dir = app.path().app_data_dir().ok().map(|d| d.join("logs"));
    let app_owned = app.clone();
    std::thread::spawn(move || {
        let _ = ollama::drain(
            started,
            db,
            Some(&app_owned),
            log_dir.as_deref(),
            Some(&control_owned),
            &turn_id,
        );
    });

    Ok(run_id)
}

// ---------------------------------------------------------------------------
// Phase 10 — Gemini provider adapter. Unlike Ollama, Gemini DOES edit files
// in the project directory (like Claude Code/Codex), so it gets the same
// checkpoint_before_run safety net. NOTE: adapters::gemini's event parsing
// is unverified against real output — see that module's doc comment. This
// command works either way (a parser mismatch surfaces as a real "error"
// exit_status per the adapter's own conservative design, never a fake
// success), but treat any Gemini run's *results* with more skepticism than
// Claude Code/Codex/Ollama until someone with real credentials verifies it.

#[tauri::command]
fn detect_gemini() -> gemini::DetectResult {
    gemini::detect_installed()
}

#[tauri::command]
fn run_gemini(
    app: tauri::AppHandle,
    state: State<DbState>,
    control: State<RunControl>,
    turn_id: String,
    project_id: String,
    project_path: String,
    prompt: String,
) -> Result<String, String> {
    let db = state.0.clone();
    checkpoint_before_run(&app, &db, &project_id, &project_path, &turn_id)?;
    let control_owned = control.inner().clone();
    let started = gemini::start(
        db.clone(),
        Some(&control_owned),
        &gemini::RunRequest {
            turn_id: &turn_id,
            project_path: &project_path,
            prompt: &prompt,
            resume_session_id: None,
        },
    )?;
    let run_id = started.run_id().to_string();

    let log_dir = app.path().app_data_dir().ok().map(|d| d.join("logs"));
    let app_owned = app.clone();
    std::thread::spawn(move || {
        let _ = gemini::drain(
            started,
            db,
            Some(&app_owned),
            log_dir.as_deref(),
            Some(&control_owned),
            &turn_id,
        );
    });

    Ok(run_id)
}

// ---------------------------------------------------------------------------
// Phase 8 — Git Checkpoints (creation is automatic, see checkpoint_before_run)
// + install diagnostics.

#[tauri::command]
fn restore_checkpoint(app: tauri::AppHandle, state: State<DbState>, checkpoint_id: String) -> Result<(), String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("could not resolve app data dir: {e}"))?;
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    core::git_checkpoint::restore_checkpoint(&conn, &app_data_dir, &checkpoint_id)
}

#[tauri::command]
fn get_latest_checkpoint_for_turn(state: State<DbState>, turn_id: String) -> Result<Option<GitCheckpoint>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    repository::get_latest_git_checkpoint_for_turn(&conn, &turn_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn run_diagnostics() -> Vec<core::diagnostics::ToolStatus> {
    core::diagnostics::run_diagnostics()
}

#[tauri::command]
fn open_db_debug_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("db-debug") {
        return w.set_focus().map_err(|e| e.to_string());
    }
    WebviewWindowBuilder::new(&app, "db-debug", WebviewUrl::App("debug.html".into()))
        .title("The Box — Debug DB")
        .inner_size(640.0, 720.0)
        .build()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn connected_preferences(state: State<DbState>) -> Result<serde_json::Value, String> {
    let conn = state.0.lock().map_err(|e|e.to_string())?;
    let read = |key| repository::get_setting(&conn,key).ok().flatten().map(|s|s.value);
    Ok(serde_json::json!({"provider":read("chat_provider"),"projectId":read("chat_project")}))
}

#[tauri::command]
fn save_connected_preferences(state: State<DbState>, provider: String, project_id: Option<String>) -> Result<(), String> {
    if !adapters::connected::valid_tool(&provider) { return Err("Unknown tool".into()); }
    let conn = state.0.lock().map_err(|e|e.to_string())?;
    repository::set_setting(&conn,"chat_provider",&provider).map_err(|e|e.to_string())?;
    if let Some(id) = project_id {
        if repository::get_project(&conn,&id).map_err(|e|e.to_string())?.is_none() { return Err("Project not found".into()); }
        repository::set_setting(&conn,"chat_project",&id).map_err(|e|e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn connected_turn_replies(state: State<DbState>, turn_id: String) -> Result<Vec<String>, String> {
    let conn = state.0.lock().map_err(|e|e.to_string())?;
    let mut stmt = conn.prepare("SELECT message FROM logs WHERE turn_id=?1 AND level='info' AND provider_run_id IN (SELECT id FROM provider_runs WHERE turn_id=?1 AND exit_status='success') ORDER BY created_at, rowid").map_err(|e|e.to_string())?;
    let rows = stmt.query_map([turn_id], |r|r.get(0)).map_err(|e|e.to_string())?;
    rows.collect::<rusqlite::Result<Vec<String>>>().map_err(|e|e.to_string())
}

#[tauri::command]
fn run_connected_chat(app: tauri::AppHandle, state: State<DbState>, control: State<RunControl>, turn_id: String, project_id: String, tool: String) -> Result<String,String> {
    let project = {
        let conn=state.0.lock().map_err(|e|e.to_string())?;
        let p=repository::get_project(&conn,&project_id).map_err(|e|e.to_string())?.ok_or("Project not found")?;
        let turn=repository::get_turn(&conn,&turn_id).map_err(|e|e.to_string())?.ok_or("Turn not found")?;
        let conversation=repository::get_conversation(&conn,&turn.conversation_id).map_err(|e|e.to_string())?.ok_or("Conversation not found")?;
        if conversation.project_id != project_id { return Err("Turn belongs to another project".into()); }
        p
    };
    let mut candidates=vec![platform::home().join("Desktop/פקודות קלוד קוד/מודלים טרמנילים")];
    if let Some(root)=std::env::var_os("AI_TERMINALS_ROOT") {candidates.push(std::path::PathBuf::from(root));}
    if let Ok(exe)=std::env::current_exe() {for parent in exe.ancestors().skip(1).take(4) {candidates.push(parent.join("מודלים טרמנילים"));}}
    let root=candidates.iter().find(|p|p.join("AI.bat").is_file()).map(|p|p.to_string_lossy().to_string());
    let data=app.path().app_data_dir().map_err(|e|e.to_string())?;
    adapters::connected::start(app,state.0.clone(),control.inner().clone(),&turn_id,&project.path,&tool,None,&data,root.as_deref())
}

#[tauri::command]
fn open_connected_login(app: tauri::AppHandle, tool: String) -> Result<(),String> {
    let data=app.path().app_data_dir().map_err(|e|e.to_string())?;
    platform::login(&tool,&data)
}


#[tauri::command]
fn lite_bootstrap(app: tauri::AppHandle, state: State<DbState>) -> Result<serde_json::Value,String> {
    let data=app.path().app_data_dir().map_err(|e|e.to_string())?;
    let conn=state.0.lock().map_err(|e|e.to_string())?;
    let mut projects=repository::list_projects(&conn).map_err(|e|e.to_string())?;
    if projects.is_empty() {
        let workspace=data.join("workspace");
        std::fs::create_dir_all(&workspace).map_err(|e|e.to_string())?;
        projects.push(repository::create_project(&conn,"מרחב אישי",&workspace.to_string_lossy(),None).map_err(|e|e.to_string())?);
    }
    let read=|key:&str| repository::get_setting(&conn,key).ok().flatten().map(|s|s.value);
    let project_id=read("chat_project").filter(|id|projects.iter().any(|p|p.id==*id)).unwrap_or_else(||projects[0].id.clone());
    let conversation=read(&format!("chat_conversation_{project_id}"));
    let tools= ["claude","codex","gemini","grok","ollama","deepseek"].iter().map(|id| {
        let provider=if *id=="claude" {"claude_code"} else {*id};
        let last:Option<String>=conn.query_row("SELECT exit_status FROM provider_runs WHERE provider=?1 AND ended_at IS NOT NULL ORDER BY started_at DESC, rowid DESC LIMIT 1",[provider],|r|r.get(0)).ok();
        serde_json::json!({"id":id,"installed":platform::resolve(if *id=="deepseek" {"curl"} else {id}).is_some(),"lastStatus":last})
    }).collect::<Vec<_>>();
    Ok(serde_json::json!({"projects":projects,"projectId":project_id,"conversationId":conversation,"provider":read("chat_provider"),"economy":read("chat_economy").as_deref()!=Some("false"),"tools":tools,"platform":std::env::consts::OS,"arch":std::env::consts::ARCH}))
}

#[tauri::command]
fn save_lite_preferences(state: State<DbState>, provider:String, project_id:String, conversation_id:Option<String>, economy:bool) -> Result<(),String> {
    if !adapters::connected::valid_tool(&provider) {return Err("Unknown tool".into());}
    let conn=state.0.lock().map_err(|e|e.to_string())?;
    if repository::get_project(&conn,&project_id).map_err(|e|e.to_string())?.is_none() {return Err("Project not found".into());}
    if let Some(id)=&conversation_id {
        let conversation=repository::get_conversation(&conn,id).map_err(|e|e.to_string())?.ok_or("Conversation not found")?;
        if conversation.project_id!=project_id {return Err("Conversation belongs to another project".into());}
    }
    let transaction=conn.unchecked_transaction().map_err(|e|e.to_string())?;
    repository::set_setting(&transaction,"chat_provider",&provider).map_err(|e|e.to_string())?;
    repository::set_setting(&transaction,"chat_project",&project_id).map_err(|e|e.to_string())?;
    repository::set_setting(&transaction,"chat_economy",if economy {"true"} else {"false"}).map_err(|e|e.to_string())?;
    repository::set_setting(&transaction,&format!("chat_conversation_{project_id}"),conversation_id.as_deref().unwrap_or("")).map_err(|e|e.to_string())?;
    transaction.commit().map_err(|e|e.to_string())
}

#[tauri::command]
fn lite_history(state:State<DbState>, conversation_id:String, before_seq:Option<i64>) -> Result<serde_json::Value,String> {
    let conn=state.0.lock().map_err(|e|e.to_string())?;
    let mut query=conn.prepare("SELECT id,seq,user_intent,status FROM turns WHERE conversation_id=?1 AND seq<?2 ORDER BY seq DESC LIMIT 31").map_err(|e|e.to_string())?;
    let mut turns=query.query_map(rusqlite::params![conversation_id,before_seq.unwrap_or(i64::MAX)],|r|Ok((r.get::<_,String>(0)?,r.get::<_,i64>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?))).map_err(|e|e.to_string())?.collect::<rusqlite::Result<Vec<_>>>().map_err(|e|e.to_string())?;
    let more=turns.len()>30;
    turns.truncate(30);turns.reverse();
    let mut items=Vec::new();
    for (id,seq,user_intent,status) in turns {
        let mut stmt=conn.prepare("SELECT message FROM logs WHERE turn_id=?1 AND level='info' AND provider_run_id IN (SELECT id FROM provider_runs WHERE turn_id=?1) ORDER BY created_at, rowid").map_err(|e|e.to_string())?;
        let reply=stmt.query_map([&id],|r|r.get::<_,String>(0)).map_err(|e|e.to_string())?.collect::<rusqlite::Result<Vec<_>>>().map_err(|e|e.to_string())?.join("");
        items.push(serde_json::json!({"id":id,"seq":seq,"userIntent":user_intent,"status":status,"reply":reply}));
    }
    Ok(serde_json::json!({"items":items,"hasMore":more}))
}

#[tauri::command]
fn open_classic_window(app:tauri::AppHandle)->Result<(),String> {
    if let Some(w)=app.get_webview_window("classic") {return w.set_focus().map_err(|e|e.to_string());}
    WebviewWindowBuilder::new(&app,"classic",WebviewUrl::App("index.html".into())).title("The Box — כלים מתקדמים").inner_size(1120.0,760.0).build().map(|_|()).map_err(|e|e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let db_path = data_dir.join("the-box.sqlite3");
            let conn = db::init(&db_path).expect("failed to initialize database");
            app.manage(DbState(Arc::new(Mutex::new(conn))));
            app.manage(RunControl::new());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            lite_bootstrap,
            lite_history,
            save_lite_preferences,
            open_classic_window,
            connected_preferences,
            save_connected_preferences,
            connected_turn_replies,
            run_connected_chat,
            open_connected_login,
            app_info,
            select_project_folder,
            check_project_folder,
            open_project_folder,
            db_list_projects,
            db_create_project,
            db_list_conversations,
            db_create_conversation,
            db_list_turns,
            db_create_turn,
            open_db_debug_window,
            detect_claude_code,
            run_claude_code,
            generate_task_plan,
            decide_task_plan,
            cancel_run,
            detect_codex,
            run_codex,
            handoff_to_codex,
            restore_checkpoint,
            get_latest_checkpoint_for_turn,
            run_diagnostics,
            detect_ollama,
            run_ollama,
            detect_gemini,
            run_gemini
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
