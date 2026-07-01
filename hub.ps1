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

function Show-Menu {
    Clear-Host
    $st = Get-QuickMap
    $tag = @{
        'ok' = @{ T = '[OK]  '; C = 'Green' }
        'wait' = @{ T = '[WAIT]'; C = 'Yellow' }
        'fail' = @{ T = '[X]  '; C = 'Red' }
        'key' = @{ T = '[KEY]'; C = 'Magenta' }
        '?' = @{ T = '[?]  '; C = 'DarkGray' }
    }
    function L($k) {
        $s = if ($st.ContainsKey($k)) { $st[$k] } else { '?' }
        $x = $tag[$s]; if (-not $x) { $x = $tag['?'] }
        Write-Host $x.T -NoNewline -ForegroundColor $x.C
    }

    Write-Host ''
    Write-Host '  =====================================' -ForegroundColor Cyan
    Write-Host '       מודלים טרמינלים — AI Hub' -ForegroundColor Cyan
    Write-Host '  =====================================' -ForegroundColor Cyan
    Write-Host ''
    Write-Host '  '; L 'grok'; Write-Host ' 1  Grok       (xAI)' -ForegroundColor White
    Write-Host '  '; L 'claude'; Write-Host ' 2  Claude     (Anthropic)' -ForegroundColor White
    Write-Host '  '; L 'gemini'; Write-Host ' 3  Gemini     (Google)' -ForegroundColor White
    Write-Host '  '; L 'deepseek'; Write-Host ' 4  DeepSeek   (API)' -ForegroundColor White
    Write-Host '  '; L 'codex'; Write-Host ' 5  Codex      (OpenAI)' -ForegroundColor White
    Write-Host '  '; L 'notebooklm'; Write-Host ' N  NotebookLM (Google - your notebook)' -ForegroundColor Magenta
    Write-Host ''
    Write-Host '  6  כל 5 ביחד (Windows Terminal)' -ForegroundColor Magenta
    Write-Host '  7  לוח AI — טוקנים ושיחות (דשבורד)' -ForegroundColor Cyan
    Write-Host '  R  רענון שיחות בלבד (מהיר)' -ForegroundColor DarkCyan
    Write-Host ''
    Write-Host '  --- חינמי ---' -ForegroundColor Green
    Write-Host '  F  try-free.bat (הכי מהיר!)' -ForegroundColor Green
    Write-Host '  C  Crush (Groq/Gemini)' -ForegroundColor White
    Write-Host '  O  Ollama (100% חינם מקומי)' -ForegroundColor White
    Write-Host '  I  install-free.bat' -ForegroundColor DarkGray
    Write-Host ''
    Write-Host '  8  בדיקת חיבור מלאה' -ForegroundColor DarkCyan
    Write-Host '  9  הוראות' -ForegroundColor DarkGray
    Write-Host '  0  יציאה' -ForegroundColor DarkGray
    Write-Host ''
}

while ($true) {
    Show-Menu
    $c = Read-Host '  בחר מספר'
    switch ($c) {
        '1' { & (Join-Path $ChatRoot 'launch-grok.ps1'); break }
        '2' { & (Join-Path $ChatRoot 'launch-claude.ps1'); break }
        '3' { & (Join-Path $ChatRoot 'launch-gemini.ps1'); break }
        '4' { & (Join-Path $ChatRoot 'launch-deepseek.ps1'); break }
        '5' { & (Join-Path $ChatRoot 'launch-codex.ps1'); break }
        'N' { & (Join-Path $ChatRoot 'launch-notebooklm.ps1'); break }
        'n' { & (Join-Path $ChatRoot 'launch-notebooklm.ps1'); break }
        '6' { Start-Process -FilePath 'cmd.exe' -ArgumentList '/c', (Join-Path $ChatRoot 'open-all.bat') -WorkingDirectory $ChatRoot; Start-Sleep 1; break }
        '7' { Start-Process -FilePath (Join-Path $ChatRoot 'open-dashboard.bat') -WorkingDirectory $ChatRoot; Start-Sleep 1; break }
        'R' { & (Join-Path $ChatRoot 'refresh-conversations.ps1'); Start-Process (Join-Path $ChatRoot 'open-dashboard.bat'); Read-Host '  Enter'; break }
        'r' { & (Join-Path $ChatRoot 'refresh-conversations.ps1'); Start-Process (Join-Path $ChatRoot 'open-dashboard.bat'); Read-Host '  Enter'; break }
        'F' { Start-Process -FilePath (Join-Path $ChatRoot 'try-free.bat') -WorkingDirectory $ChatRoot; Start-Sleep 1; break }
        'f' { Start-Process -FilePath (Join-Path $ChatRoot 'try-free.bat') -WorkingDirectory $ChatRoot; Start-Sleep 1; break }
        'C' { & (Join-Path $ChatRoot 'launch-crush.ps1'); break }
        'c' { & (Join-Path $ChatRoot 'launch-crush.ps1'); break }
        'O' { & (Join-Path $ChatRoot 'launch-ollama.ps1'); break }
        'o' { & (Join-Path $ChatRoot 'launch-ollama.ps1'); break }
        'I' { Start-Process -FilePath (Join-Path $ChatRoot 'install-free.bat') -WorkingDirectory $ChatRoot; Start-Sleep 1; break }
        'i' { Start-Process -FilePath (Join-Path $ChatRoot 'install-free.bat') -WorkingDirectory $ChatRoot; Start-Sleep 1; break }
        '8' { & (Join-Path $ChatRoot 'check-all.ps1'); Read-Host '  Enter'; break }
        '9' { notepad (Join-Path $ChatRoot 'הוראות.txt'); break }
        '0' { exit 0 }
        default { Write-Host '  ?' -ForegroundColor Yellow; Start-Sleep 1 }
    }
}