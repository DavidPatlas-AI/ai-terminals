$dir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ws = New-Object -ComObject WScript.Shell

$desktop = [Environment]::GetFolderPath('Desktop')
$fix = $ws.CreateShortcut((Join-Path $desktop 'Fix-Hebrew-RTL.lnk'))
$fix.TargetPath = Join-Path $dir 'AI-Fix-Hebrew.bat'
$fix.WorkingDirectory = $dir
$fix.Description = 'Fix Hebrew RTL in Cursor (V2 Streamdown)'
$fix.Save()

$paste = $ws.CreateShortcut((Join-Path $desktop 'Paste-Hebrew-Fix.lnk'))
$paste.TargetPath = Join-Path $dir 'paste-rtl-now.bat'
$paste.WorkingDirectory = $dir
$paste.Description = 'Copy Hebrew RTL fix to clipboard for Console'
$paste.Save()

Write-Host 'Desktop shortcuts: Fix-Hebrew-RTL.lnk, Paste-Hebrew-Fix.lnk'