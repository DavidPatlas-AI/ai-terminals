# Publish AI-Terminals to GitHub — ONLY after security-check passes
$ErrorActionPreference = 'Stop'
$Root = $PSScriptRoot
Set-Location -LiteralPath $Root

Write-Host ''
Write-Host '========================================' -ForegroundColor Cyan
Write-Host '  פרסום ל-GitHub (ציבורי)' -ForegroundColor Cyan
Write-Host '========================================' -ForegroundColor Cyan
Write-Host ''
Write-Host 'Repo: DavidPatlas-AI/ai-terminals' -ForegroundColor DarkGray
Write-Host ''

Write-Host 'Step 1/3: Security check (required)' -ForegroundColor Yellow
& (Join-Path $Root 'security-check.ps1')
if ($LASTEXITCODE -ne 0) {
    Write-Host ''
    Write-Host 'Publish cancelled — fix security issues first.' -ForegroundColor Red
    Write-Host 'Never commit: ai-secrets.ps1, API keys' -ForegroundColor Yellow
    exit 1
}

$repo = 'ai-terminals'

if (-not (Test-Path -LiteralPath (Join-Path $Root '.git'))) {
    git init
    git branch -M main
}

Write-Host ''
Write-Host 'Step 2/3: Commit changes' -ForegroundColor Yellow
git add .
git status --short
$msg = 'AI Terminals update ' + (Get-Date -Format 'yyyy-MM-dd')
git commit -m $msg 2>$null
if ($LASTEXITCODE -ne 0) {
    Write-Host 'Nothing new to commit (already up to date).' -ForegroundColor DarkYellow
}

& (Join-Path $Root 'security-check.ps1')
if ($LASTEXITCODE -ne 0) {
    Write-Host 'Publish cancelled after commit review.' -ForegroundColor Red
    exit 1
}

$gh = Get-Command gh -EA SilentlyContinue
if (-not $gh) {
    Write-Host ''
    Write-Host 'GitHub CLI missing. Install:' -ForegroundColor Red
    Write-Host '  winget install GitHub.cli' -ForegroundColor Yellow
    Write-Host 'Then: gh auth login' -ForegroundColor Yellow
    exit 1
}

$null = gh repo view "DavidPatlas-AI/$repo" 2>$null
$exists = ($LASTEXITCODE -eq 0)

Write-Host ''
Write-Host 'Step 3/3: Push to GitHub' -ForegroundColor Yellow
if (-not $exists) {
    gh repo create "DavidPatlas-AI/$repo" --public --source=. --remote=origin --push --description "Windows hub for terminal AI with token dashboard"
} else {
    $null = git remote get-url origin 2>$null
    if ($LASTEXITCODE -ne 0) {
        git remote add origin "https://github.com/DavidPatlas-AI/$repo.git"
    }
    git push -u origin main
}

gh repo edit "DavidPatlas-AI/$repo" --homepage "https://github.com/DavidPatlas-AI/$repo" 2>$null
$url = 'https://github.com/DavidPatlas-AI/' + $repo
Write-Host ''
Write-Host '========================================' -ForegroundColor Green
Write-Host ('  Published: ' + $url) -ForegroundColor Green
Write-Host '========================================' -ForegroundColor Green
Write-Host ''
Write-Host 'What is public: scripts, README, dashboard template' -ForegroundColor DarkGray
Write-Host 'What stays local: ai-secrets.ps1, your API keys' -ForegroundColor DarkGray
Write-Host ''