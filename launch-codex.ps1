. (Join-Path $PSScriptRoot '_common.ps1')
Apply-AllKeys
Set-ToolWorkDirectory 'codex'
Show-ChatBanner 'OpenAI Codex' 'Yellow' 'Tab to approve commands | sandbox on' -ToolId 'codex'

$codexDir = Join-Path $env:LOCALAPPDATA 'OpenAI\Codex\bin'
$codexExe = Get-ChildItem -Path $codexDir -Recurse -Filter 'codex.exe' -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending | Select-Object -First 1
if ($codexExe) { Ensure-PathEntry $codexExe.DirectoryName }

if (-not (Require-Command 'codex' 'Install from https://openai.com/codex')) { return }
Set-Clipboard -Value 'Continue the project from this folder.'
codex