$dir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ws = New-Object -ComObject WScript.Shell
$s = $ws.CreateShortcut("$env:USERPROFILE\Desktop\Fix-Hebrew-RTL.lnk")
$s.TargetPath = Join-Path $dir "fix-hebrew-rtl.vbs"
$s.WorkingDirectory = $dir
$s.Description = "Fix Hebrew RTL in Cursor"
$s.Save()
Write-Host OK