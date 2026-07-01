. (Join-Path $PSScriptRoot '_common.ps1')
Apply-AllKeys
if ($script:GROQ_API_KEY) { $env:GROQ_API_KEY = $script:GROQ_API_KEY }
Set-WorkDirectory
Show-ChatBanner 'OpenCode' 'Cyan' 'Ctrl+O switch model | free with Gemini/Groq key'

if (-not $env:GEMINI_API_KEY -and -not $env:GROQ_API_KEY) {
    Write-Host 'Need free API key - run setup-keys.bat' -ForegroundColor Red
    Write-Host 'Gemini: https://aistudio.google.com/apikey' -ForegroundColor Yellow
    Write-Host 'Groq:   https://console.groq.com/keys' -ForegroundColor Yellow
    Read-Host 'Press Enter to close'
    return
}

if (-not (Require-Command 'opencode' 'Run: npm install -g opencode-ai')) { return }
Set-Clipboard -Value 'Continue the project from this folder.'
opencode