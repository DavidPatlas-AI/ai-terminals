. (Join-Path $PSScriptRoot '_common.ps1')
Import-AiSecrets
if (Test-Path -LiteralPath $WorkDir) { Set-Location $WorkDir } else { Set-Location $HomeDir }
Show-ChatBanner 'DeepSeek via Claude Code' 'Blue'

$key = $script:DEEPSEEK_API_KEY
if (-not $key -or $key -eq 'sk-...') {
    Write-Host 'Missing DeepSeek API key!' -ForegroundColor Red
    Write-Host ''
    Write-Host '1. Get key: https://platform.deepseek.com/api_keys' -ForegroundColor Yellow
    Write-Host '2. Copy ai-secrets.example.ps1 to ai-secrets.ps1' -ForegroundColor Yellow
    Write-Host '3. Paste key into $script:DEEPSEEK_API_KEY' -ForegroundColor Yellow
    Write-Host ''
    Read-Host 'Press Enter to close'
    return
}

Set-DeepSeekEnv $key | Out-Null
Remove-Item Env:ANTHROPIC_API_KEY -ErrorAction SilentlyContinue

# Quick balance check before opening Claude
$testBody = '{"model":"deepseek-v4-flash","messages":[{"role":"user","content":"hi"}],"max_tokens":3}'
try {
    $null = Invoke-RestMethod -Uri 'https://api.deepseek.com/chat/completions' `
        -Method Post `
        -Headers @{ Authorization = "Bearer $key"; 'Content-Type' = 'application/json' } `
        -Body $testBody -TimeoutSec 30
    Write-Host 'DeepSeek API connected' -ForegroundColor Green
} catch {
    if ($_.Exception.Message -match '402' -or $_.ErrorDetails.Message -match '402|Insufficient') {
        Write-Host ''
        Write-Host 'DeepSeek: Insufficient Balance' -ForegroundColor Red
        Write-Host 'Key is valid but account has no credits.' -ForegroundColor Yellow
        Write-Host 'Top up: https://platform.deepseek.com/top_up' -ForegroundColor Yellow
        Write-Host ''
        Read-Host 'Press Enter to close'
        return
    }
    Write-Host "DeepSeek API warning: $($_.Exception.Message)" -ForegroundColor Yellow
}

Write-Host 'Model: deepseek-v4-pro' -ForegroundColor DarkGray
Write-Host ''

$claudeHint = 'npm install -g @anthropic-ai/claude-code'
if (-not (Require-Command 'claude' $claudeHint)) { return }
claude