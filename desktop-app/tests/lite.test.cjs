const {JSDOM}=require('jsdom');
const fs=require('fs'),assert=require('assert/strict');
const root=require('path').resolve(__dirname,'../src')+require('path').sep;
const pause=()=>new Promise(r=>setTimeout(r,25));
const saved={provider:'codex',projectId:'p1',conversationId:null,economy:true};
const conversations=[];const turns=[];const calls=[];let hold=false;let complete;
let missing=false;
async function load(){
 const dom=new JSDOM(fs.readFileSync(root+'lite.html','utf8'),{url:'http://lite.test',runScripts:'outside-only'});
 const w=dom.window;let listener;
 w.__TAURI__={event:{listen:async(name,fn)=>{listener=fn;}},core:{invoke:async(cmd,args={})=>{
  calls.push({cmd,args});
  switch(cmd){
   case 'lite_bootstrap':return {...saved,projects:[{id:'p1',name:'פרויקט אישי',path:'C:/project'}],tools:['claude','codex','gemini','grok','ollama','deepseek'].map(id=>({id,installed:!missing})),platform:'windows',arch:'x86_64'};
   case 'save_lite_preferences':Object.assign(saved,args);return;
   case 'db_list_conversations':return conversations;
   case 'db_create_conversation':{const c={id:'c'+(conversations.length+1),title:args.title};conversations.push(c);return c;}
   case 'db_create_turn':{const t={id:'t'+(turns.length+1),seq:turns.length+1,userIntent:args.userIntent,status:'running',reply:'',conversationId:args.conversationId};turns.push(t);return t;}
   case 'lite_history':return {items:turns.filter(t=>t.conversationId===args.conversationId),hasMore:false};
   case 'run_connected_chat':{
    const id='r'+turns.length;const t=turns.find(t=>t.id===args.turnId);
    const done=()=>{
     if(args.tool==='deepseek'){
      t.status='failed';listener({payload:{runId:id,event:{type:'error',message:'HTTP 402 Payment Required'}}});listener({payload:{runId:id,event:{type:'done',exit_status:'error'}}});
     }else{
      t.status='completed';t.reply='תשובה <img src=x onerror="window.bad=true">\n```js\nconst answer = 42;\n```';
      listener({payload:{runId:id,event:{type:'text_chunk',text:t.reply}}});listener({payload:{runId:id,event:{type:'done',exit_status:'success'}}});
     }
    };
    if(hold){complete=done;setTimeout(()=>{},0);}else done();
    return id;
   }
   case 'cancel_run':return;
   case 'open_connected_login':return {kind:missing?'install':'login'};
   default:throw Error('Unexpected '+cmd);
  }
 }}};
 w.eval(fs.readFileSync(root+'lite.js','utf8'));await pause();await pause();return {dom,w,d:w.document};
}
async function send(app,text){app.d.getElementById('message').value=text;app.d.getElementById('message').dispatchEvent(new app.w.Event('input'));app.d.getElementById('composer').dispatchEvent(new app.w.Event('submit',{cancelable:true}));await pause();await pause();}
(async()=>{
 let app=await load();
 assert.equal(app.d.getElementById('project').value,'p1');assert.equal(app.d.getElementById('provider').value,'codex');
 await send(app,'הסבר לי את הקוד <script>window.bad=true</script>');
 assert.equal(app.d.querySelector('.message.user').textContent,'הסבר לי את הקוד <script>window.bad=true</script>');
 assert.equal(app.d.querySelectorAll('#messages img, #messages script').length,0);
 assert.equal(app.d.querySelector('#messages pre code').textContent,'const answer = 42;\n');
 assert.equal(app.d.getElementById('message').disabled,false);
 assert.equal(calls.filter(c=>c.cmd==='run_connected_chat').length,1);assert(!calls.some(c=>c.cmd==='generate_task_plan'));
 assert.equal(saved.conversationId,'c1');
 app.dom.window.close();app=await load();
 assert(app.d.getElementById('messages').textContent.includes('const answer = 42;'));
 assert(app.d.querySelector('.message.user').textContent.includes('הסבר לי'));
 assert.equal(app.d.querySelector('.conversation[aria-current=true]').textContent,conversations[0].title);
 app.d.getElementById('provider').value='deepseek';app.d.getElementById('provider').dispatchEvent(new app.w.Event('change'));
 await send(app,'בדיקת חיבור');assert(app.d.getElementById('messages').textContent.includes('אין יתרה זמינה'));assert.equal(app.d.getElementById('message').disabled,false);
 app.d.querySelector('.retry').click();assert.equal(app.d.getElementById('message').value,'בדיקת חיבור');
 app.d.getElementById('provider').value='ollama';app.d.getElementById('provider').dispatchEvent(new app.w.Event('change'));
 hold=true;await send(app,'בדיקת המתנה');assert.equal(app.d.getElementById('project').disabled,true);assert.equal(app.d.getElementById('newChat').disabled,true);
 assert.equal(app.d.getElementById('message').disabled,false);app.d.getElementById('message').value='טיוטה להמשך';
 complete();await pause();assert.equal(app.d.getElementById('project').disabled,false);assert.equal(app.d.getElementById('message').value,'טיוטה להמשך');
 app.d.getElementById('economy').checked=false;app.d.getElementById('economy').dispatchEvent(new app.w.Event('change'));await pause();assert.equal(saved.economy,false);
 app.dom.window.close();app=await load();assert.equal(app.d.getElementById('provider').value,'ollama');assert.equal(app.d.getElementById('economy').checked,false);
 app.d.getElementById('newChat').click();await pause();assert.equal(saved.conversationId,null);assert.equal(app.d.getElementById('welcome').hidden,false);
 const dialog=app.d.getElementById('settingsDialog');Object.defineProperty(dialog,'showModal',{value:undefined});
 app.d.getElementById('settings').click();assert(dialog.hasAttribute('open'));assert.equal(app.d.querySelector('.shell').getAttribute('aria-hidden'),'true');
 dialog.dispatchEvent(new app.w.KeyboardEvent('keydown',{key:'Escape',bubbles:true}));assert(!dialog.hasAttribute('open'));assert.equal(app.d.querySelector('.shell').getAttribute('aria-hidden'),null);
 app.dom.window.close();
 console.log('PASS: real camelCase history, persisted provider/project/conversation/economy, single request, event race, failure/retry, busy navigation guards, inert user/model markup, code blocks and new chat.');
 const demo=new JSDOM(fs.readFileSync(root+'lite.html','utf8'),{url:'http://preview.test',runScripts:'outside-only'});demo.window.eval(fs.readFileSync(root+'lite.js','utf8'));await pause();await pause();assert.equal(demo.window.document.getElementById('previewBanner').hidden,false);assert.equal(demo.window.document.getElementById('connection').textContent,'תצוגה מקדימה');demo.window.close();
 console.log('PASS: browser preview clearly distinguished from live tool connections.');
 missing=true;saved.provider='codex';app=await load();
 assert(app.d.getElementById('connect').textContent.includes('עזרה והתקנה'));
 app.d.getElementById('connect').click();await pause();assert(app.d.body.textContent.includes('עמוד העזרה הרשמי'));
 app.d.getElementById('message').value='טיוטת התקנה';missing=false;
 app.w.dispatchEvent(new app.w.Event('focus'));await pause();await pause();
 assert.equal(app.d.getElementById('connection').textContent,'מזוהה במחשב');assert.equal(app.d.getElementById('message').value,'טיוטת התקנה');
 assert.equal(app.d.getElementById('provider').value,'codex');app.dom.window.close();
 console.log('PASS: missing tool setup help and re-detection on return preserve provider and draft.');
})().catch(error=>{console.error(error);process.exitCode=1;});
