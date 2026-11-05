const {spawnSync}=require('node:child_process');
const path=require('node:path'),os=require('node:os'),assert=require('node:assert/strict');
let binary=path.resolve(process.argv[2]);
if(process.platform==='win32'&&!binary.endsWith('.exe'))binary+='.exe';
const tests=[
 {tool:'not-a-provider',project:os.tmpdir(),expected:'Unknown tool'},
 {tool:'claude',project:path.join(os.tmpdir(),'box-missing-'+Date.now()),expected:'Project folder not found'},
];
for(const test of tests){
 const result=spawnSync(binary,['--box-worker'],{input:JSON.stringify({...test,prompt:'Startup check only'}),encoding:'utf8',windowsHide:true,timeout:20000});
 if(result.error)throw result.error;
 assert.equal(result.status,1,result.stderr);
 const events=result.stdout.trim().split(/\r?\n/).map(line=>JSON.parse(line));
 assert(events.some(e=>e.type==='error'&&e.message.includes(test.expected)),result.stdout);
 assert.equal(events.at(-1).exit_status,'error');
 assert(!events.some(e=>e.type==='text_chunk'));
}
console.log('PASS: production executable enters worker mode, returns valid JSON and rejects invalid requests before any account or network call.');
