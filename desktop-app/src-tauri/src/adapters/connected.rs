//! One read-only chat bridge using the same local accounts as AI Terminals.
use super::{transition_turn, with_db, ProviderEvent, RunControl};
use crate::db::repository;
use rusqlite::Connection;
use serde_json::json;
use std::io::{BufRead, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use tauri::Emitter;


pub fn valid_tool(tool: &str) -> bool {
    matches!(tool, "claude" | "codex" | "gemini" | "ollama" | "grok" | "deepseek")
}

/// Explicit, bounded replay keeps context across providers and app restarts.
/// Only completed replies are reused, never stderr, failed turns or tool logs.
pub fn chat_prompt(conn: &Connection, turn_id: &str) -> Result<String, String> {
    let current = repository::get_turn(conn, turn_id).map_err(|e| e.to_string())?
        .ok_or("Conversation turn not found")?;
    if current.user_intent.chars().count()>16000 {return Err("Message is too long (16,000 characters maximum)".into());}
    let economy=repository::get_setting(conn,"chat_economy").ok().flatten().map(|s|s.value!="false").unwrap_or(true);
    let mut query=conn.prepare("SELECT id,user_intent FROM turns WHERE conversation_id=?1 AND seq<?2 AND status='completed' ORDER BY seq DESC LIMIT ?3").map_err(|e|e.to_string())?;
    let turns=query.query_map(rusqlite::params![current.conversation_id,current.seq,if economy {4} else {8}],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).map_err(|e|e.to_string())?.collect::<rusqlite::Result<Vec<_>>>().map_err(|e|e.to_string())?;
    let mut history = Vec::new();
    let mut budget = if economy {6000usize} else {12000usize};
    for (turn_id,intent) in &turns {
        let mut stmt = conn.prepare("SELECT message FROM logs WHERE turn_id=?1 AND level='info' AND provider_run_id IN (SELECT id FROM provider_runs WHERE turn_id=?1 AND exit_status='success') ORDER BY created_at, rowid").map_err(|e|e.to_string())?;
        let replies = stmt.query_map([turn_id], |r| r.get::<_,String>(0)).map_err(|e|e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>().map_err(|e|e.to_string())?.join("");
        let segment = format!("User: {}\nAssistant: {}\n", intent, replies);
        let size = segment.chars().count();
        if size > budget { break; }
        budget -= size;
        history.push(segment);
    }
    history.reverse();
    Ok(format!("You are in read-only chat mode. Answer the user's message; do not use tools or modify files.\nRecent conversation (may be truncated):\n{}\nCurrent user message:\n{}", history.join("\n"), current.user_intent))
}

pub fn start(
    app: tauri::AppHandle, db: Arc<Mutex<Connection>>, control: RunControl,
    turn_id: &str, project_path: &str, tool: &str, model: Option<&str>,
    _data_dir: &Path, terminals_root: Option<&str>,
) -> Result<String, String> {
    if !valid_tool(tool) { return Err("Unknown tool".into()); }
    if !Path::new(project_path).is_dir() { return Err("Project folder not found".into()); }
    let prompt = with_db(&db, |conn| Ok(chat_prompt(conn, turn_id)))??;
    let economy=with_db(&db,|c|repository::get_setting(c,"chat_economy"))?.map(|s|s.value!="false").unwrap_or(true);
    transition_turn(&db, turn_id, "running")?;
    let provider = if tool == "claude" { "claude_code" } else { tool };
    let run = with_db(&db, |c| repository::create_provider_run(c, turn_id, provider, None))?;
    let executable=std::env::current_exe().map_err(|e|e.to_string())?;
    let mut cmd=Command::new(executable);
    cmd.arg("--box-worker").current_dir(project_path)
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    crate::platform::isolate(&mut cmd);
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => {
            let _ = with_db(&db, |c| repository::finish_provider_run(c, &run.id, "error"));
            let _ = transition_turn(&db, turn_id, "failed");
            return Err(format!("Could not start chat: {e}"));
        }
    };
    let request = json!({"tool":tool,"project":project_path,"prompt":prompt,"model":model,"terminalsRoot":terminals_root,"economy":economy});
    if let Err(e) = child.stdin.take().unwrap().write_all(request.to_string().as_bytes()) {
        let _ = child.kill();
        let _ = child.wait();
        let _ = with_db(&db, |c| repository::finish_provider_run(c, &run.id, "error"));
        let _ = transition_turn(&db, turn_id, "failed");
        return Err(e.to_string());
    }
    control.register(&run.id, child.id());
    let timeout_control=control.clone();
    let timeout_id=run.id.clone();
    std::thread::spawn(move || {
        for _ in 0..180 {
            std::thread::sleep(std::time::Duration::from_secs(1));
            if timeout_control.pid_of(&timeout_id).is_none() { return; }
        }
        if let Some(pid)=timeout_control.pid_of(&timeout_id) {
            let _=crate::platform::terminate(pid);
        }
    });
    let run_id = run.id.clone();
    let turn_id = turn_id.to_string();
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    std::thread::spawn(move || {
        let stderr_reader = std::thread::spawn(move || {
            crate::worker::read_tail(stderr,16000)
        });
        let mut terminal_success = false;
        let mut saw_text = false;
        let mut response = String::new();
        for line in std::io::BufReader::new(stdout).lines().map_while(Result::ok) {
            let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else { continue };
            let event = match v["type"].as_str() {
                Some("text_chunk") => {
                    let text = v["text"].as_str().unwrap_or("").to_string();
                    saw_text |= !text.is_empty();
                    response.push_str(&text);
                    Some(ProviderEvent::TextChunk { text })
                }
                Some("error") => {
                    let message = v["message"].as_str().unwrap_or("Chat failed").to_string();
                    let _ = with_db(&db, |c| repository::create_log(c, &turn_id, Some(&run.id), "error", &message));
                    Some(ProviderEvent::Error { message, fatal: true })
                }
                Some("done") => { terminal_success = v["exit_status"] == "success"; None }
                _ => None,
            };
            if let Some(event) = event {
                let _ = app.emit("provider_event", json!({"runId":run.id,"turnId":turn_id,"event":event}));
            }
        }
        let ok = child.wait().map(|s|s.success()).unwrap_or(false);
        let stderr = stderr_reader.join().unwrap_or_default();
        let saved_reply=if response.is_empty() {Ok(())} else {with_db(&db,|c|repository::create_log(c,&turn_id,Some(&run.id),"info",&response)).map(|_|())};
        if saved_reply.is_err() {
            let _=app.emit("provider_event",json!({"runId":run.id,"turnId":turn_id,"event":ProviderEvent::Error{message:"The response arrived but could not be saved. Check disk space; copy the response before closing.".into(),fatal:true}}));
        }
        let status = if control.was_cancelled(&run.id) { "cancelled" }
            else if ok && terminal_success && saw_text && saved_reply.is_ok() { "success" } else { "error" };
        control.unregister(&run.id);
        if status == "error" && !stderr.trim().is_empty() {
            let _ = app.emit("provider_event", json!({"runId":run.id,"turnId":turn_id,"event":ProviderEvent::Error{message:stderr.clone(), fatal:true}}));
        }
        if status == "error" && !terminal_success && stderr.trim().is_empty() {
            let _ = app.emit("provider_event", json!({"runId":run.id,"turnId":turn_id,"event":ProviderEvent::Error{message:"The chat did not complete. It may have timed out; your conversation is saved and you can choose another tool.".into(), fatal:true}}));
        }
        let _ = with_db(&db, |c| repository::finish_provider_run(c, &run.id, status));
        let _ = transition_turn(&db, &turn_id, match status {"success"=>"completed","cancelled"=>"cancelled",_=>"failed"});
        let _ = app.emit("provider_event", json!({"runId":run.id,"turnId":turn_id,"event":ProviderEvent::Done{exit_status:status.into()}}));
    });
    Ok(run_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn context_survives_provider_changes_and_excludes_failures() {
        let conn = crate::db::init_in_memory().unwrap();
        let p = repository::create_project(&conn,"Test","C:/test",None).unwrap();
        let c = repository::create_conversation(&conn,&p.id,None).unwrap();
        let first = repository::create_turn(&conn,&c.id,1,"Remember CODE-42").unwrap();
        let run = repository::create_provider_run(&conn,&first.id,"grok",None).unwrap();
        repository::create_log(&conn,&first.id,Some(&run.id),"info","Remembered CODE-42").unwrap();
        repository::finish_provider_run(&conn,&run.id,"success").unwrap();
        repository::update_turn_status(&conn,&first.id,"completed").unwrap();
        let bad = repository::create_turn(&conn,&c.id,2,"FAILED_SECRET").unwrap();
        repository::update_turn_status(&conn,&bad.id,"failed").unwrap();
        let next = repository::create_turn(&conn,&c.id,3,"What code?").unwrap();
        let prompt = chat_prompt(&conn,&next.id).unwrap();
        assert!(prompt.contains("Remembered CODE-42"));
        assert!(prompt.contains("What code?"));
        assert!(!prompt.contains("FAILED_SECRET"));
        assert!(!valid_tool("powershell"));
    }
}
