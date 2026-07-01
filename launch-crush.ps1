. (Join-Path $PSScriptRoot '_common.ps1')
Apply-AllKeys
if ($script:GROQ_API_KEY) { $env:GROQ_API_KEY = $script:GROQ_API_KEY }
if ($script:OPENROUTER_API_KEY) { $env:OPENROUTER_API_KEY = $script:OPENROUTER_API_KEY }
if ($script:HF_TOKEN) { $env:HF_TOKEN = $script:HF_TOKEN }
Set-ToolWorkDirectory 'crush'
Show-ChatBanner 'Crush' 'Magenta' 'Ctrl+O switch model | works with Groq/Gemini free keys'

if (-not $env:GEMINI_API_KEY -and -not $env:GROQ_API_KEY -and -not $env:OPENROUTER_API_KEY) {
    Write-Host 'Need a FREE API key!' -ForegroundColor Red
    Write-Host ''
    Write-Host 'Option A - Gemini (free): https://aistudio.google.com/apikey' -ForegroundColor Yellow
    Write-Host 'Option B - Groq (free):   https://console.groq.com/keys' -ForegroundColor Yellow
    Write-Host 'Then run: setup-keys.bat' -ForegroundColor Yellow
    Write-Host ''
    Read-Host 'Press Enter to close'
    return
}

if (-not (Require-Command 'crush' 'Run: npm install -g @charmland/crush')) { return }
Set-Clipboard -Value 'Continue the project. Reply in Hebrew when I write in Hebrew.'
crush