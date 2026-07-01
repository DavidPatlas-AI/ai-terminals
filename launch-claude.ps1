. (Join-Path $PSScriptRoot '_common.ps1')
Apply-AllKeys
Set-WorkDirectory
Show-ChatBanner 'Claude Code' 'Green' 'Paste from clipboard | ! for shell commands'

$hint = 'npm install -g @anthropic-ai/claude-code'
if (-not (Require-Command 'claude' $hint)) { return }
Set-Clipboard -Value 'Read the project and continue. Work in Hebrew when I write in Hebrew.'
claude