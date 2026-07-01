# Interactive API key setup for AI terminal models
$ErrorActionPreference = 'Stop'
$ChatRoot = $PSScriptRoot
$secretsPath = Join-Path $ChatRoot 'ai-secrets.ps1'
$examplePath = Join-Path $ChatRoot 'ai-secrets.example.ps1'

function Read-KeyFromFile([string]$Name) {
    if (-not (Test-Path -LiteralPath $secretsPath)) { return '' }
    $content = Get-Content -LiteralPath $secretsPath -Raw
    if ($content -match "\`$script:$Name\s*=\s*`"([^`"]+)`"") {
        $v = $Matches[1]
        if ($v -and $v -notmatch '\.\.\.') { return $v }
    }
    return ''
}

if (-not (Test-Path -LiteralPath $secretsPath)) {
    Copy-Item -LiteralPath $examplePath -Destination $secretsPath -Force
}

$deepseek = Read-KeyFromFile 'DEEPSEEK_API_KEY'
$gemini = Read-KeyFromFile 'GEMINI_API_KEY'
$groq = Read-KeyFromFile 'GROQ_API_KEY'

Write-Host ''
Write-Host '=== API Keys Setup (FREE keys first!) ===' -ForegroundColor Cyan
Write-Host ''

Write-Host '--- Gemini (FREE - recommended) ---' -ForegroundColor Green
Write-Host 'https://aistudio.google.com/apikey'
if ($gemini) { Write-Host "Current: $($gemini.Substring(0,7))..." -ForegroundColor DarkGray }
$inGm = Read-Host 'Gemini key (AIza...) Enter=skip'
if ($inGm) { $gemini = $inGm }

Write-Host ''
Write-Host '--- Groq (FREE - fast) ---' -ForegroundColor Green
Write-Host 'https://console.groq.com/keys'
if ($groq) { Write-Host "Current: $($groq.Substring(0,7))..." -ForegroundColor DarkGray }
$inGq = Read-Host 'Groq key (gsk_...) Enter=skip'
if ($inGq) { $groq = $inGq }

Write-Host ''
Write-Host '--- DeepSeek (paid balance) ---' -ForegroundColor Yellow
Write-Host 'https://platform.deepseek.com/api_keys'
if ($deepseek) { Write-Host "Current: $($deepseek.Substring(0,7))..." -ForegroundColor DarkGray }
$inDs = Read-Host 'DeepSeek key (sk-...) Enter=skip'
if ($inDs) { $deepseek = $inDs }

$lines = @(
    '# AI API keys - do not share this file',
    '',
    '# Gemini FREE - https://aistudio.google.com/apikey',
    $(if ($gemini) { "`$script:GEMINI_API_KEY = `"$gemini`"" } else { '# $script:GEMINI_API_KEY = "AIza..."' }),
    '',
    '# Groq FREE - https://console.groq.com/keys',
    $(if ($groq) { "`$script:GROQ_API_KEY = `"$groq`"" } else { '# $script:GROQ_API_KEY = "gsk_..."' }),
    '',
    '# DeepSeek - https://platform.deepseek.com/api_keys',
    $(if ($deepseek) { "`$script:DEEPSEEK_API_KEY = `"$deepseek`"" } else { '# $script:DEEPSEEK_API_KEY = "sk-..."' }),
    '',
    '# Optional: $script:OPENROUTER_API_KEY = "sk-or-..."',
    '# Optional: $script:XAI_API_KEY = "xai-..."'
)
[System.IO.File]::WriteAllText($secretsPath, ($lines -join "`r`n"), [System.Text.UTF8Encoding]::new($false))
Write-Host ''
Write-Host 'Saved: ai-secrets.ps1' -ForegroundColor Green
Write-Host 'Next: AI-Gemini.bat or AI-Crush.bat' -ForegroundColor Cyan
Write-Host ''