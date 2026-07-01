# Fix Hebrew in Cursor Streamdown / Glass agent chat (requires Administrator)
$ErrorActionPreference = 'Stop'

function Test-Admin {
    $p = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
    return $p.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

if (-not (Test-Admin)) {
    Write-Host 'Run as Administrator' -ForegroundColor Red
    exit 1
}

$CursorOut = 'C:\Program Files\cursor\resources\app\out'
$WbCss = Join-Path $CursorOut 'vs\workbench\workbench.desktop.main.css'
$WbJs = Join-Path $CursorOut 'vs\workbench\workbench.desktop.main.js'
$V2Css = 'START-CURSOR-RTL-V2-STREAMDOWN'
$V2Js = 'START-CURSOR-RTL-V2-JS'

Write-Host ''
Write-Host '=== Streamdown / Glass RTL Fix ===' -ForegroundColor Cyan

if (Test-Path -LiteralPath $WbCss) {
    $css = [IO.File]::ReadAllText($WbCss)
    if ($css -notmatch $V2Css) {
        $block = @"

/* ===== $V2Css ===== */
[data-streamdown],
[data-streamdown] p,
[data-streamdown] li,
[data-streamdown] span,
[data-streamdown] div,
[data-streamdown] h1,
[data-streamdown] h2,
[data-streamdown] h3,
.markdown-root.markdown-normalized,
.markdown-root.markdown-normalized p,
.markdown-root.markdown-normalized li,
.markdown-root.markdown-normalized > div,
body[data-cursor-glass-mode=true] [data-component=agent-panel] .markdown-root,
body[data-cursor-glass-mode=true] [data-component=agent-panel] [data-streamdown],
body[data-cursor-glass-mode=true] [data-component=agent-panel] .markdown-root.markdown-normalized,
.ui-agent-tray__prompt-wrap .ui-prompt-input-editor__input,
.ui-agent-tray__prompt-wrap .ui-prompt-input-editor__input p {
  unicode-bidi: plaintext !important;
  text-align: start !important;
}
[data-streamdown] code,
[data-streamdown] pre,
.markdown-root.markdown-normalized code,
.markdown-root.markdown-normalized pre {
  direction: ltr !important;
  text-align: left !important;
  unicode-bidi: isolate !important;
}
/* ===== END-CURSOR-RTL-V2-STREAMDOWN ===== */
"@
        [IO.File]::WriteAllText($WbCss, $css + $block, [Text.UTF8Encoding]::new($false))
        Write-Host '[OK] CSS streamdown fix applied' -ForegroundColor Green
    } else {
        Write-Host '[OK] CSS streamdown fix already present' -ForegroundColor Green
    }
}

if (Test-Path -LiteralPath $WbJs) {
    $js = [IO.File]::ReadAllText($WbJs)
    if ($js -notmatch $V2Js) {
        $snippet = @"

/* ===== $V2Js ===== */
(function(){
  var RTL=/[\u0590-\u05FF\u0600-\u06FF]/;
  var SEL='[data-streamdown],.markdown-root.markdown-normalized,.markdown-root.markdown-normalized p,.markdown-root.markdown-normalized li,[data-component=agent-panel] .markdown-root p,[data-component=agent-panel] [data-streamdown]';
  function fix(){
    document.querySelectorAll(SEL).forEach(function(el){
      if(!el||!el.textContent||el.closest('code,pre,.monaco-editor'))return;
      el.style.setProperty('unicode-bidi','plaintext','important');
      el.style.setProperty('text-align','start','important');
      if(RTL.test(el.textContent))el.setAttribute('dir','rtl');
    });
  }
  fix();
  new MutationObserver(fix).observe(document.documentElement,{childList:true,subtree:true,characterData:true});
  window.__cursorRtlStreamdownFix=fix;
  console.log('[RTL V2] streamdown/glass fix active');
})();
/* ===== END-CURSOR-RTL-V2-JS ===== */
"@
        [IO.File]::WriteAllText($WbJs, $js + $snippet, [Text.UTF8Encoding]::new($false))
        Write-Host '[OK] JS streamdown fix applied' -ForegroundColor Green
    } else {
        Write-Host '[OK] JS streamdown fix already present' -ForegroundColor Green
    }
}

Write-Host ''
Write-Host 'DONE - close ALL Cursor windows and reopen' -ForegroundColor Green
exit 0