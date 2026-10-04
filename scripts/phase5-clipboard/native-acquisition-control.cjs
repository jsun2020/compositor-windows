const {chromium,expect}=require('@playwright/test');
const fs=require('node:fs'),path=require('node:path'),cp=require('node:child_process'),assert=require('node:assert/strict');
const root=path.resolve(process.argv[2]),fixed=process.argv[3]==='fixed';
const helpers=process.env.COMPOSITOR_QA_HELPER_DIRECTORY||__dirname;
const marker=process.env.COMPOSITOR_QA_MARKER||'COMPOSITOR_BUILD_0.8.0_20261004-1527';
let unlocked=null;const cases=[];const record=(completed,error)=>fs.writeFileSync(path.join(root,'native-acquisition-control.json'),JSON.stringify({diagnosticOnly:true,fixedBackend:fixed,systemClipboardMutation:false,originalTestAssertionsAndRetriesUnchanged:true,completed,error:error?String(error):null,unlocked,cases},null,2));
const delay=ms=>new Promise(r=>setTimeout(r,ms));
const json=file=>JSON.parse(fs.readFileSync(file,'utf8').replace(/^\uFEFF/,''));
(async()=>{
 const b=await chromium.connectOverCDP(process.env.COMPOSITOR_QA_CDP_ENDPOINT);let locker;
 try{
  await expect.poll(()=>b.contexts()[0]?.pages().some(p=>p.url().includes('tauri')),{timeout:15000}).toBe(true);
  const p=b.contexts()[0].pages().find(p=>p.url().includes('tauri'));await expect(p.getByTestId('engine-ready')).toContainText(marker);
  assert.deepEqual(await p.evaluate(()=>({native:'__TAURI_INTERNALS__'in window,testApi:typeof window.__compositor})),{native:true,testApi:'undefined'});
  const read=()=>p.evaluate(async()=>{const started=Date.now();try{const value=await window.__TAURI_INTERNALS__.invoke('read_clipboard_image');return {ok:true,bytes:value?.byteLength??value?.length??null,startedUtc:new Date(started).toISOString(),finishedUtc:new Date().toISOString(),elapsedMs:Date.now()-started};}catch(e){return {ok:false,error:String(e),startedUtc:new Date(started).toISOString(),finishedUtc:new Date().toISOString(),elapsedMs:Date.now()-started};}});
  unlocked=await read();record(false);assert.ok(unlocked.ok||unlocked.error==='The clipboard does not contain an image','Unlocked clipboard did not open');
  for(const [name,hold] of [['short',160],['persistent',500]]){
   const dir=path.join(root,name);fs.mkdirSync(dir);const started=Date.now();
   locker=cp.spawn(path.join(helpers,'ClipboardReadOnlyLocker.exe'),[dir,String(hold)],{windowsHide:true,stdio:'ignore'});
   let code=null,spawnError=null;locker.on('exit',n=>code=n);locker.on('error',e=>spawnError=e);
   while(!fs.existsSync(path.join(dir,'lock-ready.json'))&&Date.now()-started<10000){if(spawnError)throw spawnError;if(code!==null)throw Error('Locker exited before ready');await delay(5);}
   assert.ok(fs.existsSync(path.join(dir,'lock-ready.json')),'Read-only lock not ready');
   const ready=json(path.join(dir,'lock-ready.json'));const result=await read();
   while(code===null&&Date.now()-started<15000)await delay(5);assert.equal(code,0);locker=null;
   const closed=json(path.join(dir,'lock-closed.json'));assert.equal(closed.sequenceBefore,closed.sequenceAfter,'Read-only control changed clipboard');
   cases.push({name,holdMs:hold,locker:ready,result,closed,clipboardSequenceUnchanged:true});record(false);
   if(!fixed){assert.equal(result.ok,false);assert.match(result.error,/Windows denied clipboard access/);assert.ok(Date.parse(result.finishedUtc)<Date.parse(closed.utc),'Original refusal did not occur while lock held');}
   else if(name==='short'){assert.equal(result.ok,unlocked.ok);if(!unlocked.ok)assert.equal(result.error,unlocked.error);assert.ok(result.elapsedMs>=50,'Short contention was not waited out');}
   else{assert.equal(result.ok,false);assert.match(result.error,/Windows clipboard is busy or unavailable/);assert.ok(result.elapsedMs>=200&&result.elapsedMs<1000,'Acquisition deadline was not bounded');assert.ok(Date.parse(result.finishedUtc)<Date.parse(closed.utc),'Persistent error arrived after lock release');}
  }
  record(true);console.log(JSON.stringify({fixedBackend:fixed,cases:cases.map(x=>({name:x.name,elapsedMs:x.result.elapsedMs,error:x.result.error??null}))}));
 }catch(e){record(false,e);throw e;}finally{if(locker)locker.kill();await Promise.race([b.close(),delay(5000)]);}
})().then(()=>process.exit(0)).catch(e=>{console.error(e);process.exit(1);});
