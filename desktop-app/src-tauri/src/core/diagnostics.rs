use serde::Serialize;
#[derive(Serialize,Debug,Clone)]
#[serde(rename_all="camelCase")]
pub struct ToolStatus {pub name:String,pub installed:bool,pub version:Option<String>}
pub fn run_diagnostics()->Vec<ToolStatus> {
    ["git","node","claude","codex","gemini","grok","ollama"].iter().map(|name|ToolStatus{name:(*name).into(),installed:crate::platform::resolve(name).is_some(),version:None}).collect()
}
