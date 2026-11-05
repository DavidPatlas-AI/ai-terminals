/* The Box Lite: dependency-free UI, one active request, durable local history. */
(() => {
  'use strict';
  const $ = id => document.getElementById(id);
  const preview = window.__BOX_PREVIEW__ === true || !window.__TAURI__;
  const names = { claude:'Claude', codex:'Codex', gemini:'Gemini', grok:'Grok', ollama:'Ollama', deepseek:'DeepSeek' };
  const colors = { claude:'#b58b63',codex:'#6e9586',gemini:'#6e88bf',grok:'#8b84a3',ollama:'#94a876',deepseek:'#7696bc' };
  const state = { projects:[],tools:[],projectId:null,conversationId:null,provider:'claude',economy:true,busy:false,loading:false,active:null,firstSeq:null,more:false,navigation:0 };
  const pending = new Map();
  let demoListener;
  const demoTurns=[];
  async function demoInvoke(command,args={}) {
    switch(command) {
      case 'lite_bootstrap':return {projects:[{id:'personal',name:'מרחב אישי',path:'תצוגה מקדימה'}],projectId:'personal',provider:'claude',conversationId:null,economy:true,tools:Object.keys(names).map(id=>({id,installed:true})),platform:'preview',arch:''};
      case 'db_list_conversations':return demoTurns.length ? [{id:'preview-chat',title:demoTurns[0].userIntent.slice(0,40)}] : [];
      case 'db_create_conversation':return {id:'preview-chat',title:args.title};
      case 'db_create_turn':{const turn={id:'preview-'+Date.now(),seq:demoTurns.length+1,userIntent:args.userIntent,status:'completed',reply:'זוהי תצוגה מקדימה של הממשק. באפליקציית המחשב ההודעה נשלחת לכלי שבחרת באמצעות החשבון הקיים שלך. כאן לא נשלחת בקשה לשום שירות.'};demoTurns.push(turn);return turn;}
      case 'lite_history':return {items:demoTurns,hasMore:false};
      case 'run_connected_chat':{
        const id='preview-run';const turn=demoTurns[demoTurns.length-1];
        demoListener({payload:{runId:id,event:{type:'text_chunk',text:turn.reply}}});
        demoListener({payload:{runId:id,event:{type:'done',exit_status:'success'}}});
        return id;
      }
      case 'save_lite_preferences':return;
      default:throw Error('פעולה זו זמינה באפליקציית המחשב.');
    }
  }
  const invoke = preview ? demoInvoke : window.__TAURI__.core.invoke;
  const listenerReady = preview ? Promise.resolve().then(()=>{demoListener=handleEvent;}) : window.__TAURI__.event.listen('provider_event',handleEvent);
  listenerReady.catch(error=>notice('לא ניתן לקבל תשובות מהכלים. נסה לפתוח את האפליקציה מחדש. '+String(error)));
  function icon(id) { const svg=document.createElementNS('http://www.w3.org/2000/svg','svg');const use=document.createElementNS(svg.namespaceURI,'use');use.setAttribute('href','#i-'+id);svg.append(use);svg.setAttribute('aria-hidden','true');return svg; }
  function notice(text) {$('noticeText').textContent=text;$('notice').hidden=!text;}
  $('dismissNotice').onclick=()=>notice('');
  function save() {
    if (!state.projectId) return Promise.resolve();
    return invoke('save_lite_preferences',{provider:state.provider,projectId:state.projectId,conversationId:state.conversationId,economy:state.economy});
  }
  function remember() {return save().catch(()=>notice('לא ניתן לשמור את הבחירה כרגע. השיחה נשארת פתוחה.'));}
  function nearBottom() {return $('thread').scrollHeight-$('thread').scrollTop-$('thread').clientHeight<140;}
  function scrollEnd(force=false) {if(force || nearBottom()) $('thread').scrollTop=$('thread').scrollHeight;}
  function resizeInput() {$('message').style.height='auto';$('message').style.height=Math.min($('message').scrollHeight||29,180)+'px';}
  function updateControls() {
    const disabled=state.busy||state.loading;
    for(const id of ['provider','project','newChat','addProject','connect','economy','classic']) $(id).disabled=disabled;
    for(const button of $('conversations').querySelectorAll('button')) button.disabled=disabled;
    $('send').disabled=disabled||!state.projectId||!$('message').value.trim();
    $('message').disabled=state.loading;
    $('send').hidden=state.busy;
    $('stop').hidden=!state.busy;
    $('economyLabel').textContent=state.economy?'מצב חסכוני מופעל':'הקשר שיחה מורחב';
    $('composerHint').textContent=state.provider==='deepseek'?'DeepSeek · שימוש ב-API בתשלום':state.provider==='ollama'?'מקומי · משתמש במשאבי המחשב':'שיחה בלבד · ללא שינוי קבצים';
  }
  function renderTools() {
    const tool=state.tools.find(t=>t.id===state.provider);
    $('provider').value=state.provider;
    $('connect').replaceChildren(icon(state.provider==='ollama'?'settings':'key'),document.createTextNode(state.provider==='ollama'?'בדיקת מודלים':state.provider==='deepseek'?'פרטי חיבור':!tool?.installed?'עזרה והתקנה':'חיבור חשבון'));
    $('providerDot').style.backgroundColor=colors[state.provider];
    $('connection').textContent=preview?'תצוגה מקדימה':!tool?.installed?'לא זוהה במחשב':tool.lastStatus==='success'?'אימות קודם תקין':tool.lastStatus==='error'?'נדרש לבדוק חיבור':state.provider==='deepseek'?'API · דורש מפתח ויתרה':'מזוהה במחשב';
    const strip=document.querySelector('.tool-strip');strip.replaceChildren();
    for(const tool of state.tools) {
      const span=document.createElement('span');span.className='tool-token'+(tool.installed?' detected':'');span.textContent=names[tool.id];span.title=preview?'תצוגה מקדימה':tool.installed?'הכלי זוהה; ההתחברות נבדקת כששולחים הודעה':'לא זוהה במחשב';strip.append(span);
    }
    updateControls();
  }
  $('provider').onchange=()=>{state.provider=$('provider').value;renderTools();remember();notice('');};
  $('message').oninput=()=>{resizeInput();updateControls();};
  $('message').onkeydown=event=>{if(event.key==='Enter'&&!event.shiftKey&&!event.isComposing){event.preventDefault();send();}};
  document.querySelectorAll('[data-prompt]').forEach(button=>{button.onclick=()=>{$('message').value=button.dataset.prompt;resizeInput();updateControls();$('message').focus();};});
  function closeSidebar() {$('sidebar').classList.remove('open');$('scrim').hidden=true;$('menu').setAttribute('aria-expanded','false');}
  $('menu').onclick=()=>{const open=!$('sidebar').classList.contains('open');$('sidebar').classList.toggle('open',open);$('scrim').hidden=!open;$('menu').setAttribute('aria-expanded',String(open));if(open)$('newChat').focus();};
  $('scrim').onclick=closeSidebar;
  document.addEventListener('keydown',event=>{
    if(event.key==='Escape'&&$('sidebar').classList.contains('open')) {closeSidebar();$('menu').focus();}
    if((event.metaKey||event.ctrlKey)&&event.key.toLowerCase()==='n'&&!state.busy&&!state.loading){event.preventDefault();newChat();}
  });
  function showWelcome() {$('welcome').hidden=$('messages').childElementCount>0;}
  function newChat() {
    if(state.busy||state.loading)return;
    state.conversationId=null;state.firstSeq=null;state.more=false;state.navigation++;
    $('messages').replaceChildren();$('older').hidden=true;showWelcome();notice('');remember();renderConversations().catch(error=>notice(String(error)));closeSidebar();$('message').focus();
  }
  $('newChat').onclick=newChat;
  async function renderConversations() {
    const project=state.projectId;
    const rows=await invoke('db_list_conversations',{projectId:project});
    if(project!==state.projectId)return;
    const list=$('conversations');list.replaceChildren();$('chatCount').textContent=rows.length||'';
    for(const conversation of rows.slice(-25).reverse()) {
      const button=document.createElement('button');button.className='conversation';button.setAttribute('aria-current',String(conversation.id===state.conversationId));button.append(icon('chat'));
      const title=document.createElement('span');title.textContent=conversation.title||'שיחה קודמת';button.append(title);button.title=title.textContent;button.disabled=state.busy||state.loading;
      button.onclick=()=>loadConversation(conversation.id);list.append(button);
    }
    if(!rows.length) {const p=document.createElement('p');p.className='small-muted';p.textContent='השיחה הראשונה שלך מתחילה כאן.';list.append(p);}
  }
  function renderAnswer(element,value) {
    element.replaceChildren();
    const fenced=/```([^\n]*)\n([\s\S]*?)```/g;let start=0;let match;
    while((match=fenced.exec(value))) {
      element.append(document.createTextNode(value.slice(start,match.index)));
      const pre=document.createElement('pre');const code=document.createElement('code');code.textContent=match[2];pre.append(code);element.append(pre);start=fenced.lastIndex;
    }
    element.append(document.createTextNode(value.slice(start)));
  }
  function turnElements(prompt,reply='',status='completed') {
    const turn=document.createElement('article');turn.className='turn';
    const user=document.createElement('div');user.className='message user';user.textContent=prompt;turn.append(user);
    const row=document.createElement('div');row.className='assistant-row';
    const avatar=document.createElement('span');avatar.className='assistant-avatar';avatar.append(icon('box'));avatar.setAttribute('aria-hidden','true');row.append(avatar);
    const content=document.createElement('div');content.className='assistant-content';const answer=document.createElement('div');answer.className='message assistant';renderAnswer(answer,reply);content.append(answer);
    const meta=document.createElement('div');meta.className='answer-meta';content.append(meta);row.append(content);turn.append(row);
    if(status!=='completed') {const note=document.createElement('span');note.textContent=status==='cancelled'?'נעצרה':status==='failed'?'החיבור לא הושלם':'שיחה שלא הושלמה';meta.append(note);}
    if(reply)addCopy(meta,reply);
    return {turn,answer,meta};
  }
  function addCopy(meta,text) {
    const button=document.createElement('button');button.className='icon-button';button.title='העתק תשובה';button.setAttribute('aria-label','העתק תשובה');button.append(icon('copy'));
    button.onclick=async()=>{try{await navigator.clipboard.writeText(text);$('runStatus').textContent='התשובה הועתקה';}catch{notice('ההעתקה אינה זמינה כרגע. אפשר לסמן את הטקסט ולהעתיק.');}};meta.append(button);
  }
  async function loadConversation(id,older=false) {
    if(state.busy||state.loading)return;
    state.loading=true;updateControls();const navigation=++state.navigation;
    try {
      const history=await invoke('lite_history',{conversationId:id,beforeSeq:older?state.firstSeq:null});
      if(navigation!==state.navigation)return;
      state.conversationId=id;state.more=history.hasMore;
      const oldHeight=$('thread').scrollHeight;
      if(!older)$('messages').replaceChildren();
      const fragment=document.createDocumentFragment();
      for(const item of history.items) fragment.append(turnElements(item.userIntent,item.reply,item.status).turn);
      if(older)$('messages').prepend(fragment);else $('messages').append(fragment);
      if(history.items.length)state.firstSeq=history.items[0].seq;
      $('older').hidden=!state.more;showWelcome();
      if(older)$('thread').scrollTop+=$('thread').scrollHeight-oldHeight;else scrollEnd(true);
      await remember();await renderConversations();closeSidebar();
    } catch(error){notice('לא ניתן לטעון את השיחה. '+String(error));}
    finally {state.loading=false;updateControls();}
  }
  $('older').onclick=()=>loadConversation(state.conversationId,true);
  $('project').onchange=async()=>{
    if(state.busy||state.loading)return;
    state.projectId=$('project').value;state.conversationId=null;newChat();await remember();
  };
  $('addProject').onclick=async()=>{
    try {
      const path=await invoke('select_project_folder');if(!path)return;
      const existing=state.projects.find(p=>p.path===path);
      const info=existing?null:await invoke('check_project_folder',{path});
      const project=existing||await invoke('db_create_project',{name:info.folderName,path,icon:null});
      if(!existing)state.projects.push(project);
      renderProjects();state.projectId=project.id;$('project').value=project.id;newChat();
    }catch(error){notice(String(error));}
  };
  function renderProjects() {
    $('project').replaceChildren();
    for(const project of state.projects){const option=document.createElement('option');option.value=project.id;option.textContent=project.name;option.title=project.path;$('project').append(option);}
    $('project').value=state.projectId;
  }
  function readableError(error) {
    const text=String(error||'');
    if(/could not be saved/i.test(text))return 'התשובה התקבלה, אך לא ניתן לשמור אותה. כדאי להעתיק אותה ולבדוק שיש מקום פנוי בדיסק.';
    if(/402|Payment Required|Insufficient/i.test(text))return 'אין יתרה זמינה ב-DeepSeek. אפשר לבחור כלי אחר ולהמשיך באותה שיחה.';
    if(/UNSUPPORTED_CLIENT|IneligibleTier|no longer supported/i.test(text))return 'Google דחתה את גרסת Gemini המותקנת. אפשר לעדכן את הכלי או לבחור כלי אחר.';
    if(/4551|Application Control|Access is denied|ShellExecute/i.test(text))return 'Windows חסם את הפעלת הכלי. ההודעה נשמרה; נדרש לפתור את חסימת ההפעלה במחשב.';
    if(/not installed|No such file|not found|os error 2/i.test(text))return 'לא מצאנו את הכלי במחשב. אפשר לחבר כלי שכבר מותקן, או לבחור כלי אחר.';
    if(/401|403|API.key|API_KEY|authenticate|authentication|logged.in|login/i.test(text))return 'נדרשת התחברות לכלי. לחץ על חיבור חשבון, התחבר בחלון הרשמי וחזור לכאן.';
    if(/11434|connect|Connection refused|Failed to connect/i.test(text)&&state.provider==='ollama')return 'Ollama אינו זמין כרגע. פתח את Ollama במחשב, או בחר כלי אחר.';
    if(/No local model/i.test(text))return 'עדיין אין מודל מקומי מותקן ב-Ollama. לא התחלנו הורדה. אפשר לבחור כלי אחר בינתיים.';
    if(/timed out|timeout|did not complete/i.test(text))return 'התשובה לא הושלמה בזמן. השיחה נשמרה; אפשר לנסות שוב או לבחור כלי אחר.';
    return text.slice(0,500)||'החיבור לא הושלם. אפשר לנסות שוב.';
  }
  async function send() {
    const prompt=$('message').value.trim();if(!prompt||state.busy||state.loading||!state.projectId)return;
    const tool=state.tools.find(t=>t.id===state.provider);
    if(tool&&!tool.installed){notice('הכלי שנבחר לא זוהה במחשב. בחר כלי מזוהה או חבר את הכלי בחשבון הרשמי.');return;}
    notice('');state.busy=true;updateControls();
    const elements=turnElements(prompt,'','running');$('messages').append(elements.turn);showWelcome();scrollEnd(true);
    state.active={...elements,prompt,provider:state.provider,runId:null,text:'',error:'',cancelRequested:false};
    $('message').value='';resizeInput();$('message').focus();$('runStatus').textContent='מתחבר ל-'+names[state.provider]+'…';
    try {
      await listenerReady;
      if(!state.conversationId){const c=await invoke('db_create_conversation',{projectId:state.projectId,title:prompt.slice(0,45)});state.conversationId=c.id;await save();}
      const turn=await invoke('db_create_turn',{conversationId:state.conversationId,userIntent:prompt});
      state.active.turnId=turn.id;
      const runId=await invoke('run_connected_chat',{turnId:turn.id,projectId:state.projectId,tool:state.provider});
      state.active.runId=runId;
      if(state.active.cancelRequested)await invoke('cancel_run',{runId});
      const events=pending.get(runId)||[];pending.clear();for(const event of events)handleEvent({payload:event});
      await renderConversations();
    }catch(error){if(state.active){state.active.error=readableError(error);finish('error');}}
  }
  $('composer').onsubmit=event=>{event.preventDefault();send();};
  $('stop').onclick=async()=>{
    const active=state.active;if(!active)return;
    active.cancelRequested=true;$('runStatus').textContent='עוצר את הבקשה…';
    if(active.runId){try{await invoke('cancel_run',{runId:active.runId});}catch(error){if(state.active===active)notice(readableError(error));}}
  };
  function handleEvent(event) {
    const payload=event.payload;const active=state.active;
    if(!active||!state.busy)return;
    if(!active.runId){const queue=pending.get(payload.runId)||[];if(queue.length<256)queue.push(payload);if(pending.size<8)pending.set(payload.runId,queue);return;}
    if(payload.runId!==active.runId)return;
    const part=payload.event;
    if(part.type==='text_chunk'){const scroll=nearBottom();active.text+=part.text;active.answer.textContent=active.text;$('runStatus').textContent=names[active.provider]+' משיב…';if(scroll)scrollEnd(true);}
    else if(part.type==='error')active.error=readableError(part.message);
    else if(part.type==='done')finish(part.exit_status);
  }
  function finish(status) {
    const active=state.active;if(!active)return;
    const scroll=nearBottom();renderAnswer(active.answer,active.text);active.meta.replaceChildren();
    if(status==='success') {const tag=document.createElement('span');tag.textContent=names[active.provider];active.meta.append(tag);addCopy(active.meta,active.text);$('runStatus').textContent='התשובה התקבלה · השיחה נשמרה';}
    else {
      const error=document.createElement('div');error.className='answer-error';error.textContent=status==='cancelled'?'הבקשה נעצרה. אפשר להמשיך כשנוח לך.':active.error||'התשובה לא הושלמה. אפשר לנסות שוב.';active.answer.append(error);
      const retry=document.createElement('button');retry.className='retry';retry.textContent='החזר את ההודעה לעריכה';retry.onclick=()=>{$('message').value=active.prompt;resizeInput();updateControls();$('message').focus();};active.meta.append(retry);$('runStatus').textContent=status==='cancelled'?'נעצרה לבקשתך':'השיחה נשמרה · אפשר לבחור כלי אחר';
    }
    const tool=state.tools.find(t=>t.id===active.provider);if(tool)tool.lastStatus=status;
    state.active=null;state.busy=false;pending.clear();renderTools();if(scroll)scrollEnd(true);remember();
  }
  $('connect').onclick=async()=>{if(state.provider==='deepseek'){notice('DeepSeek עובד עם מפתח API ויתרה בחשבון. המפתח הקיים של AI Terminals נקרא אוטומטית, אם הוא זמין.');return;}try{const action=await invoke('open_connected_login',{tool:state.provider});if(action?.kind==='install'){notice('נפתח עמוד העזרה הרשמי. אחרי ההתקנה חזור לכאן; הכלי יזוהה מחדש ואפשר יהיה להתחבר לחשבון.');return;}notice(state.provider==='ollama'?'נפתח חלון בדיקת המודלים המקומיים. אחרי הבדיקה אפשר לחזור ולהמשיך כאן.':'נפתח חלון ההתחברות הרשמי. השלם בו את ההתחברות וחזור לכאן.');}catch(error){notice(readableError(error));}};
  const dialog=$('settingsDialog');
  let fallbackBackdrop=null;
  function closeSettings() {
    if(dialog.dataset.fallback)dialog.removeAttribute('open');else if(dialog.close)dialog.close();else dialog.removeAttribute('open');
    if(fallbackBackdrop){fallbackBackdrop.remove();fallbackBackdrop=null;document.querySelector('.shell').removeAttribute('aria-hidden');delete dialog.dataset.fallback;}
    $('settings').focus();
  }
  $('settings').onclick=()=>{
    if(dialog.hasAttribute('open'))return;
    if(dialog.showModal){dialog.showModal();return;}
    fallbackBackdrop=document.createElement('div');fallbackBackdrop.className='dialog-shade';fallbackBackdrop.onclick=closeSettings;document.body.append(fallbackBackdrop);
    dialog.dataset.fallback='true';dialog.setAttribute('open','');dialog.setAttribute('aria-modal','true');document.querySelector('.shell').setAttribute('aria-hidden','true');$('closeSettings').focus();
  };
  $('closeSettings').onclick=closeSettings;
  dialog.addEventListener('keydown',event=>{
    if(!dialog.dataset.fallback)return;
    if(event.key==='Escape'){event.preventDefault();closeSettings();}
    if(event.key==='Tab'){
      const controls=Array.from(dialog.querySelectorAll('button,input,select')).filter(el=>!el.disabled);
      const first=controls[0],last=controls[controls.length-1];
      if(event.shiftKey&&document.activeElement===first){event.preventDefault();last.focus();}
      else if(!event.shiftKey&&document.activeElement===last){event.preventDefault();first.focus();}
    }
  });
  dialog.addEventListener('click',event=>{if(event.target===dialog&&dialog.getBoundingClientRect){const r=dialog.getBoundingClientRect();if(event.clientX<r.left||event.clientX>r.right||event.clientY<r.top||event.clientY>r.bottom)closeSettings();}});
  $('economy').onchange=()=>{state.economy=$('economy').checked;updateControls();remember();};
  $('classic').onclick=async()=>{try{await invoke('open_classic_window');}catch(error){notice(String(error));}};
  let theme='system';try{theme=localStorage.getItem('box-lite-theme')||'system';}catch{}
  const media=window.matchMedia?window.matchMedia('(prefers-color-scheme: dark)'):null;
  function applyTheme(){document.documentElement.dataset.theme=theme==='system'?(media?.matches?'dark':'light'):theme;}
  $('theme').value=theme;$('theme').onchange=()=>{theme=$('theme').value;try{localStorage.setItem('box-lite-theme',theme);}catch{}applyTheme();};media?.addEventListener?.('change',applyTheme);applyTheme();
  async function init() {
    state.loading=true;updateControls();$('previewBanner').hidden=!preview;
    const mac=/Mac/.test(navigator.platform||'');document.querySelector('.shortcut').textContent=mac?'⌘ N':'Ctrl N';
    try {
      const bootstrap=await invoke('lite_bootstrap');
      state.projects=bootstrap.projects;state.projectId=bootstrap.projectId;state.tools=bootstrap.tools;state.provider=names[bootstrap.provider]?bootstrap.provider:(state.tools.find(t=>t.installed&&t.id!=='deepseek')?.id||'claude');state.economy=bootstrap.economy!==false;$('economy').checked=state.economy;
      renderProjects();renderTools();$('platformInfo').textContent=preview?'תצוגה מקדימה · ללא חיבור לשירותים':`The Box Lite · ${bootstrap.platform} ${bootstrap.arch}`;
      const conversations=await invoke('db_list_conversations',{projectId:state.projectId});
      state.loading=false;
      const previous=conversations.find(c=>c.id===bootstrap.conversationId);
      if(previous)await loadConversation(previous.id);else await renderConversations();
      await remember();
    }catch(error){notice('לא ניתן לטעון את סביבת העבודה. '+String(error));}
    finally{state.loading=false;updateControls();}
  }
  let refreshedAt=0;
  window.addEventListener('focus',async()=>{
    if(preview||state.busy||state.loading||Date.now()-refreshedAt<2000)return;
    refreshedAt=Date.now();
    try{const latest=await invoke('lite_bootstrap');state.tools=latest.tools;renderTools();}catch{}
  });
  init();
})();
