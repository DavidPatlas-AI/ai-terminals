//! Native worker mode of the app binary. No shell, Node sidecar or PowerShell bridge.
use crate::platform;
use serde_json::{json, Value};
use std::io::{BufRead, Read, Write};
use std::path::Path;
use std::process::Stdio;

fn emit(v: Value) { let mut out=std::io::stdout().lock(); let _=writeln!(out,"{v}"); let _=out.flush(); }
pub fn read_tail(mut input: impl Read, limit: usize) -> String {
    let mut saved=Vec::new(); let mut buffer=[0u8;4096];
    while let Ok(n)=input.read(&mut buffer) {
        if n==0 {break;}
        saved.extend_from_slice(&buffer[..n]);
        if saved.len()>limit { saved.drain(..saved.len()-limit); }
    }
    String::from_utf8_lossy(&saved).to_string()
}
fn text(s:&str) { if !s.is_empty() {emit(json!({"type":"text_chunk","text":s}));} }
fn field<'a>(v:&'a Value,key:&str)->Result<&'a str,String> {v[key].as_str().ok_or_else(||format!("Missing {key}"))}
fn config_quote(s:&str)->String {format!("\"{}\"",s.replace('\\',"\\\\").replace('"',"\\\"").replace('\n',"\\n").replace('\r',"\\r"))}

fn http(url:&str, body:Option<Value>, key:Option<&str>)->Result<Value,String> {
    let program=platform::resolve("curl").ok_or("curl is required for Ollama and DeepSeek connections")?;
    let mut command=program.command();
    command.args(["--silent","--show-error","--fail","--connect-timeout","8","--max-time","150","--config","-"])
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut config=format!("url = {}\n",config_quote(url));
    if let Some(key)=key {
        if key.contains(['\r','\n']) || key.len()>4096 {return Err("Invalid API key format".into());}
        config.push_str(&format!("header = {}\n",config_quote(&format!("Authorization: Bearer {key}"))));
    }
    if let Some(body)=body {
        config.push_str("request = \"POST\"\nheader = \"Content-Type: application/json\"\n");
        config.push_str(&format!("data = {}\n",config_quote(&body.to_string())));
    }
    let mut child=command.spawn().map_err(|e|e.to_string())?;
    child.stdin.take().ok_or("No HTTP stdin")?.write_all(config.as_bytes()).map_err(|e|e.to_string())?;
    let output=child.wait_with_output().map_err(|e|e.to_string())?;
    if !output.status.success() {return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());}
    serde_json::from_slice(&output.stdout).map_err(|_|"Service returned an invalid response".into())
}
fn legacy_key(root:&Path)->Option<String> {
    let mut candidates=vec![root.join("ai-secrets.ps1")];
    if let Ok(entries)=std::fs::read_dir(root) {
        for entry in entries.flatten().filter(|e|e.file_name().to_string_lossy().starts_with("09")) {
            candidates.push(entry.path().join("ai-secrets.ps1"));
        }
    }
    for file in candidates {
        let Ok(contents)=std::fs::read_to_string(file) else {continue;};
        for line in contents.lines() {
            let Some((name,value))=line.trim().split_once('=') else {continue;};
            if !matches!(name.trim(),"$script:DEEPSEEK_API_KEY"|"$env:DEEPSEEK_API_KEY"|"$DEEPSEEK_API_KEY") {continue;}
            let value=value.trim(); let quote=value.chars().next()?;
            if quote!='\'' && quote!='"' {continue;}
            if let Some(end)=value[1..].find(quote) {
                let key=&value[1..end+1];
                if key.starts_with("sk-") && key!="sk-..." {return Some(key.into());}
            }
        }
    }
    None
}

fn service(req:&Value,tool:&str,prompt:&str)->Result<(),String> {
    let max=if req["economy"].as_bool().unwrap_or(true) {768} else {2048};
    if tool=="ollama" {
        let tags=http("http://127.0.0.1:11434/api/tags",None,None)?;
        let model=req["model"].as_str().or_else(||tags["models"][0]["name"].as_str())
            .ok_or("No local model is installed. No download was started.")?;
        let r=http("http://127.0.0.1:11434/api/generate",Some(json!({"model":model,"prompt":prompt,"stream":false,"options":{"num_predict":max}})),None)?;
        let answer=r["response"].as_str().filter(|s|!s.is_empty()).ok_or("No local answer was returned")?;
        if r["done"]!=true {return Err("Local answer did not complete".into());}
        text(answer);
    } else {
        let key=std::env::var("DEEPSEEK_API_KEY").ok().filter(|k|!k.is_empty())
            .or_else(||req["terminalsRoot"].as_str().and_then(|r|legacy_key(Path::new(r))))
            .ok_or("DeepSeek: configure DEEPSEEK_API_KEY once; it is not stored in the chat database.")?;
        let model=req["model"].as_str().unwrap_or("deepseek-v4-flash");
        let r=http("https://api.deepseek.com/chat/completions",Some(json!({"model":model,"messages":[{"role":"user","content":prompt}],"max_tokens":max})),Some(&key))?;
        let answer=r["choices"][0]["message"]["content"].as_str().filter(|s|!s.is_empty()).ok_or("DeepSeek returned no answer")?;
        text(answer);
    }
    Ok(())
}

pub fn cli_args(tool:&str)->Vec<String> {
    let args:&[&str]=match tool {
        "claude"=>&["-p","--tools","","--output-format","stream-json","--verbose","--max-turns","1","--disable-slash-commands","--strict-mcp-config"],
        "codex"=>&["exec","--ephemeral","--skip-git-repo-check","--sandbox","read-only","--json","-"],
        "gemini"=>&["-p","Answer the request on stdin. Do not use tools.","--output-format","stream-json","--approval-mode","plan"],
        "grok"=>&["-p","Answer the request on stdin. Do not use tools.","--output-format","json","--max-turns","1","--no-subagents","--disable-web-search","--tools","","--permission-mode","plan"],
        _=>&[],
    };
    args.iter().map(|s|s.to_string()).collect()
}

fn cli(tool:&str,project:&Path,prompt:&str)->Result<(),String> {
    let program=platform::resolve(tool).ok_or_else(||format!("{tool} is not installed"))?;
    let mut cmd=program.command();
    let mut arguments=cli_args(tool);
    if tool=="grok" {
        if prompt.encode_utf16().count()>24000 {return Err("Message is too long. Shorten it or start a new conversation.".into());}
        arguments.remove(1);
        arguments[0]=format!("--single={prompt}");
    }
    cmd.args(arguments).current_dir(project).stdin(if tool=="grok" {Stdio::null()} else {Stdio::piped()}).stdout(Stdio::piped()).stderr(Stdio::piped());
    if tool=="claude" {
        for name in ["ANTHROPIC_BASE_URL","ANTHROPIC_AUTH_TOKEN","ANTHROPIC_MODEL","ANTHROPIC_DEFAULT_OPUS_MODEL","ANTHROPIC_DEFAULT_SONNET_MODEL","ANTHROPIC_DEFAULT_HAIKU_MODEL"] {cmd.env_remove(name);}
    }
    let mut child=cmd.spawn().map_err(|e|format!("Could not start {tool}: {e}"))?;
    if tool!="grok" {
        if let Err(e)=child.stdin.take().ok_or("No chat stdin")?.write_all(prompt.as_bytes()) {
            let _=child.kill(); let _=child.wait(); return Err(e.to_string());
        }
    }
    let stderr=child.stderr.take().ok_or("No chat stderr")?;
    let errors=std::thread::spawn(move||read_tail(stderr,8000));
    let stdout=child.stdout.take().ok_or("No chat output")?;
    let mut success=false;let mut saw_text=false;let mut grok=String::new();let mut provider_error=None;
    for line in std::io::BufReader::new(stdout).lines().map_while(Result::ok) {
        if tool=="grok" {grok.push_str(&line);grok.push('\n');continue;}
        let Ok(v)=serde_json::from_str::<Value>(&line) else {continue;};
        let kind=v["type"].as_str().unwrap_or("");
        match tool {
            "claude" if kind=="result" => {
                success=v["subtype"]=="success" && v["is_error"]!=true;
                if let Some(answer)=v["result"].as_str() {
                    if success {text(answer);saw_text=!answer.is_empty();} else {provider_error=Some(answer.to_string());}
                }
            }
            "codex" => {
                if kind=="item.completed" && v["item"]["type"]=="agent_message" {
                    if let Some(answer)=v["item"]["text"].as_str() {text(answer);saw_text|=!answer.is_empty();}
                }
                if kind=="turn.completed" {success=true;}
                if kind=="turn.failed" || kind=="error" {success=false;provider_error=v["error"]["message"].as_str().or_else(||v["message"].as_str()).map(str::to_string);}
            }
            "gemini" => {
                if kind=="message" && v["role"]=="assistant" {
                    if let Some(answer)=v["content"].as_str() {text(answer);saw_text|=!answer.is_empty();}
                }
                if kind=="result" {success=v["status"]=="success";}
                if kind=="error" {provider_error=v["message"].as_str().map(str::to_string);}
            }
            _=>{}
        }
    }
    let status=child.wait().map_err(|e|e.to_string())?;
    let errors=errors.join().unwrap_or_default();
    if tool=="grok" && status.success() {
        let result:Value=serde_json::from_str(&grok).map_err(|_|"Grok returned an invalid response")?;
        if let Some(answer)=result["text"].as_str().filter(|s|!s.is_empty()) {text(answer);success=true;saw_text=true;}
    }
    if !status.success() || !success || !saw_text {
        return Err(provider_error.unwrap_or_else(||format!("{tool} did not complete the chat. {}",errors.trim())));
    }
    Ok(())
}

pub fn run()->i32 {
    let result=(|| {
        let mut input=String::new();std::io::stdin().read_to_string(&mut input).map_err(|e|e.to_string())?;
        let req:Value=serde_json::from_str(&input).map_err(|e|e.to_string())?;
        let tool=field(&req,"tool")?;
        if !crate::adapters::connected::valid_tool(tool) {return Err("Unknown tool".into());}
        let project=Path::new(field(&req,"project")?);
        if !project.is_dir() {return Err("Project folder not found".into());}
        let prompt=field(&req,"prompt")?;
        if matches!(tool,"ollama"|"deepseek") {service(&req,tool,prompt)} else {cli(tool,project,prompt)}
    })();
    match result {
        Ok(())=>{emit(json!({"type":"done","exit_status":"success"}));0}
        Err(message)=>{emit(json!({"type":"error","message":message,"fatal":true}));emit(json!({"type":"done","exit_status":"error"}));1}
    }
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn no_shell_or_auto_approval_in_chat_args() {
        assert!(cli_args("claude").windows(2).any(|w|w[0]=="--tools" && w[1].is_empty()));
        assert!(cli_args("grok").windows(2).any(|w|w[0]=="--tools" && w[1].is_empty()));
        assert!(cli_args("codex").contains(&"read-only".into()));
        assert!(!cli_args("gemini").contains(&"--yolo".into()));
    }
    #[test] fn curl_config_preserves_json_and_blocks_new_directives() {
        let quoted=config_quote("quoted\"\nheader = secret");
        assert!(!quoted.contains('\n'));
        assert!(quoted.contains("\\n"));
    }
    #[test] fn stderr_is_fully_drained_but_memory_is_bounded() {
        assert_eq!(read_tail(&b"123456789"[..],4),"6789");
    }
}
