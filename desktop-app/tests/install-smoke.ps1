param([Parameter(Mandatory=$true)][string]$Target)
$ErrorActionPreference='Stop'
$installer=Get-ChildItem -LiteralPath "src-tauri/target/$Target/release/bundle/nsis" -Filter '*setup.exe' | Select-Object -First 1
if (-not $installer) {throw 'Installer not found'}
$tempRoot=[IO.Path]::GetFullPath($env:RUNNER_TEMP)
$installDir=[IO.Path]::GetFullPath((Join-Path $tempRoot 'the-box-install-check'))
if (-not $installDir.StartsWith(($tempRoot.TrimEnd('\')+'\'),[StringComparison]::OrdinalIgnoreCase)) {throw 'Installer test path escaped the CI temporary directory'}
$result=Start-Process -FilePath $installer.FullName -ArgumentList @('/S',("/D="+$installDir)) -WindowStyle Hidden -Wait -PassThru
if ($result.ExitCode -ne 0) {throw "Installer exited with $($result.ExitCode)"}
$binary=Join-Path $installDir 'desktop-app.exe'
if (-not (Test-Path -LiteralPath $binary)) {throw 'Installed application binary not found'}
node tests/worker-smoke.cjs $binary
if ($LASTEXITCODE -ne 0) {throw 'Installed worker smoke test failed'}
$uninstaller=Get-ChildItem -LiteralPath $installDir -Filter '*uninstall*.exe' | Select-Object -First 1
if (-not $uninstaller) {throw 'Uninstaller not found'}
$removed=Start-Process -FilePath $uninstaller.FullName -ArgumentList '/S' -WindowStyle Hidden -Wait -PassThru
if ($removed.ExitCode -ne 0) {throw "Uninstaller exited with $($removed.ExitCode)"}
for($i=0;$i -lt 30 -and (Test-Path -LiteralPath $binary);$i++){Start-Sleep -Milliseconds 200}
if(Test-Path -LiteralPath $binary){throw 'Uninstall left the application binary behind'}
Write-Output 'PASS: NSIS install, installed worker startup, and uninstall.'
