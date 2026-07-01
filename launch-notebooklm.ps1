. (Join-Path $PSScriptRoot '_common.ps1')
. (Join-Path $PSScriptRoot '_notebooklm.ps1')

$config = Join-Path $PSScriptRoot 'notebooklm-config.ps1'
if (Test-Path -LiteralPath $config) { . $config }

Set-WorkDirectory
Show-ChatBanner 'NotebookLM' 'Magenta' 'ask your notebook | /help for commands'

$nbId = $script:NOTEBOOKLM_NOTEBOOK_ID
$nbUrl = $script:NOTEBOOKLM_NOTEBOOK_URL
if (-not $nbId) {
    Write-Host '  Missing notebook ID in notebooklm-config.ps1' -ForegroundColor Red
    Read-Host '  Enter to close'
    return
}

if (-not (Test-NotebookLmInstalled)) {
    Write-Host '  notebooklm-py not installed!' -ForegroundColor Red
    Write-Host '  Run: install-notebooklm.bat' -ForegroundColor Yellow
    Read-Host '  Enter to close'
    return
}

$auth = Test-NotebookLmAuth
if (-not $auth.ok) {
    Write-Host "  $($auth.detail)" -ForegroundColor Yellow
    Write-Host ''
    Write-Host '  One-time setup: Google login in browser' -ForegroundColor Cyan
    $go = Read-Host '  Run login now? (Y/n)'
    if ($go -ne 'n' -and $go -ne 'N') {
        $py = Get-NotebookLmPython
        & $py -m notebooklm login
        $auth = Test-NotebookLmAuth
        if (-not $auth.ok) {
            Write-Host '  Login failed or incomplete.' -ForegroundColor Red
            Read-Host '  Enter to close'
            return
        }
    } else {
        Read-Host '  Enter to close'
        return
    }
}

$env:NOTEBOOKLM_NOTEBOOK = $nbId
$use = Invoke-NotebookLm -Args @('use', $nbId, '--force')
if (-not $use.ok) {
    Write-Host '  Could not set notebook context.' -ForegroundColor Red
    Write-Host "  $($use.out)" -ForegroundColor DarkGray
}

Write-Host "  notebook: $nbId" -ForegroundColor DarkCyan
if ($nbUrl) { Write-Host "  web:      $nbUrl" -ForegroundColor DarkGray }
Write-Host ''
Write-Host '  Type a question and press Enter. Commands: /help /history /new /sources /exit' -ForegroundColor DarkYellow
Write-Host ''

while ($true) {
    $q = Read-Host '  you'
    if (-not $q) { continue }
    $cmd = $q.Trim().ToLower()

    switch -Regex ($cmd) {
        '^/(exit|quit|q)$' { break }
        '^/help$' {
            Write-Host '  /history  - show chat history' -ForegroundColor DarkGray
            Write-Host '  /new      - start fresh conversation' -ForegroundColor DarkGray
            Write-Host '  /sources  - list notebook sources' -ForegroundColor DarkGray
            Write-Host '  /status   - show notebook context' -ForegroundColor DarkGray
            Write-Host '  /exit     - leave' -ForegroundColor DarkGray
            continue
        }
        '^/history$' {
            $r = Invoke-NotebookLm -Args @('history', '-l', '5')
            if ($r.out) { Write-Host $r.out }
            continue
        }
        '^/new$' {
            $r = Invoke-NotebookLm -Args @('ask', '--new', '-y', 'Continue from here')
            if ($r.out) { Write-Host $r.out }
            Write-Host '  New conversation started.' -ForegroundColor Green
            continue
        }
        '^/sources$' {
            $r = Invoke-NotebookLm -Args @('source', 'list', '--limit', '20')
            if ($r.out) { Write-Host $r.out }
            continue
        }
        '^/status$' {
            $r = Invoke-NotebookLm -Args @('status')
            if ($r.out) { Write-Host $r.out }
            continue
        }
    }

    Write-Host ''
    Write-Host '  thinking...' -ForegroundColor DarkGray
    $r = Invoke-NotebookLm -Args @('ask', $q)
    Write-Host ''
    if ($r.out) {
        foreach ($line in ($r.out -split "`n")) {
            Write-Host "  $line"
        }
    }
    if (-not $r.ok) {
        Write-Host '  (request failed - try notebooklm-login.bat if auth expired)' -ForegroundColor Red
    }
    Write-Host ''
}