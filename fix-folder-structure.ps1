# Fix nested AI-Terminals subfolder - move project to Hebrew root
$ErrorActionPreference = 'Stop'
$Desktop = [Environment]::GetFolderPath('Desktop')
$hebName = -join @(
    [char]0x05DE, [char]0x05D5, [char]0x05D3, [char]0x05DC, [char]0x05D9, [char]0x05DD,
    ' ', [char]0x05D8, [char]0x05E8, [char]0x05DE, [char]0x05E0, [char]0x05D9, [char]0x05DC, [char]0x05D9, [char]0x05DD
)
$Root = Join-Path $Desktop $hebName
$Nested = Join-Path $Root 'AI-Terminals'

Write-Host ''
Write-Host '  Fix folder structure' -ForegroundColor Cyan
Write-Host "  Root:   $Root"
Write-Host "  Nested: $Nested"
Write-Host ''

if (-not (Test-Path -LiteralPath $Nested)) {
    Write-Host '  No nested folder - nothing to fix.' -ForegroundColor Yellow
    exit 0
}

if (-not (Test-Path -LiteralPath (Join-Path $Nested 'hub.ps1'))) {
    Write-Host '  Nested folder is not the project - abort.' -ForegroundColor Red
    exit 1
}

if (Test-Path -LiteralPath (Join-Path $Root 'hub.ps1')) {
    Write-Host '  hub.ps1 already at root - removing empty nested only.' -ForegroundColor Green
} else {
    Write-Host '  Moving files to Hebrew root...' -ForegroundColor Yellow
    Get-ChildItem -LiteralPath $Nested -Force | ForEach-Object {
        $dest = Join-Path $Root $_.Name
        if (Test-Path -LiteralPath $dest) {
            Write-Host "  skip (exists): $($_.Name)" -ForegroundColor DarkYellow
        } else {
            Move-Item -LiteralPath $_.FullName -Destination $Root -Force
            Write-Host "  moved: $($_.Name)" -ForegroundColor Green
        }
    }
}

if ((Get-ChildItem -LiteralPath $Nested -Force -EA SilentlyContinue | Measure-Object).Count -eq 0) {
    Remove-Item -LiteralPath $Nested -Force -Recurse
    Write-Host '  Removed empty nested AI-Terminals folder' -ForegroundColor Green
} else {
    Write-Host '  Nested folder still has items - check manually' -ForegroundColor Yellow
}

$link = Join-Path $Desktop 'AI-Terminals'
if (Test-Path -LiteralPath $link) {
    $item = Get-Item -LiteralPath $link -Force
    if ($item.LinkType -eq 'Junction') { cmd /c "rmdir `"$link`"" 2>$null | Out-Null }
    else { Remove-Item -LiteralPath $link -Force -Recurse -EA SilentlyContinue }
}
cmd /c "mklink /J `"$link`" `"$Root`"" 2>$null | Out-Null
Write-Host '  Junction: Desktop\AI-Terminals -> Hebrew root' -ForegroundColor Green

Write-Host ''
Write-Host '  Done. Run setup-all.bat' -ForegroundColor Green
Write-Host ''