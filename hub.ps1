$ErrorActionPreference = 'Continue'
$ChatRoot = $PSScriptRoot
. (Join-Path $ChatRoot '_common.ps1')
. (Join-Path $ChatRoot '_models.ps1')

function Get-QuickMap {
    $map = @{}
    if (Test-Path (Join-Path $ChatRoot 'status.json')) {
        try {
            $j = Get-Content (Join-Path $ChatRoot 'status.json') -Raw | ConvertFrom-Json
            foreach ($m in $j.models) { $map[$m.id] = $m.status }
        } catch {}
    }
    if ($map.Count -eq 0 -and (Test-Path (Join-Path $ChatRoot 'STATUS.txt'))) {
        $st = Get-Content (Join-Path $ChatRoot 'STATUS.txt') -Raw
        if ($st -match '\[OK\].*Grok') { $map.grok = 'ok' }
        if ($st -match '\[WAIT\].*Claude') { $map.claude = 'wait' }
        if ($st -match '\[WAIT\].*Codex') { $map.codex = 'wait' }
        if ($st -match '\[FAIL\].*DeepSeek') { $map.deepseek = 'fail' }
    }
    foreach ($m in $script:AiModels) {
        if (-not $map.ContainsKey($m.Id)) {
            if ($m.Id -eq 'grok' -and -not (Get-Command grok -EA SilentlyContinue)) { $map[$m.Id] = 'fail' }
            elseif ($m.Id -eq 'gemini' -and -not (Get-Command gemini -EA SilentlyContinue)) { $map[$m.Id] = 'fail' }
            else { $map[$m.Id] = '?' }
        }
    }
    return $map
}

function Write-StatusTag {
    param([hashtable]$Status, [string]$ModelId)
    $tag = @{
        ok = @{ T = '[OK]  '; C = 'Green' }
        wait = @{ T = '[WAIT]'; C = 'Yellow' }
        fail = @{ T = '[X]   '; C = 'Red' }
        key = @{ T = '[KEY] '; C = 'Magenta' }
        '?' = @{ T = '[?]   '; C = 'DarkGray' }
    }
    $s = if ($Status.ContainsKey($ModelId)) { $Status[$ModelId] } else { '?' }
    $x = $tag[$s]
    if (-not $x) { $x = $tag['?'] }
    Write-Host $x.T -NoNewline -ForegroundColor $x.C
}

function Show-Menu {
    Clear-Host
    $st = Get-QuickMap

    Write-Host ''
    Write-Host '  =====================================' -ForegroundColor Cyan
    Write-Host '          AI Terminals - AI Hub' -ForegroundColor Cyan
    Write-Host '  =====================================' -ForegroundColor Cyan
    Write-Host ''
    Write-Host '  '; Write-StatusTag $st 'grok'; Write-Host ' 1  Grok       (xAI)' -ForegroundColor White
    Write-Host '  '; Write-StatusTag $st 'claude'; Write-Host ' 2  Claude     (Anthropic)' -ForegroundColor White
    Write-Host '  '; Write-StatusTag $st 'gemini'; Write-Host ' 3  Gemini     (Google)' -ForegroundColor White
    Write-Host '  '; Write-StatusTag $st 'deepseek'; Write-Host ' 4  DeepSeek   (API via Claude)' -ForegroundColor White
    Write-Host '  '; Write-StatusTag $st 'codex'; Write-Host ' 5  Codex      (OpenAI)' -ForegroundColor White
    Write-Host '  '; Write-StatusTag $st 'notebooklm'; Write-Host ' N  NotebookLM (Google)' -ForegroundColor Magenta
    Write-Host ''
    Write-Host '  6  Open all main tools (Windows Terminal)' -ForegroundColor Magenta
    Write-Host '  7  Open dashboard' -ForegroundColor Cyan
    Write-Host '  R  Refresh conversations only' -ForegroundColor DarkCyan
    Write-Host ''
    Write-Host '  --- Free / local options ---' -ForegroundColor Green
    Write-Host '  F  Try free setup' -ForegroundColor Green
    Write-Host '  C  Crush (Groq/Gemini/OpenRouter)' -ForegroundColor White
    Write-Host '  O  Ollama (local, no API key)' -ForegroundColor White
    Write-Host '  D  OpenCode' -ForegroundColor White
    Write-Host '  I  Install free options' -ForegroundColor DarkGray
    Write-Host ''
    Write-Host '  8  Full health check' -ForegroundColor DarkCyan
    Write-Host '  W  Update work-folder shortcuts' -ForegroundColor Cyan
    Write-Host '  P  Open work folders' -ForegroundColor Cyan
    Write-Host '  E  Fix Hebrew RTL in Cursor' -ForegroundColor Yellow
    Write-Host '  9  Open docs' -ForegroundColor DarkGray
    Write-Host '  0  Exit' -ForegroundColor DarkGray
    Write-Host ''
}

function Start-Model {
    param([string]$ModelId)
    & (Join-Path $ChatRoot 'launch-ai.ps1') -Model $ModelId
}

function Start-BatchFile {
    param(
        [string]$FileName,
        [switch]$Wait
    )
    $file = Join-Path $ChatRoot $FileName
    $args = @('/c', "`"$file`"")
    if ($Wait) {
        Start-Process -FilePath 'cmd.exe' -ArgumentList $args -WorkingDirectory $ChatRoot -Wait
    } else {
        Start-Process -FilePath 'cmd.exe' -ArgumentList $args -WorkingDirectory $ChatRoot
    }
}

while ($true) {
    Show-Menu
    $c = Read-Host '  Choose'
    switch ($c) {
        '1' { Start-Model 'grok'; break }
        '2' { Start-Model 'claude'; break }
        '3' { Start-Model 'gemini'; break }
        '4' { Start-Model 'deepseek'; break }
        '5' { Start-Model 'codex'; break }
        'N' { Start-Model 'notebooklm'; break }
        'n' { Start-Model 'notebooklm'; break }
        '6' { Start-BatchFile 'open-all.bat'; Start-Sleep 1; break }
        '7' { Start-Process -FilePath (Join-Path $ChatRoot 'open-dashboard.bat') -WorkingDirectory $ChatRoot; Start-Sleep 1; break }
        'R' { & (Join-Path $ChatRoot 'refresh-conversations.ps1'); Start-Process (Join-Path $ChatRoot 'open-dashboard.bat'); Read-Host '  Enter'; break }
        'r' { & (Join-Path $ChatRoot 'refresh-conversations.ps1'); Start-Process (Join-Path $ChatRoot 'open-dashboard.bat'); Read-Host '  Enter'; break }
        'F' { Start-BatchFile 'try-free.bat'; Start-Sleep 1; break }
        'f' { Start-BatchFile 'try-free.bat'; Start-Sleep 1; break }
        'C' { Start-Model 'crush'; break }
        'c' { Start-Model 'crush'; break }
        'O' { Start-Model 'ollama'; break }
        'o' { Start-Model 'ollama'; break }
        'D' { Start-Model 'opencode'; break }
        'd' { Start-Model 'opencode'; break }
        'P' { & (Join-Path $ChatRoot 'open-work-folders.ps1'); break }
        'p' { & (Join-Path $ChatRoot 'open-work-folders.ps1'); break }
        'I' { Start-BatchFile 'install-free.bat'; Start-Sleep 1; break }
        'i' { Start-BatchFile 'install-free.bat'; Start-Sleep 1; break }
        '8' { & (Join-Path $ChatRoot 'check-all.ps1'); Read-Host '  Enter'; break }
        'W' { Start-BatchFile 'setup-work-shortcuts.bat' -Wait; break }
        'w' { Start-BatchFile 'setup-work-shortcuts.bat' -Wait; break }
        'E' { Start-BatchFile 'fix-hebrew-chat.bat' -Wait; break }
        'e' { Start-BatchFile 'fix-hebrew-chat.bat' -Wait; break }
        '9' {
            notepad (Join-Path $ChatRoot 'MAP.txt')
            notepad (Join-Path $ChatRoot 'docs\UNIVERSAL-LAUNCHER.md')
            break
        }
        '0' { exit 0 }
        default { Write-Host '  Unknown choice' -ForegroundColor Yellow; Start-Sleep 1 }
    }
}
