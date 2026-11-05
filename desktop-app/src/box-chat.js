/*
 * Phase 7 — The Box chat screen: the actual end-user-facing feature the
 * whole project vision was for (pick a project, ask in Hebrew, approve a
 * plan, watch it run, cancel if needed). Everything before this phase was
 * backend + an internal debug screen.
 *
 * Deliberately its own file, not merged into tauri-bridge.js or the main
 * inline script: keeps this feature's logic isolated and easy to find/edit
 * without touching the fragile giant minified translation blob elsewhere
 * in index.html. No-op entirely on the web build (no window.__TAURI__).
 */
(function () {
  'use strict';

  var isTauri = typeof window.__TAURI__ !== 'undefined';
  if (!isTauri) return;

  var invoke = window.__TAURI__.core.invoke;

  var state = {
    projectId: null,
    projectPath: null,
    conversationId: null,
    nextSeq: 1,
    busy: false, // a turn is in flight; composer disabled
  };

  // runId -> { textEl, bubbleEl, chipsEl, statusEl, cancelBtn, turnId }
  var liveRuns = {};
  var pendingEvents = {};
  var restoredProjectId = null;
  var provider = 'claude';
  var listenerReady = Promise.resolve();
  function rememberSelection() {
    return invoke('save_connected_preferences', { provider: provider, projectId: state.projectId });
  }
  var preferencesReady = invoke('connected_preferences').then(function(p) {
    provider = p.provider || 'claude';
    restoredProjectId = p.projectId;
    $('boxToolSelect').value = provider;
  }).catch(function(e) { showErr('boxComposerErr', e); });
  $('boxToolSelect').addEventListener('change', function() {
    provider = this.value;
    rememberSelection().catch(function(e) { showErr('boxComposerErr',e); });
  });
  $('boxConnectBtn').addEventListener('click', function() {
    invoke('open_connected_login',{tool:provider}).catch(function(e) { showErr('boxComposerErr',e); });
  });

  function $(id) { return document.getElementById(id); }

  function openBoxChat() {
    $('simpleHome').style.display = 'none';
    var proOnly = document.querySelector('.pro-only');
    if (proOnly) proOnly.style.display = 'none';
    var footer = $('siteFooter');
    if (footer) footer.style.display = 'none';
    $('boxChatView').hidden = false;
    if (!state.projectId) preferencesReady.then(loadProjects);
    renderDiagnostics();
  }

  // Phase 9 — doc 08 scenario 2: "מוצגת רשימה של Claude Code/Codex עם
  // סטטוס מותקן/חסר מדויק, לא ניחוש". A brand-new user needs to see this
  // before anything else here can work. Only git/node get an install link
  // (their official sites are stable, well-known URLs); claude/codex just
  // show status text, since guessing an install URL for them isn't safe.
  function renderDiagnostics() {
    var el = $('boxDiagnostics');
    el.innerHTML = '<span class="box-muted" style="font-size:11.5px">בודק כלים מותקנים…</span>';
    invoke('run_diagnostics').then(function (tools) {
      el.innerHTML = '';
      var anyMissing = false;
      tools.forEach(function (t) {
        var pill = document.createElement('span');
        pill.className = 'box-diag-pill ' + (t.installed ? 'ok' : 'missing');
        pill.textContent = (t.installed ? '✓ ' : '✗ ') + t.name;
        el.appendChild(pill);
        if (!t.installed) anyMissing = true;
      });
      if (anyMissing) {
        var hint = document.createElement('div');
        hint.className = 'box-diag-hint';
        hint.innerHTML =
          'חסרים כלים למעלה — עבור git: <a href="https://git-scm.com/download/win" target="_blank">git-scm.com</a>, ' +
          'עבור node: <a href="https://nodejs.org" target="_blank">nodejs.org</a>. ' +
          'עבור claude/codex — התקינו לפי ההוראות הרשמיות של הכלי המתאים.';
        el.appendChild(hint);
      }
    }).catch(function () {
      el.innerHTML = '<span class="box-muted" style="font-size:11.5px">לא ניתן היה לבדוק כלים מותקנים.</span>';
    });
  }

  function closeBoxChat() {
    $('boxChatView').hidden = true;
    $('simpleHome').style.display = '';
    var proOnly = document.querySelector('.pro-only');
    if (proOnly) proOnly.style.display = '';
    var footer = $('siteFooter');
    if (footer) footer.style.display = '';
  }

  function escapeHtml(s) {
    var d = document.createElement('div');
    d.textContent = String(s == null ? '' : s);
    return d.innerHTML;
  }

  // ---------------------------------------------------------- project picker

  function loadProjects() {
    invoke('db_list_projects').then(function (projects) {
      if (restoredProjectId) {
        var previous = projects.find(function(p) { return p.id === restoredProjectId; });
        restoredProjectId = null;
        if (previous) { selectProject(previous); return; }
      }
      var ul = $('boxProjectList');
      ul.innerHTML = '';
      projects.forEach(function (p) {
        var li = document.createElement('li');
        li.innerHTML =
          '<div class="bpl-name">' + escapeHtml(p.name) + '</div>' +
          '<div class="bpl-path">' + escapeHtml(p.path) + '</div>';
        li.addEventListener('click', function () { selectProject(p); });
        ul.appendChild(li);
      });
    }).catch(function (e) {
      showErr('boxAddProjectErr', e);
    });
  }

  function selectProject(p) {
    state.projectId = p.id;
    rememberSelection().catch(function(e) { showErr('boxComposerErr',e); });
    state.projectPath = p.path;
    state.conversationId = null;
    state.nextSeq = 1;
    $('boxProjectName').textContent = p.name;
    $('boxProjectPath').textContent = p.path;
    $('boxProjectPicker').hidden = true;
    $('boxThreadWrap').hidden = false;
    $('boxSwitchProjectBtn').hidden = false;
    $('boxThread').innerHTML = '';
    setComposerBusy(true);
    loadConversationHistory(p.id).finally(function() { setComposerBusy(false); });
  }

  function loadConversationHistory(projectId) {
    return invoke('db_list_conversations', { projectId: projectId }).then(function (list) {
      if (list.length === 0) return;
      var conv = list[list.length - 1];
      state.conversationId = conv.id;
      return invoke('db_list_turns', { conversationId: conv.id }).then(function (turns) {
        state.nextSeq = turns.length + 1;
        return turns.reduce(function(chain, turn) {
          return chain.then(function() {
            renderPastTurn(turn);
            return invoke('connected_turn_replies',{turnId:turn.id}).then(function(replies) {
              if (replies.length) addBubble('assistant',replies.join(''));
            });
          });
        },Promise.resolve());
      });
    }).catch(function (e) { showErr('boxComposerErr', e); });
  }

  function renderPastTurn(t) {
    addBubble('user', t.userIntent);
    var badge = document.createElement('div');
    badge.className = 'box-status-badge ' + statusToBadgeClass(t.status);
    badge.textContent = statusToHebrew(t.status);
    $('boxThread').appendChild(badge);
  }

  function statusToBadgeClass(status) {
    if (status === 'completed') return 'success';
    if (status === 'failed' || status === 'cancelled') return 'error';
    return 'running';
  }

  function statusToHebrew(status) {
    var map = {
      planning: 'בתכנון', awaiting_approval: 'ממתין לאישור', running: 'רץ',
      blocked: 'חסום', completed: 'הושלם', failed: 'נכשל', cancelled: 'בוטל',
    };
    return map[status] || status;
  }

  $('boxAddProjectBtn').addEventListener('click', function () {
    clearErr('boxAddProjectErr');
    invoke('select_project_folder').then(function (path) {
      if (!path) return; // user cancelled the native dialog
      return invoke('check_project_folder', { path: path }).then(function (info) {
        return invoke('db_create_project', { name: info.folderName, path: path, icon: null });
      }).then(function (project) {
        selectProject(project);
      });
    }).catch(function (e) { showErr('boxAddProjectErr', e); });
  });

  $('boxSwitchProjectBtn').addEventListener('click', function () {
    state.projectId = null;
    state.projectPath = null;
    state.conversationId = null;
    $('boxProjectPicker').hidden = false;
    $('boxThreadWrap').hidden = true;
    $('boxSwitchProjectBtn').hidden = true;
    $('boxProjectName').textContent = 'The Box';
    $('boxProjectPath').textContent = '';
    loadProjects();
  });

  // -------------------------------------------------------------- messaging

  function addBubble(kind, text) {
    var el = document.createElement('div');
    el.className = 'box-bubble ' + kind;
    el.textContent = text || '';
    $('boxThread').appendChild(el);
    scrollThreadToBottom();
    return el;
  }

  function scrollThreadToBottom() {
    var t = $('boxThread');
    t.scrollTop = t.scrollHeight;
  }

  function showErr(elId, e) { $(elId).textContent = String(e); }
  function clearErr(elId) { $(elId).textContent = ''; }

  function setComposerBusy(busy) {
    state.busy = busy;
    $('boxToolSelect').disabled = busy;
    $('boxConnectBtn').disabled = busy;
    $('boxWorkMode').disabled = busy;
    $('boxSwitchProjectBtn').disabled = busy;
    $('boxSendBtn').disabled = busy;
    $('boxComposerInput').disabled = busy;
  }

  function ensureConversation() {
    if (state.conversationId) return Promise.resolve(state.conversationId);
    return invoke('db_create_conversation', { projectId: state.projectId, title: null })
      .then(function (conv) {
        state.conversationId = conv.id;
        return conv.id;
      });
  }

  $('boxSendBtn').addEventListener('click', sendMessage);
  $('boxComposerInput').addEventListener('keydown', function (ev) {
    if (ev.key === 'Enter' && !ev.shiftKey) {
      ev.preventDefault();
      sendMessage();
    }
  });

  function sendMessage() {
    clearErr('boxComposerErr');
    if (state.busy || !state.projectId) return;
    var text = $('boxComposerInput').value.trim();
    if (!text) return;

    setComposerBusy(true);
    addBubble('user', text);
    $('boxComposerInput').value = '';

    if (!$('boxWorkMode').checked) {
      ensureConversation().then(function(conversationId) {
        return invoke('db_create_turn',{conversationId:conversationId,userIntent:text});
      }).then(function(turn) { startConnectedRun(turn.id); })
        .catch(function(e) { showErr('boxComposerErr',e); setComposerBusy(false); });
      return;
    }
    if (provider !== 'claude') {
      addBubble('assistant','עבודת קוד עם תוכנית זמינה כרגע ב-Claude. לשיחה עם הכלי שנבחר בטלו את מצב עבודת הקוד.');
      setComposerBusy(false);
      return;
    }
    var planningBubble = addBubble('assistant', 'בונה תוכנית עבודה…');

    ensureConversation()
      .then(function (conversationId) {
        return invoke('db_create_turn', { conversationId: conversationId, userIntent: text });
      })
      .then(function (turn) {
        return invoke('generate_task_plan', {
          turnId: turn.id,
          projectPath: state.projectPath,
          userIntent: text,
        }).then(function (plan) { return { turn: turn, plan: plan }; });
      })
      .then(function (res) {
        planningBubble.remove();
        renderPlanCard(res.turn, res.plan, text);
      })
      .catch(function (e) {
        planningBubble.remove();
        addBubble('assistant', 'שגיאה בבניית התוכנית: ' + e);
        setComposerBusy(false);
      });
  }

  function renderPlanCard(turn, plan, originalIntent) {
    var steps;
    try { steps = JSON.parse(plan.stepsJson); } catch (e) { steps = [plan.stepsJson]; }

    var card = document.createElement('div');
    card.className = 'box-plan-card';
    var riskClass = ['low', 'medium', 'high'].indexOf(plan.riskLevel) >= 0 ? plan.riskLevel : 'medium';
    var riskLabel = { low: 'סיכון נמוך', medium: 'סיכון בינוני', high: 'סיכון גבוה' }[riskClass];
    var ol = document.createElement('ol');
    steps.forEach(function (s) {
      var li = document.createElement('li');
      li.textContent = s;
      ol.appendChild(li);
    });
    card.innerHTML = '<span class="bpc-risk ' + riskClass + '">' + riskLabel + '</span>';
    card.appendChild(ol);
    var actions = document.createElement('div');
    actions.className = 'bpc-actions';
    var approveBtn = document.createElement('button');
    approveBtn.className = 'bpc-approve';
    approveBtn.textContent = 'אשר והרץ';
    var rejectBtn = document.createElement('button');
    rejectBtn.className = 'bpc-reject';
    rejectBtn.textContent = 'דחה';
    actions.appendChild(approveBtn);
    actions.appendChild(rejectBtn);
    card.appendChild(actions);
    $('boxThread').appendChild(card);
    scrollThreadToBottom();

    approveBtn.addEventListener('click', function () {
      card.classList.add('decided');
      actions.remove();
      invoke('decide_task_plan', { turnId: turn.id, taskPlanId: plan.id, decision: 'approved' })
        .then(function () {
          var mode = riskClass === 'high' ? 'default' : 'acceptEdits';
          startClaudeRun(turn.id, originalIntent, mode);
        })
        .catch(function (e) {
          addBubble('assistant', 'שגיאה באישור התוכנית: ' + e);
          setComposerBusy(false);
        });
    });

    rejectBtn.addEventListener('click', function () {
      card.classList.add('decided');
      actions.remove();
      invoke('decide_task_plan', { turnId: turn.id, taskPlanId: plan.id, decision: 'rejected' })
        .then(function () { setComposerBusy(false); })
        .catch(function (e) { showErr('boxComposerErr', e); setComposerBusy(false); });
    });
  }

  // ------------------------------------------------------------- run + live

  function startConnectedRun(turnId) {
    var bubble = addBubble('assistant','');
    var chips = document.createElement('div'); chips.className='box-chip-row';
    addChip(chips,provider); $('boxThread').appendChild(chips);
    var status = document.createElement('div'); status.className='box-status-badge running'; status.textContent='ממתין לתשובה…'; $('boxThread').appendChild(status);
    var cancelBtn=document.createElement('button'); cancelBtn.className='box-cancel-btn'; cancelBtn.textContent='בטל'; $('boxThread').appendChild(cancelBtn);
    listenerReady.then(function() {
      return invoke('run_connected_chat',{turnId:turnId,projectId:state.projectId,tool:provider});
    }).then(function(runId) {
      liveRuns[runId]={bubbleEl:bubble,chipsEl:chips,statusEl:status,cancelBtn:cancelBtn,turnId:turnId,provider:provider};
      cancelBtn.addEventListener('click',function() {
        invoke('cancel_run',{runId:runId}).then(function() {cancelBtn.disabled=true;}).catch(function(e) {showErr('boxComposerErr',e);});
      });
      (pendingEvents[runId] || []).forEach(handleProviderEvent);
      delete pendingEvents[runId];
    }).catch(function(e) {
      status.textContent='לא ניתן להתחיל'; cancelBtn.remove(); bubble.textContent=String(e); setComposerBusy(false);
    });
  }

  function startClaudeRun(turnId, prompt, permissionMode) {
    var bubble = addBubble('assistant', '');
    var chips = document.createElement('div');
    chips.className = 'box-chip-row';
    $('boxThread').appendChild(chips);
    var status = document.createElement('div');
    status.className = 'box-status-badge running';
    status.textContent = 'רץ…';
    $('boxThread').appendChild(status);
    var cancelBtn = document.createElement('button');
    cancelBtn.type = 'button';
    cancelBtn.className = 'box-cancel-btn';
    cancelBtn.textContent = 'בטל';
    $('boxThread').appendChild(cancelBtn);
    scrollThreadToBottom();

    invoke('run_claude_code', {
      turnId: turnId,
      projectId: state.projectId,
      projectPath: state.projectPath,
      prompt: prompt,
      permissionMode: permissionMode,
    }).then(function (runId) {
      liveRuns[runId] = { bubbleEl: bubble, chipsEl: chips, statusEl: status, cancelBtn: cancelBtn, turnId: turnId, provider: 'claude_code' };
      cancelBtn.addEventListener('click', function () {
        cancelBtn.disabled = true;
        invoke('cancel_run', { runId: runId }).catch(function (e) { showErr('boxComposerErr', e); });
      });
    }).catch(function (e) {
      status.remove();
      cancelBtn.remove();
      bubble.textContent = 'שגיאה בהפעלה: ' + e;
      setComposerBusy(false);
    });
  }

  function startCodexHandoff(turnId) {
    var bubble = addBubble('assistant', '');
    var label = document.createElement('div');
    label.className = 'box-chip-row';
    label.innerHTML = '<span class="box-chip">Codex (handoff)</span>';
    $('boxThread').appendChild(label);
    var chips = document.createElement('div');
    chips.className = 'box-chip-row';
    $('boxThread').appendChild(chips);
    var status = document.createElement('div');
    status.className = 'box-status-badge running';
    status.textContent = 'Codex רץ…';
    $('boxThread').appendChild(status);
    var cancelBtn = document.createElement('button');
    cancelBtn.type = 'button';
    cancelBtn.className = 'box-cancel-btn';
    cancelBtn.textContent = 'בטל';
    $('boxThread').appendChild(cancelBtn);
    scrollThreadToBottom();

    invoke('handoff_to_codex', { turnId: turnId, projectId: state.projectId, projectPath: state.projectPath })
      .then(function (runId) {
        liveRuns[runId] = { bubbleEl: bubble, chipsEl: chips, statusEl: status, cancelBtn: cancelBtn, turnId: turnId, provider: 'codex' };
        cancelBtn.addEventListener('click', function () {
          cancelBtn.disabled = true;
          invoke('cancel_run', { runId: runId }).catch(function (e) { showErr('boxComposerErr', e); });
        });
      }).catch(function (e) {
        status.remove();
        cancelBtn.remove();
        bubble.textContent = 'שגיאה בהעברה ל-Codex: ' + e;
        setComposerBusy(false);
      });
  }

  function addChip(container, text) {
    var chip = document.createElement('span');
    chip.className = 'box-chip';
    chip.textContent = text;
    container.appendChild(chip);
  }

  function finalizeRun(runId, live, exitStatus) {
    live.cancelBtn.remove();
    if (exitStatus === 'success') {
      live.statusEl.className = 'box-status-badge success';
      live.statusEl.textContent = 'הושלם';
      setComposerBusy(false);
    } else if (exitStatus === 'cancelled') {
      live.statusEl.className = 'box-status-badge error';
      live.statusEl.textContent = 'בוטל';
      setComposerBusy(false);
    } else {
      live.statusEl.className = 'box-status-badge error';
      live.statusEl.textContent = 'נכשל';
      if (live.provider === 'claude_code') {
        var retryBtn = document.createElement('button');
        retryBtn.type = 'button';
        retryBtn.className = 'box-retry-btn';
        retryBtn.textContent = 'נסה עם Codex במקום';
        retryBtn.addEventListener('click', function () {
          retryBtn.remove();
          setComposerBusy(true);
          startCodexHandoff(live.turnId);
        });
        $('boxThread').appendChild(retryBtn);
        setComposerBusy(false);
      } else {
        setComposerBusy(false);
      }
    }
    addUndoButton(live.turnId);
    scrollThreadToBottom();
    delete liveRuns[runId];
  }

  // Phase 8: a checkpoint was taken automatically right before this run
  // started (see lib.rs checkpoint_before_run) — offer to restore it
  // regardless of whether the run reports success, since "success" per the
  // adapter's exit_status doesn't mean the user is happy with the result.
  function addUndoButton(turnId) {
    invoke('get_latest_checkpoint_for_turn', { turnId: turnId }).then(function (checkpoint) {
      if (!checkpoint) return;
      var undoBtn = document.createElement('button');
      undoBtn.type = 'button';
      undoBtn.className = 'box-retry-btn';
      undoBtn.textContent = '↩ בטל שינויים (שחזר לפני הריצה)';
      undoBtn.addEventListener('click', function () {
        undoBtn.disabled = true;
        invoke('restore_checkpoint', { checkpointId: checkpoint.id })
          .then(function () {
            undoBtn.textContent = '✓ שוחזר';
            addBubble('assistant', 'השינויים בוטלו — הפרויקט חזר למצב שהיה לפני הריצה הזו.');
          })
          .catch(function (e) {
            undoBtn.disabled = false;
            showErr('boxComposerErr', e);
          });
      });
      $('boxThread').appendChild(undoBtn);
      scrollThreadToBottom();
    }).catch(function () { /* no checkpoint available — nothing to offer */ });
  }

  function handleProviderEvent(payload) {
    var live=liveRuns[payload.runId];
    if (!live) {
      if (!pendingEvents[payload.runId]) pendingEvents[payload.runId]=[];
      pendingEvents[payload.runId].push(payload);
      return;
    }
    var e=payload.event;
    if (e.type==='text_chunk') { live.bubbleEl.textContent+=e.text; scrollThreadToBottom(); }
    else if (e.type==='file_changed') { addChip(live.chipsEl,e.change_type+': '+e.path); }
    else if (e.type==='error') {
      var message=String(e.message || 'החיבור נכשל');
      if (/402|Payment Required|Insufficient/i.test(message)) message='אין יתרה זמינה ב-DeepSeek. אפשר לבחור כלי אחר; השיחה נשמרת.';
      else if (/UNSUPPORTED_CLIENT|IneligibleTier|no longer supported/i.test(message)) message='Google דחתה את לקוח Gemini המותקן. אפשר לבחור כלי אחר; השיחה נשמרת.';
      else if (/API_KEY|authentication|login|401/i.test(message)) message='נדרשת התחברות לכלי. לחצו על התחברות / בדיקת חשבון ונסו שוב.';
      live.bubbleEl.textContent+=message;
    }
    else if (e.type==='done') { finalizeRun(payload.runId,live,e.exit_status); }
  }
  if (window.__TAURI__.event) {
    listenerReady=window.__TAURI__.event.listen('provider_event',function(evt) { handleProviderEvent(evt.payload); });
  }

  // ------------------------------------------------------------------- init

  $('boxChatBtn').hidden = false;
  $('boxChatBtn').addEventListener('click', openBoxChat);
  $('boxBackBtn').addEventListener('click', closeBoxChat);
  preferencesReady.then(openBoxChat);
})();
