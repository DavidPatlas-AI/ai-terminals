. (Join-Path $PSScriptRoot '_common.ps1')
Apply-AllKeys
Ensure-PathEntry (Join-Path $env:APPDATA 'npm')
Set-ToolWorkDirectory 'gemini'
Show-ChatBanner 'Gemini CLI' 'Blue' 'gemini -y = YOLO | -m gemini-2.5-flash'

if (-not $env:GEMINI_API_KEY) {
    Write-Host 'Missing Gemini API key!' -ForegroundColor Red
    Write-Host ''
    Write-Host '1. Get key: https://aistudio.google.com/apikey' -ForegroundColor Yellow
    Write-Host '2. Run setup-keys.bat and paste GEMINI_API_KEY' -ForegroundColor Yellow
    Write-Host ''
    Read-Host 'Press Enter to close'
    return
}

if (-not (Require-Command 'gemini' 'Run: npm install -g @google/gemini-cli')) { return }
Set-Clipboard -Value 'Continue the project from this folder. Reply in Hebrew when I write in Hebrew.'
gemini --skip-trust -y