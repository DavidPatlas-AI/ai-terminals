/*
 * Phase 2 — Web/Tauri compatibility bridge.
 *
 * Loaded after the existing app script. Detects whether the page is running
 * inside the Tauri desktop shell (window.__TAURI__ present) or as a normal
 * web page (browser / Netlify). On the web, this file is a no-op except for
 * showing a friendly Hebrew/English/Russian message if a desktop-only action
 * is attempted. It never changes any existing behavior of the original site.
 */
(function () {
  'use strict';

  var isTauri = typeof window.__TAURI__ !== 'undefined';

  var UNAVAILABLE_MSG = {
    he: 'פעולה זו זמינה באפליקציית המחשב',
    en: 'This action is available in the desktop app',
    ru: 'Это действие доступно в настольном приложении',
  };

  var LABELS = {
    he: { path: 'נתיב', type: 'סוג', git: 'Git', status: 'סטטוס', ready: 'מוכן', none: 'לא נבחרה תיקייה', open: 'פתח בסייר' },
    en: { path: 'Path', type: 'Type', git: 'Git', status: 'Status', ready: 'Ready', none: 'No folder selected', open: 'Open in Explorer' },
    ru: { path: 'Путь', type: 'Тип', git: 'Git', status: 'Статус', ready: 'Готово', none: 'Папка не выбрана', open: 'Открыть в проводнике' },
  };

  function currentLang() {
    var l = document.documentElement.getAttribute('lang');
    return LABELS[l] ? l : 'he';
  }

  function label(key) {
    return LABELS[currentLang()][key];
  }

  function notifyUnavailable() {
    var text = UNAVAILABLE_MSG[currentLang()] || UNAVAILABLE_MSG.he;
    if (typeof window.showToast === 'function') {
      window.showToast(text);
    } else {
      alert(text);
    }
  }

  function invoke(cmd, args) {
    return window.__TAURI__.core.invoke(cmd, args);
  }

  var selectedPath = null;

  function renderPanel(info) {
    var panel = document.getElementById('desktopProjectPanel');
    if (!panel) return;
    panel.hidden = false;

    if (!info) {
      panel.innerHTML = '<div class="dpp-row dpp-muted">' + label('none') + '</div>';
      return;
    }

    var typeText = info.detectedProjectType;
    var gitText = info.hasGit ? '✓' : '—';

    panel.innerHTML =
      '<div class="dpp-row"><strong>' + escapeHtml(info.folderName) + '</strong></div>' +
      '<div class="dpp-row dpp-path">' + escapeHtml(info.path) + '</div>' +
      '<div class="dpp-row dpp-meta">' +
        '<span>' + label('type') + ': ' + escapeHtml(typeText) + '</span>' +
        '<span>' + label('git') + ': ' + gitText + '</span>' +
        '<span class="dpp-ready">' + label('status') + ': ' + label('ready') + '</span>' +
      '</div>' +
      '<div class="dpp-row">' +
        '<button type="button" class="mbtn" id="dppOpenBtn">' + label('open') + '</button>' +
      '</div>';

    var openBtn = document.getElementById('dppOpenBtn');
    if (openBtn) {
      openBtn.addEventListener('click', function () {
        invoke('open_project_folder', { path: selectedPath }).catch(function (e) {
          console.error('[tauri-bridge] open_project_folder failed', e);
        });
      });
    }
  }

  function escapeHtml(s) {
    var d = document.createElement('div');
    d.textContent = String(s == null ? '' : s);
    return d.innerHTML;
  }

  function pickProjectFolder() {
    if (!isTauri) {
      notifyUnavailable();
      return;
    }
    invoke('select_project_folder')
      .then(function (path) {
        if (!path) return; // user cancelled the native dialog
        selectedPath = path;
        return invoke('check_project_folder', { path: path }).then(function (info) {
          renderPanel(Object.assign({}, info, { path: path }));
        });
      })
      .catch(function (e) {
        console.error('[tauri-bridge] project folder selection failed', e);
      });
  }

  function openDbDebugWindow() {
    if (!isTauri) return;
    invoke('open_db_debug_window').catch(function (e) {
      console.error('[tauri-bridge] open_db_debug_window failed', e);
    });
  }

  function init() {
    var btn = document.getElementById('desktopProjectBtn');
    if (btn) {
      if (isTauri) btn.hidden = false; // stays hidden on the web build
      btn.addEventListener('click', pickProjectFolder);
    }

    var debugBtn = document.getElementById('dbDebugBtn');
    if (debugBtn) {
      if (isTauri) debugBtn.hidden = false; // stays hidden on the web build
      debugBtn.addEventListener('click', openDbDebugWindow);
    }

    if (isTauri) {
      invoke('app_info')
        .then(function (info) {
          console.log('[tauri-bridge] running under Tauri:', info);
        })
        .catch(function (e) {
          console.error('[tauri-bridge] app_info failed', e);
        });
    }
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }

  // Exposed for manual testing from the devtools console only.
  window.__desktopBridge = { isTauri: isTauri, pickProjectFolder: pickProjectFolder };
})();
