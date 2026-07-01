# Desktop cleanup — keeps only AI Hub + AI Dashboard shortcuts for AI tools
$ErrorActionPreference = 'Continue'
$Folder = (Resolve-Path $PSScriptRoot).Path
$Desktop = [Environment]::GetFolderPath('Desktop')
$Projects = Join-Path $Desktop 'פרויקטים'
$ContentDir = Join-Path $Projects 'תוכן-יהדות'

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
    cmd /c mklink /J `"$link`" `"$Folder`" 2>$null | Out-Null
    Write-Host "Junction: AI-Terminals" -ForegroundColor Green
}

# 2. Remove stray AI shortcuts / bats from desktop root
$keep = @('AI Hub.lnk', 'AI Dashboard.lnk', 'YouTube.lnk')
Get-ChildItem -LiteralPath $Desktop -File -EA SilentlyContinue | ForEach-Object {
    $n = $_.Name
    if ($keep -contains $n) { return }
    if ($n -like 'AI - *' -or $n -like 'AI-*' -or $n -match '^AI\.lnk$' -or
        $n -like '*פתח*עברית*' -or $n -like '*?????*??????*') {
        Remove-Item -LiteralPath $_.FullName -Force -EA SilentlyContinue
        Write-Host "Removed desktop file: $n" -ForegroundColor DarkYellow
    }
}

# 3. Move loose Torah content from user profile to projects
Ensure-Dir $ContentDir
$homeFiles = @(
    'פרשת_קורח_5_רעיונות_סרטונים_קצרים.md',
    'פרשת_קורח_5_רעיונות_סרטונים_קצרים.html',
    'פרשת_קורח_AI_תורה_תוכן_נכון.html',
    'פוסט_ויראלי_AI_ותורה.md'
)
foreach ($f in $homeFiles) {
    $src = Join-Path $env:USERPROFILE $f
    if (Test-Path -LiteralPath $src) {
        $dst = Join-Path $ContentDir $f
        if (-not (Test-Path -LiteralPath $dst)) {
            Move-Item -LiteralPath $src -Destination $dst -Force
            Write-Host "Moved: $f -> פרויקטים\תוכן-יהדות" -ForegroundColor Green
        } else {
            Remove-Item -LiteralPath $src -Force -EA SilentlyContinue
            Write-Host "Removed duplicate: $f (already in projects)" -ForegroundColor DarkGray
        }
    }
}

# 4. Remove .lnk clutter inside project folder
Get-ChildItem -LiteralPath $Folder -Filter '*.lnk' -File -EA SilentlyContinue | ForEach-Object {
    Remove-Item -LiteralPath $_.FullName -Force
    Write-Host "Removed internal shortcut: $($_.Name)" -ForegroundColor DarkGray
}

# 5. Create exactly 2 desktop shortcuts
$Wsh = New-Object -ComObject WScript.Shell
$Cmd = $env:ComSpec
$shortcuts = @(
    @{ Bat = 'start.bat'; Lnk = 'AI Hub.lnk'; Desc = 'AI menu' }
    @{ Bat = 'open-dashboard.bat'; Lnk = 'AI Dashboard.lnk'; Desc = 'Token dashboard' }
)
foreach ($s in $shortcuts) {
    $bat = Join-Path $link $s.Bat
    if (-not (Test-Path -LiteralPath $bat)) { $bat = Join-Path $Folder $s.Bat }
    $lnk = Join-Path $Desktop $s.Lnk
    $sc = $Wsh.CreateShortcut($lnk)
    $sc.TargetPath = $Cmd
    $sc.Arguments = "/c `"$bat`""
    $sc.WorkingDirectory = $link
    $sc.Description = $s.Desc
    $sc.Save()
    Write-Host "Shortcut: $($s.Lnk)" -ForegroundColor Green
}

Write-Host ''
Write-Host '  Desktop is clean: AI Hub + AI Dashboard only' -ForegroundColor Green
Write-Host '  Project: Desktop\AI-Terminals (junction)' -ForegroundColor DarkGray
Write-Host ''