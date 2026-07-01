# Create desktop shortcuts to AI work + data folders
$ErrorActionPreference = 'Continue'
. (Join-Path $PSScriptRoot '_work-folders.ps1')

$Desktop = [Environment]::GetFolderPath('Desktop')

Write-Host ''
Write-Host '  AI Work Folder Shortcuts' -ForegroundColor Cyan
Write-Host ''

$catalog = Get-AiWorkFolderCatalog
$made = 0
foreach ($item in $catalog) {
    $ok = New-FolderShortcut -Desktop $Desktop -LnkName $item.Shortcut -TargetDir $item.Path -Description $item.Tip
    if ($ok) {
        Write-Host "  $($item.Shortcut)  ->  $($item.Path)" -ForegroundColor Green
        $made++
    }
}

Write-Host ''
Write-Host "  Created $made shortcuts on desktop" -ForegroundColor Green
Write-Host ''
Write-Host '  Codex files are usually in:' -ForegroundColor Yellow
Write-Host '    Documents\Codex\DATE\...\work  or  ...\outputs' -ForegroundColor DarkGray
Write-Host ''
Write-Host '  Re-run anytime: setup-work-shortcuts.bat' -ForegroundColor DarkGray
Write-Host ''