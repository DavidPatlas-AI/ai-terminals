$MainJs = 'C:\Program Files\cursor\resources\app\out\main.js'
$Loader = 'C:\Program Files\cursor\resources\app\out\cursor-rtl-loader.cjs'
$WbJs = 'C:\Program Files\cursor\resources\app\out\vs\workbench\workbench.desktop.main.js'
$WbCss = 'C:\Program Files\cursor\resources\app\out\vs\workbench\workbench.desktop.main.css'
$Log = Join-Path $env:USERPROFILE 'cursor-rtl.log'

Write-Host ''
Write-Host '=== Hebrew RTL Status ===' -ForegroundColor Cyan

$mainText = if (Test-Path -LiteralPath $MainJs) { [IO.File]::ReadAllText($MainJs) } else { '' }
$wbText = if (Test-Path -LiteralPath $WbJs) { [IO.File]::ReadAllText($WbJs) } else { '' }
$cssText = if (Test-Path -LiteralPath $WbCss) { [IO.File]::ReadAllText($WbCss) } else { '' }

$checks = @{
    'main.js patched' = $mainText -match 'cursor-rtl-loader'
    'loader file' = Test-Path -LiteralPath $Loader
    'workbench universal RTL' = $wbText -match 'START-UNIVERSAL-RTL-JS'
    'workbench FULL rtl.js (agent panel)' = $wbText -match 'START-CURSOR-RTL-FULL'
    'workbench CSS (agent panel)' = $cssText -match 'START-CURSOR-RTL-CSS'
    'V2 streamdown/glass fix CSS' = $cssText -match 'START-CURSOR-RTL-V2-STREAMDOWN'
    'V2 streamdown/glass fix JS' = $wbText -match 'START-CURSOR-RTL-V2-JS'
    'cursor-rtl.log' = Test-Path -LiteralPath $Log
    'Cursor running' = $null -ne (Get-Process Cursor -ErrorAction SilentlyContinue)
}

foreach ($kv in $checks.GetEnumerator()) {
    if ($kv.Value) { Write-Host "[OK] $($kv.Key)" -ForegroundColor Green }
    else { Write-Host "[--] $($kv.Key)" -ForegroundColor Yellow }
}

Write-Host ''
if ($checks['V2 streamdown/glass fix CSS'] -and $checks['V2 streamdown/glass fix JS']) {
    Write-Host 'V2 Streamdown/Glass fix INSTALLED (latest).' -ForegroundColor Green
} elseif ($checks['workbench FULL rtl.js (agent panel)'] -and $checks['workbench CSS (agent panel)']) {
    Write-Host 'V1 fix installed. Run fix-streamdown-rtl.vbs for V2.' -ForegroundColor Yellow
    Write-Host 'Restart Cursor: restart-cursor-for-rtl.bat' -ForegroundColor Yellow
} else {
    Write-Host 'Run fix-hebrew-rtl-full.vbs as Administrator.' -ForegroundColor Red
}
Write-Host ''
exit 0