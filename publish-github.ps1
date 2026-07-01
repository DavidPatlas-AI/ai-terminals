# Publish AI-Terminals to GitHub (DavidPatlas-AI)
$ErrorActionPreference = 'Stop'
$Root = $PSScriptRoot
Set-Location -LiteralPath $Root

if (Test-Path -LiteralPath (Join-Path $Root 'ai-secrets.ps1')) {
    Write-Host 'WARNING: ai-secrets.ps1 exists — it is gitignored, will NOT be pushed.' -ForegroundColor Yellow
}

$repo = 'ai-terminals'
$remote = "https://github.com/DavidPatlas-AI/$repo.git"

if (-not (Test-Path -LiteralPath (Join-Path $Root '.git'))) {
    git init
    git branch -M main
}

git add .
git status --short
$msg = "AI Terminals hub — dashboard, tokens, conversation status, Hebrew UX"
git commit -m $msg 2>$null
if ($LASTEXITCODE -ne 0) {
    Write-Host 'Nothing new to commit (or commit failed).' -ForegroundColor DarkYellow
}

$gh = Get-Command gh -EA SilentlyContinue
if (-not $gh) {
    Write-Host 'Install GitHub CLI: winget install GitHub.cli' -ForegroundColor Yellow
    Write-Host "Then: gh repo create DavidPatlas-AI/$repo --public --source=. --push"
    exit 1
}

$exists = gh repo view "DavidPatlas-AI/$repo" 2>$null
if (-not $exists) {
    gh repo create "DavidPatlas-AI/$repo" --public --source=. --remote=origin --push --description "Windows hub for terminal AI — token dashboard, conversation status, Grok/Claude/Gemini"
} else {
    if (-not (git remote get-url origin 2>$null)) { git remote add origin $remote }
    git push -u origin main
}

gh repo edit "DavidPatlas-AI/$repo" --homepage "https://github.com/DavidPatlas-AI/$repo#readme"
Write-Host ''
Write-Host "Published: https://github.com/DavidPatlas-AI/$repo" -ForegroundColor Green