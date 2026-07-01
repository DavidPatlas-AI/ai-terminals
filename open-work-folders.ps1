# Open AI work/data folders in Explorer (ASCII only)
$ErrorActionPreference = 'Continue'
. (Join-Path $PSScriptRoot '_work-folders.ps1')

function Show-Menu {
    Clear-Host
    Write-Host ''
    Write-Host '  AI Work Folders' -ForegroundColor Cyan
    Write-Host ''
    $i = 1
    $catalog = Get-AiWorkFolderCatalog
    $map = @{}
    foreach ($item in $catalog) {
        $tag = if ($item.Kind -eq 'data') { '[data]' } else { '[work]' }
        Write-Host "  $i  $tag $($item.Name)" -ForegroundColor White
        Write-Host "      $($item.Path)" -ForegroundColor DarkGray
        $map[[string]$i] = $item.Path
        $i++
    }
    Write-Host ''
    Write-Host '  S  Create desktop shortcuts' -ForegroundColor Yellow
    Write-Host '  0  Back' -ForegroundColor DarkGray
    Write-Host ''
    return $map
}

while ($true) {
    $map = Show-Menu
    $c = Read-Host '  Choose'
    if ($c -eq '0') { break }
    if ($c -eq 'S' -or $c -eq 's') {
        & (Join-Path $PSScriptRoot 'setup-work-shortcuts.ps1')
        Read-Host '  Enter'
        continue
    }
    if ($map.ContainsKey($c) -and $map[$c]) {
        Start-Process explorer.exe $map[$c]
    }
}