# Security gate - run BEFORE any git push
$ErrorActionPreference = 'Continue'
$Root = $PSScriptRoot
Set-Location -LiteralPath $Root

Write-Host ''
Write-Host '=== Security Check ===' -ForegroundColor Cyan

$blocked = $false
$secretFiles = @('ai-secrets.ps1', 'status.json', 'dashboard.html', 'STATUS.txt', '.env')

foreach ($f in $secretFiles) {
    if (git ls-files -- $f 2>$null) {
        Write-Host "[FAIL] $f is tracked by git" -ForegroundColor Red
        $blocked = $true
    }
    $staged = git diff --cached --name-only 2>$null
    if ($staged -match [regex]::Escape($f)) {
        Write-Host "[FAIL] $f is staged for commit" -ForegroundColor Red
        $blocked = $true
    }
}

$hist = git log --all --oneline -- ai-secrets.ps1 2>$null
if ($hist) {
    Write-Host '[FAIL] ai-secrets.ps1 was committed in the past' -ForegroundColor Red
    $blocked = $true
}

$keyPatterns = @(
    @{ Name = 'DeepSeek'; Pat = 'sk-[a-f0-9]{20,}' }
    @{ Name = 'Google'; Pat = 'AIza[a-zA-Z0-9_-]{30,}' }
    @{ Name = 'Groq'; Pat = 'gsk_[a-zA-Z0-9]{20,}' }
    @{ Name = 'xAI'; Pat = 'xai-[a-zA-Z0-9]{20,}' }
    @{ Name = 'GitHub'; Pat = 'ghp_[a-zA-Z0-9]{20,}' }
)

foreach ($kp in $keyPatterns) {
    $found = git grep -E $kp.Pat HEAD 2>$null
    if ($found) {
        foreach ($line in ($found -split "`n")) {
            if ($line -match '\.\.\.' -or $line -match 'example') { continue }
            Write-Host "[FAIL] $($kp.Name) key in repo" -ForegroundColor Red
            $blocked = $true
        }
    }
}

$toAdd = git ls-files --others --exclude-standard 2>$null
$modified = git diff --name-only 2>$null
$checkList = @()
if ($toAdd) { $checkList += ($toAdd -split "`n") }
if ($modified) { $checkList += ($modified -split "`n") }
foreach ($rel in ($checkList | Where-Object { $_ -and $_ -notmatch '[\x00-\x1f]'})) {
    if ($secretFiles -contains $rel) {
        Write-Host "[FAIL] Pending secret file: $rel" -ForegroundColor Red
        $blocked = $true
        continue
    }
    $path = Join-Path $Root $rel
    try { $exists = Test-Path -LiteralPath $path } catch { continue }
    if (-not $exists) { continue }
    try {
        $text = Get-Content -LiteralPath $path -Raw -Encoding UTF8 -EA SilentlyContinue
        if (-not $text) { continue }
        foreach ($kp in $keyPatterns) {
            if ($text -match $kp.Pat -and $rel -notmatch 'example') {
                if ($text -match '\.\.\.') { continue }
                Write-Host "[FAIL] $($kp.Name) key in pending file: $rel" -ForegroundColor Red
                $blocked = $true
            }
        }
    } catch {}
}

if ($blocked) {
    Write-Host ''
    Write-Host 'BLOCKED - do NOT push to GitHub' -ForegroundColor Red
    exit 1
}

Write-Host '[OK] No API keys in git' -ForegroundColor Green
Write-Host '[OK] ai-secrets.ps1 not tracked' -ForegroundColor Green
Write-Host '[OK] Generated files not tracked' -ForegroundColor Green
if (Test-Path (Join-Path $Root 'ai-secrets.ps1')) {
    Write-Host '[INFO] ai-secrets.ps1 is local only' -ForegroundColor DarkGray
}
Write-Host ''
Write-Host 'Safe to publish' -ForegroundColor Green
exit 0