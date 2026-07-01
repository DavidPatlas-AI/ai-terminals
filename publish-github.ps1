# Publish AI-Terminals to GitHub (DavidPatlas-AI)
$ErrorActionPreference = 'Stop'
$Root = $PSScriptRoot
Set-Location -LiteralPath $Root

if (Test-Path -LiteralPath (Join-Path $Root 'ai-secrets.ps1')) {
    Write-Host 'NOTE: ai-secrets.ps1 is gitignored and will not be pushed.' -ForegroundColor Yellow
}

$repo = 'ai-terminals'

if (-not (Test-Path -LiteralPath (Join-Path $Root '.git'))) {
    git init
    git branch -M main
}

git add .
git status --short
git commit -m 'AI Terminals hub: dashboard, tokens, conversation status' 2>$null
if ($LASTEXITCODE -ne 0) {
    Write-Host 'Nothing new to commit.' -ForegroundColor DarkYellow
}

$gh = Get-Command gh -EA SilentlyContinue
if (-not $gh) {
    Write-Host 'Install GitHub CLI: winget install GitHub.cli'
    exit 1
}

$null = gh repo view "DavidPatlas-AI/$repo" 2>$null
$exists = ($LASTEXITCODE -eq 0)

if (-not $exists) {
    gh repo create "DavidPatlas-AI/$repo" --public --source=. --remote=origin --push --description "Windows hub for terminal AI with token dashboard"
} else {
    $null = git remote get-url origin 2>$null
    if ($LASTEXITCODE -ne 0) {
        git remote add origin "https://github.com/DavidPatlas-AI/$repo.git"
    }
    git push -u origin main
}

gh repo edit "DavidPatlas-AI/$repo" --homepage "https://github.com/DavidPatlas-AI/$repo"
Write-Host ''
Write-Host ('Published: https://github.com/DavidPatlas-AI/' + $repo) -ForegroundColor Green