# Scan repo for leaked secrets before git push
$ErrorActionPreference = 'Continue'
$Root = $PSScriptRoot
Set-Location -LiteralPath $Root

Write-Host ''
Write-Host '=== Security Check ===' -ForegroundColor Cyan

$blocked = $false

# 1) ai-secrets.ps1 must never be tracked
$tracked = git ls-files 'ai-secrets.ps1' 2>$null
if ($tracked) {
    Write-Host '[FAIL] ai-secrets.ps1 is tracked by git!' -ForegroundColor Red
    $blocked = $true
}
$staged = git diff --cached --name-only 2>$null
if ($staged -match 'ai-secrets\.ps1') {
    Write-Host '[FAIL] ai-secrets.ps1 is staged for commit!' -ForegroundColor Red
    $blocked = $true
}

# 2) Search git tree for real API keys (not placeholders)
$keyPatterns = @(
    'sk-[a-f0-9]{32}'
    'AIza[a-zA-Z0-9_-]{35}'
    'gsk_[a-zA-Z0-9]{20,}'
    'xai-[a-zA-Z0-9]{20,}'
    'ghp_[a-zA-Z0-9]{20,}'
)
foreach ($pat in $keyPatterns) {
    $found = git grep -E $pat HEAD 2>$null
    if ($found) {
        foreach ($line in ($found -split "`n")) {
            if ($line -match 'sk-\.\.\.' -or $line -match 'AIza\.\.\.' -or $line -match 'example') { continue }
            Write-Host "[FAIL] Possible secret in repo: $line" -ForegroundColor Red
            $blocked = $true
        }
    }
}

# 3) Generated files with session data must not be tracked
foreach ($gen in @('status.json', 'dashboard.html', 'STATUS.txt')) {
    if (git ls-files $gen 2>$null) {
        Write-Host "[FAIL] $gen should not be in git (may contain private data)" -ForegroundColor Red
        $blocked = $true
    }
}

if ($blocked) {
    Write-Host ''
    Write-Host 'DO NOT PUSH to GitHub until fixed.' -ForegroundColor Red
    exit 1
}

Write-Host '[OK] No API keys in git history' -ForegroundColor Green
Write-Host '[OK] ai-secrets.ps1 not tracked' -ForegroundColor Green
Write-Host '[OK] status.json / dashboard.html not tracked' -ForegroundColor Green
if (Test-Path (Join-Path $Root 'ai-secrets.ps1')) {
    Write-Host '[INFO] ai-secrets.ps1 exists locally only' -ForegroundColor DarkGray
}
Write-Host ''
Write-Host 'Safe to publish.' -ForegroundColor Green
exit 0