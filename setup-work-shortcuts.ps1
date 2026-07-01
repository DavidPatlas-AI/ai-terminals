# Create shortcuts INSIDE project folder (not on desktop)
$ErrorActionPreference = 'Continue'
. (Join-Path $PSScriptRoot '_work-folders.ps1')
. (Join-Path $PSScriptRoot '_paths.ps1')

$Folder = Get-AiTerminalsRoot
if (-not $Folder) { $Folder = $PSScriptRoot }
$Folder = (Get-Item -LiteralPath $Folder).FullName
$ShortcutsDir = Join-Path $Folder 'shortcuts'
New-Item -ItemType Directory -Path $ShortcutsDir -Force | Out-Null
$ShortcutsDir = (Get-Item -LiteralPath $ShortcutsDir).FullName

function New-BatShortcut {
    param([string]$Dir, [string]$LnkName, [string]$BatPath, [string]$Description = '')
    if (-not (Test-Path -LiteralPath $BatPath)) { return $false }
    $Wsh = New-Object -ComObject WScript.Shell
    $lnk = Join-Path $Dir $LnkName
    $sc = $Wsh.CreateShortcut($lnk)
    $sc.TargetPath = $env:ComSpec
    $sc.Arguments = "/c `"$BatPath`""
    $sc.WorkingDirectory = Split-Path -Parent $BatPath
    if ($Description) { $sc.Description = $Description }
    $sc.Save()
    return $true
}

Write-Host ''
Write-Host '  Shortcuts inside project folder' -ForegroundColor Cyan
Write-Host "  $ShortcutsDir"
Write-Host ''

$hubLinks = @(
    @{ Lnk = '01 Hub.lnk'; Bat = 'start.bat'; Tip = 'AI menu' }
    @{ Lnk = '02 Dashboard.lnk'; Bat = 'open-dashboard.bat'; Tip = 'Tokens dashboard' }
    @{ Lnk = '03 Start Here.lnk'; Bat = 'START-HERE.bat'; Tip = 'Main entry' }
    @{ Lnk = '04 Codex.lnk'; Bat = 'Codex.bat'; Tip = 'OpenAI Codex' }
    @{ Lnk = '05 Grok.lnk'; Bat = 'Grok.bat'; Tip = 'Grok' }
    @{ Lnk = '06 Claude.lnk'; Bat = 'Claude.bat'; Tip = 'Claude' }
    @{ Lnk = '07 NotebookLM.lnk'; Bat = 'NotebookLM.bat'; Tip = 'NotebookLM' }
    @{ Lnk = '08 Open Folders.lnk'; Bat = 'open-work-folders.bat'; Tip = 'Work folders menu' }
)
$made = 0
foreach ($h in $hubLinks) {
    $bat = Join-Path $Folder $h.Bat
    if (New-BatShortcut -Dir $ShortcutsDir -LnkName $h.Lnk -BatPath $bat -Description $h.Tip) {
        Write-Host "  $($h.Lnk)" -ForegroundColor Green
        $made++
    }
}

$catalog = Get-AiWorkFolderCatalog
foreach ($item in $catalog) {
    $name = $item.Shortcut -replace '\.lnk$', ''
    $lnkName = "$name.lnk"
    if (New-FolderShortcut -Desktop $ShortcutsDir -LnkName $lnkName -TargetDir $item.Path -Description $item.Tip) {
        Write-Host "  $lnkName  ->  $($item.Path)" -ForegroundColor DarkCyan
        $made++
    }
}

$readme = @(
    'Shortcuts - all inside the project folder'
    'Open this folder from: Desktop\AI-Terminals (junction)'
    'or: Desktop\מודלים טרמנילים'
    ''
    '01 Hub          - main menu'
    '02 Dashboard    - tokens + work folders'
    'AI Work - Codex - Documents\Codex (where Codex builds files)'
) -join "`r`n"
[System.IO.File]::WriteAllText((Join-Path $ShortcutsDir 'README.txt'), $readme, [System.Text.UTF8Encoding]::new($false))

Write-Host ''
Write-Host "  Created $made shortcuts in shortcuts\" -ForegroundColor Green
Write-Host '  Desktop stays clean - open folder, use shortcuts\' -ForegroundColor DarkGray
Write-Host ''