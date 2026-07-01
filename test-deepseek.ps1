. (Join-Path $PSScriptRoot '_common.ps1')
Import-AiSecrets

Write-Host ''
Write-Host '=== DeepSeek Connection Test ===' -ForegroundColor Cyan
Write-Host ''

$key = $script:DEEPSEEK_API_KEY
if (-not $key -or $key -eq 'sk-...') {
    Write-Host '[FAIL] No API key in ai-secrets.ps1' -ForegroundColor Red
    Write-Host 'Run: setup-keys.bat' -ForegroundColor Yellow
    exit 1
}

$masked = $key.Substring(0, 7) + '...'
Write-Host "[OK]   Key loaded ($masked)" -ForegroundColor Green

try {
    $models = Invoke-RestMethod -Uri 'https://api.deepseek.com/models' `
        -Headers @{ Authorization = "Bearer $key" } -TimeoutSec 30
    Write-Host '[OK]   API key valid' -ForegroundColor Green
    Write-Host "       Models: $($models.data.id -join ', ')" -ForegroundColor DarkGray
} catch {
    Write-Host '[FAIL] API key rejected' -ForegroundColor Red
    Write-Host $_.Exception.Message -ForegroundColor Yellow
    exit 1
}

$body = @{
    model = 'deepseek-v4-flash'
    messages = @(@{ role = 'user'; content = 'hi' })
    max_tokens = 5
} | ConvertTo-Json -Depth 5

try {
    $null = Invoke-RestMethod -Uri 'https://api.deepseek.com/chat/completions' `
        -Method Post `
        -Headers @{ Authorization = "Bearer $key"; 'Content-Type' = 'application/json' } `
        -Body $body -TimeoutSec 60
    Write-Host '[OK]   Chat API works - balance OK' -ForegroundColor Green
    Write-Host ''
    Write-Host 'Ready! Open: Desktop\🔵 DeepSeek.bat' -ForegroundColor Green
} catch {
    $msg = $_.Exception.Message
    if ($msg -match '402' -or $_.ErrorDetails.Message -match '402|Insufficient') {
        Write-Host '[FAIL] Insufficient Balance (402)' -ForegroundColor Red
        Write-Host ''
        Write-Host 'Your key is valid but the account has no credits.' -ForegroundColor Yellow
        Write-Host 'Top up at: https://platform.deepseek.com/top_up' -ForegroundColor Yellow
        Write-Host 'Or: https://platform.deepseek.com/usage' -ForegroundColor Yellow
        exit 2
    }
    Write-Host "[FAIL] Chat API: $msg" -ForegroundColor Red
    exit 1
}
Write-Host ''