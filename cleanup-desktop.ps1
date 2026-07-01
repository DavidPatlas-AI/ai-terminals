# Desktop cleanup — keeps only AI Hub + AI Dashboard shortcuts for AI tools
$ErrorActionPreference = 'Continue'
$Folder = (Resolve-Path $PSScriptRoot).Path
$Desktop = [Environment]::GetFolderPath('Desktop')
$Projects = Join-Path $Desktop ([char]0x05E4 + [char]0x05E8 + [char]0x05D5 + [char]0x05D9 + [char]0x05E7 + [char]0x05D8 + [char]0x05D9 + [char]0x05DD)
$ContentDir = Join-Path $Projects 'torah-content'

function Ensure-Dir([string]$p) {
    if (-not (Test-Path -LiteralPath $p)) { New-Item -ItemType Directory -Path $p -Force | Out-Null }
}

Write-Host ''
Write-Host '  Desktop cleanup' -ForegroundColor Cyan
Write-Host ''

# 1. Junction AI-Terminals -> this folder
$link = Join-Path $Desktop 'AI-Terminals'
if (Test-Path -LiteralPath $link) {
    $item = Get-Item -LiteralPath $link -Force
    if ($item.LinkType -ne 'Junction' -or $item.Target -notcontains $Folder) {
        Remove-Item -LiteralPath $link -Force -Recurse -EA SilentlyContinue
    }
}
if (-not (Test-Path -LiteralPath $link)) {
    cmd /c "mklink /J `"$link`" `"$Folder`"" 2>$null | Out-Null
    Write-Host 'Junction: AI-Terminals' -ForegroundColor Green
}

# 2. Remove stray AI shortcuts / bats from desktop root
$keep = @('AI Hub.lnk', 'AI Dashboard.lnk', 'YouTube.lnk')
Get-ChildItem -LiteralPath $Desktop -File -EA SilentlyContinue | ForEach-Object {
    $n = $_.Name
    if ($keep -contains $n) { return }
    $remove = $false
    if ($n -like 'AI - *' -or $n -like 'AI-*' -or $n -match '^AI\.lnk$') { $remove = $true }
    if ($n -like '*.bat' -and $n -notlike 'YouTube*') {
        if ($n -match 'open|hub|dashboard|start|check|setup|try|install|refresh|publish') { }
        elseif ($_.Extension -eq '.bat') { $remove = $true }
    }
    if ($remove) {
        Remove-Item -LiteralPath $_.FullName -Force -EA SilentlyContinue
        Write-Host "Removed: $n" -ForegroundColor DarkYellow
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

# 4. Remove .lnk clutter inside project folder
Get-ChildItem -LiteralPath $Folder -Filter '*.lnk' -File -EA SilentlyContinue | ForEach-Object {
    Remove-Item -LiteralPath $_.FullName -Force
}

# 5. Create exactly 2 desktop shortcuts
$Wsh = New-Object -ComObject WScript.Shell
$Cmd = $env:ComSpec
$linkRoot = if (Test-Path -LiteralPath $link) { $link } else { $Folder }
foreach ($s in @(
    @{ Bat = 'start.bat'; Lnk = 'AI Hub.lnk' }
    @{ Bat = 'open-dashboard.bat'; Lnk = 'AI Dashboard.lnk' }
)) {
    $bat = Join-Path $linkRoot $s.Bat
    $lnk = Join-Path $Desktop $s.Lnk
    $sc = $Wsh.CreateShortcut($lnk)
    $sc.TargetPath = $Cmd
    $sc.Arguments = "/c `"$bat`""
    $sc.WorkingDirectory = $linkRoot
    $sc.Save()
    Write-Host "Shortcut: $($s.Lnk)" -ForegroundColor Green
}

Write-Host ''
Write-Host 'Done: desktop has AI Hub + AI Dashboard only' -ForegroundColor Green
Write-Host ''