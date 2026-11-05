//! Platform-specific boundaries. Chat spawning itself never uses a shell.
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct Program { pub executable: PathBuf, pub prefix: Vec<String> }
impl Program {
    pub fn command(&self) -> Command {
        let mut c=Command::new(&self.executable);
        c.args(&self.prefix);
        if let Ok(path)=std::env::join_paths(search_dirs()) { c.env("PATH",path); }
        hide(&mut c);
        c
    }
}

pub fn home() -> PathBuf {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from).unwrap_or_default()
}
pub fn hide(command: &mut Command) {
    #[cfg(windows)] { use std::os::windows::process::CommandExt; command.creation_flags(0x08000000); }
    #[cfg(not(windows))] { let _=command; }
}
pub fn isolate(command: &mut Command) {
    hide(command);
    #[cfg(unix)] { use std::os::unix::process::CommandExt; command.process_group(0); }
}
fn is_executable(path: &Path) -> bool {
    if !path.is_file() { return false; }
    #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; return path.metadata().map(|m|m.permissions().mode() & 0o111 != 0).unwrap_or(false); }
    #[cfg(not(unix))] { true }
}
pub fn search_dirs() -> Vec<PathBuf> {
    let h=home();
    let mut dirs=vec![h.join(".local/bin"),h.join(".cargo/bin"),h.join(".grok/bin"),h.join(".npm-global/bin")];
    #[cfg(windows)] {
        if let Some(local)=std::env::var_os("LOCALAPPDATA") {
            let local=PathBuf::from(local);
            dirs.push(local.join("Programs/OpenAI/Codex/bin"));
            dirs.push(local.join("Programs/Ollama"));
        }
        if let Some(root)=std::env::var_os("SystemRoot") { dirs.push(PathBuf::from(root).join("System32")); }
    }
    #[cfg(unix)] { dirs.extend([PathBuf::from("/opt/homebrew/bin"),PathBuf::from("/usr/local/bin"),PathBuf::from("/usr/bin"),PathBuf::from("/bin")]); }
    if let Some(paths)=std::env::var_os("PATH") { dirs.extend(std::env::split_paths(&paths)); }
    dirs
}
fn native(name: &str) -> Option<Program> {
    if !name.bytes().all(|b| b.is_ascii_alphanumeric() || b==b'-') { return None; }
    let file=if cfg!(windows) {format!("{name}.exe")} else {name.to_string()};
    search_dirs().into_iter().map(|d|d.join(&file)).find(|p|is_executable(p)).map(|executable|Program{executable,prefix:vec![]})
}
pub fn resolve(name: &str) -> Option<Program> {
    if let Some(p)=native(name) { return Some(p); }
    // npm's Windows launchers are batch/PowerShell files. Resolve their JS
    // entry using the installed package metadata, then invoke Node directly.
    #[cfg(windows)] {
        let package=match name {"gemini"=>"@google/gemini-cli","claude"=>"@anthropic-ai/claude-code","codex"=>"@openai/codex",_=>return None};
        let root=PathBuf::from(std::env::var_os("APPDATA")?).join("npm/node_modules").join(package);
        let metadata: serde_json::Value=serde_json::from_slice(&std::fs::read(root.join("package.json")).ok()?).ok()?;
        let bin=metadata["bin"].as_str().or_else(||metadata["bin"][name].as_str())?;
        let entry=root.join(bin);
        if entry.is_file() {
            let mut node=native("node")?;
            node.prefix.push(entry.to_string_lossy().to_string());
            return Some(node);
        }
    }
    None
}
pub fn terminate(pid: u32) -> Result<(),String> {
    #[cfg(windows)] let mut c={let mut c=Command::new("taskkill");c.args(["/F","/T","/PID",&pid.to_string()]);c};
    #[cfg(unix)] let mut c={let mut c=Command::new("kill");c.args(["-TERM","--",&format!("-{pid}")]);c};
    hide(&mut c);
    let out=c.output().map_err(|e|e.to_string())?;
    if out.status.success() {Ok(())} else {Err(String::from_utf8_lossy(&out.stderr).trim().into())}
}
pub fn open_folder(path: &Path) -> Result<(),String> {
    #[cfg(windows)] let app="explorer.exe";
    #[cfg(target_os="macos")] let app="open";
    #[cfg(target_os="linux")] let app="xdg-open";
    let mut c=Command::new(app); c.arg(path); hide(&mut c);
    c.spawn().map(|_|()).map_err(|e|e.to_string())
}

pub fn login(tool: &str, data: &Path) -> Result<(),String> {
    let args:Vec<&str>=match tool {"claude"=>vec!["auth","login"],"codex"|"grok"=>vec!["login"],"gemini"=>vec![],"ollama"=>vec!["list"],_=>return Err("DeepSeek uses an API key; no browser sign-in is available.".into())};
    let program=resolve(tool).ok_or("Tool is not installed")?;
    std::fs::create_dir_all(data).map_err(|e|e.to_string())?;
    #[cfg(windows)] {
        let quote=|s:&str| format!("'{}'",s.replace('\'',"''"));
        let mut words=vec![quote(&program.executable.to_string_lossy())];
        words.extend(program.prefix.iter().map(|s|quote(s)));
        words.extend(args.iter().map(|s|quote(s)));
        let mut c=Command::new("powershell.exe");
        c.args(["-NoProfile","-NoExit","-Command",&format!("& {}",words.join(" "))]);
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x00000010);
        c.spawn().map(|_|()).map_err(|e|e.to_string())
    }
    #[cfg(target_os="macos")] {
        use std::os::unix::fs::PermissionsExt;
        let quote=|s:&str|format!("'{}'",s.replace('\'',"'\\''"));
        let mut words=vec![quote(&program.executable.to_string_lossy())];
        words.extend(program.prefix.iter().map(|s|quote(s)));
        words.extend(args.iter().map(|s|quote(s)));
        let script=data.join(format!("login-{tool}.command"));
        std::fs::write(&script,format!("#!/bin/sh\nexec {}\n",words.join(" "))).map_err(|e|e.to_string())?;
        std::fs::set_permissions(&script,std::fs::Permissions::from_mode(0o700)).map_err(|e|e.to_string())?;
        Command::new("open").args(["-a","Terminal"]).arg(script).spawn().map(|_|()).map_err(|e|e.to_string())
    }
    #[cfg(target_os="linux")] {
        for terminal in ["x-terminal-emulator","gnome-terminal","konsole","xterm"] {
            if let Some(p)=resolve(terminal) {
                let mut c=p.command();
                c.arg(if terminal=="gnome-terminal" {"--"} else {"-e"});
                c.arg(&program.executable).args(&program.prefix).args(&args);
                return c.spawn().map(|_|()).map_err(|e|e.to_string());
            }
        }
        Err("No terminal application found. Sign in using the tool's own terminal command, then return here.".into())
    }
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn rejects_shell_fragments_as_program_names() {
        assert!(resolve("claude; rm -rf").is_none());
        assert!(resolve("../codex").is_none());
    }
}
