. (Join-Path $PSScriptRoot '_common.ps1')
Apply-AllKeys
Ensure-PathEntry (Join-Path $HomeDir '.grok\bin')
Set-ToolWorkDirectory 'grok'
Show-ChatBanner 'Grok Build' 'Magenta' 'Ctrl+M = switch model | /model deepseek-v4'

if (-not (Require-Command 'grok' 'Run: grok update')) { return }
Set-Clipboard -Value 'Read CLAUDE.md and continue the project.'
grok