# AI Terminals File Map

`C:\Users\DAVID\Desktop\AI-Terminals` is a junction to
`C:\Users\DAVID\Desktop\מודלים טרמנילים`. These are two names for the same
folder, not two separate copies.

## Main Entry Points

- `AI.bat` - one command for every model, for example `AI.bat codex`
- `START-HERE.bat` - full menu for setup, dashboard, keys, checks, and docs
- `start.bat` - opens the AI Hub menu
- `hub.ps1` - the AI Hub menu logic
- `open-dashboard.bat` - opens the dashboard

## Universal Launcher

- `launch-ai.ps1` - the single launch engine for terminal AI tools
- `_models.ps1` - model catalog and launch profiles
- `_common.ps1` - shared key, path, banner, and working-folder helpers
- `work-folders-config.ps1` - edit this to choose where tools start

## Compatibility Wrappers

These files are kept so old shortcuts and muscle memory still work. They are
not separate systems anymore.

- `Grok.bat`, `Claude.bat`, `Gemini.bat`, `DeepSeek.bat`, `Codex.bat`
- `Crush.bat`, `Ollama.bat`, `NotebookLM.bat`, `OpenCode.bat`
- `AI-Grok.bat`, `AI-Claude.bat`, `AI-Gemini.bat`, `AI-DeepSeek.bat`
- `AI-Codex.bat`, `AI-Crush.bat`, `AI-Ollama.bat`, `AI-NotebookLM.bat`
- `AI-OpenCode.bat`
- `launch-grok.ps1`, `launch-claude.ps1`, `launch-gemini.ps1`
- `launch-deepseek.ps1`, `launch-codex.ps1`, `launch-crush.ps1`
- `launch-ollama.ps1`, `launch-opencode.ps1`

## Dashboard And Status

- `check-all.ps1` / `check-all.bat` - full status refresh
- `refresh-conversations.ps1` / `.bat` - conversation-only refresh
- `_status.ps1` - model health checks
- `_conversations.ps1` - chat/session scanners
- `dashboard-template.html` - dashboard source template
- `dashboard.html`, `status.json`, `STATUS.txt` - generated local outputs

## Setup And Maintenance

- `setup-all.bat` - complete setup flow
- `setup-desktop.bat`, `cleanup-desktop.ps1` - desktop cleanup and junction
- `setup-work-shortcuts.bat`, `setup-work-shortcuts.ps1` - shortcuts folder
- `setup-keys.bat`, `setup-keys.ps1` - API key setup
- `security-check.bat`, `security-check.ps1` - pre-publish key scan
- `publish-github.bat`, `publish-github.ps1` - publish flow

## Hebrew RTL Tools

- `fix-hebrew-chat.bat` - menu for Cursor Hebrew fixes
- `check-rtl-status.ps1`
- `enable-hebrew-rtl.bat`, `enable-hebrew-rtl.ps1`
- `inject-rtl-streamdown-fix.ps1`
- `fix-streamdown-rtl.vbs`, `fix-hebrew-rtl.vbs`, `fix-hebrew-rtl-full.vbs`
- `paste-rtl-now.bat`, `restart-cursor-for-rtl.bat`, `open-cursor.bat`

## Local / Private Files

- `ai-secrets.ps1` - local API keys, do not publish
- `notebooklm-config.ps1` - NotebookLM notebook config
- `.gitignore` already keeps secrets and generated dashboard files out of Git

## Rule Of Thumb

Use `AI.bat <model>` or `START-HERE.bat`. Keep the wrapper files unless you are
sure no shortcut points to them.
