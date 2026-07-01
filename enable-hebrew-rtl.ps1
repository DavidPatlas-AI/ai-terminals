# Apply Hebrew RTL patch to Cursor (requires Administrator)
$ErrorActionPreference = 'Stop'

$CursorOut = 'C:\Program Files\cursor\resources\app\out'
$MainJs = Join-Path $CursorOut 'main.js'
$ExtRtl = Join-Path $env:USERPROFILE '.cursor\extensions\satan2049.cursor-rtl-1.0.2-universal'
$ExtUni = Join-Path $env:USERPROFILE '.cursor\extensions\talco.universal-ide-rtl-1.3.3-universal'
$PatchLine = 'import{createRequire}from"module";try{createRequire(import.meta.url)("./cursor-rtl-loader.cjs")}catch(e){console.error("[Cursor RTL] error loading ./cursor-rtl-loader.cjs: ", e)}'
$Marker = 'cursor-rtl-loader.cjs'

function Test-Admin {
    $id = [Security.Principal.WindowsIdentity]::GetCurrent()
    $p = New-Object Security.Principal.WindowsPrincipal($id)
    return $p.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

Write-Host ''
Write-Host '=== Enable Hebrew RTL in Cursor ===' -ForegroundColor Cyan

if (-not (Test-Admin)) {
    Write-Host 'Need Administrator. Right-click enable-hebrew-rtl.bat -> Run as administrator' -ForegroundColor Red
    exit 1
}

if (-not (Test-Path -LiteralPath $MainJs)) {
    Write-Host "Cursor not found: $MainJs" -ForegroundColor Red
    exit 1
}

$content = [System.IO.File]::ReadAllText($MainJs)
if ($content -notmatch [regex]::Escape($Marker)) {
    $backup = $MainJs + '.rtl-backup-' + (Get-Date -Format 'yyyyMMddTHHmmss')
    Copy-Item -LiteralPath $MainJs -Destination $backup -Force
    $end = $content.IndexOf('*/')
    if ($end -lt 0) { throw 'Cannot find copyright block in main.js' }
    $insertAt = $end + 2
    $patched = $content.Substring(0, $insertAt) + "`n" + $PatchLine + $content.Substring($insertAt)
    [System.IO.File]::WriteAllText($MainJs, $patched, [System.Text.UTF8Encoding]::new($false))
    Write-Host '[OK] Patched main.js' -ForegroundColor Green
} else {
    Write-Host '[OK] main.js already patched' -ForegroundColor Green
}

$loaderSrc = Join-Path $ExtRtl 'resources\cursor-rtl-loader.cjs'
$loaderDst = Join-Path $CursorOut 'cursor-rtl-loader.cjs'
if (Test-Path -LiteralPath $loaderSrc) {
    Copy-Item -LiteralPath $loaderSrc -Destination $loaderDst -Force
    Write-Host '[OK] Copied loader' -ForegroundColor Green
}

$WbJs = Join-Path $CursorOut 'vs\workbench\workbench.desktop.main.js'
$JsPatch = Join-Path $ExtUni 'inject\workbench-rtl.js'
$UniMarker = 'START-UNIVERSAL-RTL-JS'

if ((Test-Path -LiteralPath $WbJs) -and (Test-Path -LiteralPath $JsPatch)) {
    $wb = [System.IO.File]::ReadAllText($WbJs)
    if ($wb -notmatch $UniMarker) {
        $wbBackup = $WbJs + '.rtl-backup'
        if (-not (Test-Path -LiteralPath $wbBackup)) {
            Copy-Item -LiteralPath $WbJs -Destination $wbBackup -Force
        }
        $inject = Get-Content -LiteralPath $JsPatch -Raw -Encoding UTF8
        $block = "`n/* ===== $UniMarker ===== */`n$inject`n/* ===== END-UNIVERSAL-RTL-JS ===== */`n"
        [System.IO.File]::WriteAllText($WbJs, $wb + $block, [System.Text.UTF8Encoding]::new($false))
        Write-Host '[OK] Patched workbench JS (universal)' -ForegroundColor Green
        $wb = [System.IO.File]::ReadAllText($WbJs)
    } else {
        Write-Host '[OK] workbench universal patch present' -ForegroundColor Green
    }

    $FullMarker = 'START-CURSOR-RTL-FULL'
    $RtlJs = Join-Path $ExtRtl 'resources\rtl.js'
    if ((Test-Path -LiteralPath $RtlJs) -and ($wb -notmatch $FullMarker)) {
        $rtlContent = Get-Content -LiteralPath $RtlJs -Raw -Encoding UTF8
        $fullBlock = "`n/* ===== $FullMarker ===== */`n$rtlContent`n/* ===== END-CURSOR-RTL-FULL ===== */`n"
        [System.IO.File]::WriteAllText($WbJs, $wb + $fullBlock, [System.Text.UTF8Encoding]::new($false))
        Write-Host '[OK] Injected full rtl.js into workbench (agent chat fix)' -ForegroundColor Green
    } elseif ($wb -match $FullMarker) {
        Write-Host '[OK] full rtl.js already in workbench' -ForegroundColor Green
    }
}

$WbCss = Join-Path $CursorOut 'vs\workbench\workbench.desktop.main.css'
$CssMarker = 'START-CURSOR-RTL-CSS'
if (Test-Path -LiteralPath $WbCss) {
    $css = [System.IO.File]::ReadAllText($WbCss)
    if ($css -notmatch $CssMarker) {
        $cssBlock = @"

/* ===== $CssMarker ===== */
[data-component=agent-panel] .markdown-root,
[data-component=agent-panel] .markdown-root p,
[data-component=agent-panel] .markdown-root li,
[data-component=agent-panel] .markdown-root > div,
[data-component=agent-panel] .markdown-section,
.agent-panel .markdown-root,
.agent-panel .markdown-root p,
.agent-panel .markdown-section,
.markdown-root,
.markdown-root p,
.markdown-root li,
.markdown-root > div,
.markdown-section,
.aislash-editor-input,
.aislash-editor-input p,
.aislash-editor-input-readonly,
.aislash-editor-input-readonly p,
.composer-human-message,
.composer-human-message-container,
.ui-prompt-input-editor__input,
.ui-prompt-input-editor__input > p {
  unicode-bidi: plaintext !important;
  text-align: start !important;
}
code, pre, .markdown-code-outer-container, .cursor-code-block-content,
.monaco-editor, .ui-code-block {
  direction: ltr !important;
  text-align: left !important;
  unicode-bidi: isolate !important;
}
/* ===== END-CURSOR-RTL-CSS ===== */
"@
        [System.IO.File]::WriteAllText($WbCss, $css + $cssBlock, [System.Text.UTF8Encoding]::new($false))
        Write-Host '[OK] Patched workbench CSS for agent-panel' -ForegroundColor Green
    } else {
        Write-Host '[OK] workbench CSS already patched' -ForegroundColor Green
    }
}

$V2Script = Join-Path $PSScriptRoot 'inject-rtl-streamdown-fix.ps1'
if (Test-Path -LiteralPath $V2Script) {
    Write-Host ''
    Write-Host 'Applying V2 Streamdown/Glass fix...' -ForegroundColor Cyan
    & $V2Script
} else {
    Write-Host '[WARN] inject-rtl-streamdown-fix.ps1 not found - skip V2' -ForegroundColor Yellow
}

Write-Host ''
Write-Host 'DONE - Close ALL Cursor windows and reopen' -ForegroundColor Green
exit 0