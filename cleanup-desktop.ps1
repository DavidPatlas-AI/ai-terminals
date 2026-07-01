# Desktop cleanup — project lives in folder only, no AI clutter on desktop
$ErrorActionPreference = 'Continue'
$Desktop = [Environment]::GetFolderPath('Desktop')

. (Join-Path $PSScriptRoot '_paths.ps1')
$Folder = Get-AiTerminalsRoot
if (-not $Folder -or -not (Test-Path (Join-Path $Folder 'hub.ps1'))) {
    $Folder = (Resolve-Path $PSScriptRoot).Path
}
$Projects = Join-Path $Desktop ([char]0x05E4 + [char]0x05E8 + [char]0x05D5 + [char]0x05D9 + [char]0x05E7 + [char]0x05D8 + [char]0x05D9 + [char]0x05DD)
$ContentDir = Join-Path $Projects 'torah-content'

function Ensure-Dir([string]$p) {
    if (-not (Test-Path -LiteralPath $p)) { New-Item -ItemType Directory -Path $p -Force | Out-Null }
}

Write-Host ''
Write-Host '  Desktop cleanup (folder-only mode)' -ForegroundColor Cyan
Write-Host ''

# 1. Junction AI-Terminals -> project folder (ASCII access, not clutter)
$link = Join-Path $Desktop 'AI-Terminals'
if (Test-Path -LiteralPath $link) {
    $item = Get-Item -LiteralPath $link -Force
    $bad = ($item.LinkType -ne 'Junction') -or ($item.Target -notcontains $Folder) -or ($item.Target -contains $link)
    if ($bad) {
        if ($item.LinkType -eq 'Junction') { cmd /c "rmdir `"$link`"" 2>$null | Out-Null }
        else { Remove-Item -LiteralPath $link -Force -Recurse -EA SilentlyContinue }
    }
}
if (-not (Test-Path -LiteralPath $link)) {
    cmd /c "mklink /J `"$link`" `"$Folder`"" 2>$null | Out-Null
    Write-Host 'Junction: AI-Terminals (opens project folder)' -ForegroundColor Green
}

# 2. Remove ALL AI shortcuts from desktop — everything is in the folder
$desktopKeep = @('YouTube.lnk')
Get-ChildItem -LiteralPath $Desktop -File -EA SilentlyContinue | ForEach-Object {
    $n = $_.Name
    if ($desktopKeep -contains $n) { return }
    $remove = $false
    if ($n -like 'AI*.lnk' -or $n -like 'AI *.lnk') { $remove = $true }
    if ($n -like 'AI - *' -or $n -like 'AI-*') { $remove = $true }
    if ($n -like '*.bat' -and $n -notlike 'YouTube*') { $remove = $true }
    if ($remove) {
        Remove-Item -LiteralPath $_.FullName -Force -EA SilentlyContinue
        Write-Host "Removed from desktop: $n" -ForegroundColor DarkYellow
    }
}

# 3. Move loose content files from user profile root
Ensure-Dir $ContentDir
Get-ChildItem -LiteralPath $env:USERPROFILE -File -EA SilentlyContinue | Where-Object {
    $_.Name -like 'parshat_*' -or $_.Name -like '*kora*' -or $_.Name -like '*torah*' -or
    $_.Name -match '^[\u05D0-\u05EA].*\.(md|html)$'
} | ForEach-Object {
    $dst = Join-Path $ContentDir $_.Name
    if (-not (Test-Path -LiteralPath $dst)) {
        Move-Item -LiteralPath $_.FullName -Destination $dst -Force
        Write-Host "Moved: $($_.Name)" -ForegroundColor Green
    } else {
        Remove-Item -LiteralPath $_.FullName -Force -EA SilentlyContinue
    }
}

# 4. Remove stray .lnk from project root (shortcuts live in shortcuts\)
Get-ChildItem -LiteralPath $Folder -Filter '*.lnk' -File -EA SilentlyContinue | ForEach-Object {
    Remove-Item -LiteralPath $_.FullName -Force
    Write-Host "Moved out of root: $($_.Name)" -ForegroundColor DarkGray
}

# 5. Shortcuts inside project folder
$wfScript = Join-Path $Folder 'setup-work-shortcuts.ps1'
if (Test-Path -LiteralPath $wfScript) {
    & $wfScript
}

# 6. Mark old duplicate folders
$oldFolders = @(
    (Join-Path $Desktop 'כלים\AI-Chats')
    (Join-Path $Desktop 'כלים\צאטים')
)
$redirect = @(
    'MOVED — use the new folder instead'
    ''
    'Active project:'
    '  Desktop\מודלים טרמנילים'
    '  or Desktop\AI-Terminals (same folder)'
    ''
    'Open shortcuts\ inside the folder'
) -join "`r`n"
foreach ($old in $oldFolders) {
    if (-not (Test-Path -LiteralPath $old)) { continue }
    $note = Join-Path $old 'MOVED-READ-ME.txt'
    [System.IO.File]::WriteAllText($note, $redirect, [System.Text.UTF8Encoding]::new($false))
}

Write-Host ''
Write-Host 'Done: desktop clean' -ForegroundColor Green
Write-Host "  Project: $Folder" -ForegroundColor Cyan
Write-Host '  Open: Desktop\AI-Terminals  or  מודלים טרמנילים' -ForegroundColor DarkGray
Write-Host '  Inside: shortcuts\  +  START-HERE.bat' -ForegroundColor DarkGray
Write-Host ''