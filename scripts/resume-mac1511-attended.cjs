// Visible-terminal attendance. No injected input, keep-awake or policy changes.
const fs=require('node:fs'),path=require('node:path'),cp=require('node:child_process'),readline=require('node:readline'),assert=require('node:assert/strict');
const root=path.resolve(__dirname,'..');
const value=k=>{const i=process.argv.indexOf(k);return i<0?null:process.argv[i+1];};
assert.equal(process.platform,'win32');assert(process.stdin.isTTY&&process.stdout.isTTY,'Run in a visible interactive Windows terminal; redirected input refused');
const outArg=value('--out');assert(outArg,'Explicit frozen output directory required');const out=path.resolve(root,outArg);
assert(fs.existsSync(path.join(out,'inputs.json')),'Prepare frozen inputs first');
const stages=(value('--stages')||'functional,cpu,gpu,package,native,native157,native1511').split(',');assert(stages.length&&new Set(stages).size===stages.length&&stages.every(s=>['functional','cpu','gpu','package','native','native157','native1511'].includes(s)));
const d=path.join(out,'attended-'+new Date().toISOString().replace(/[:.]/g,'-'));fs.mkdirSync(d);
const write=(name,v)=>fs.writeFileSync(path.join(d,name),JSON.stringify(v,null,2)+'\n',{flag:'wx'});
const events=(type,v={})=>fs.appendFileSync(path.join(d,'events.jsonl'),JSON.stringify({utc:new Date().toISOString(),type,...v})+'\n');
let child=null,pending=null,reminder=null,failed=false,closing=false,currentStage=null;const results=[];
const terminal=readline.createInterface({input:process.stdin,output:process.stdout});
terminal.on('line',()=>{events('actual-terminal-enter',{stage:currentStage,state:child?'running':'waiting'});if(child&&currentStage.startsWith('native')){failed=true;events('unexpected-input-during-native');stopChild();}if(pending){const resolve=pending;pending=null;resolve();}});
function policy(){const script="$ErrorActionPreference='Stop';$base=[Microsoft.Win32.RegistryKey]::OpenBaseKey([Microsoft.Win32.RegistryHive]::CurrentUser,[Microsoft.Win32.RegistryView]::Registry64);$key=$base.OpenSubKey('Software\\Policies\\Microsoft\\Windows\\Control Panel\\Desktop',$false);try{$r=[ordered]@{sid=[Security.Principal.WindowsIdentity]::GetCurrent().User.Value};foreach($name in @('ScreenSaveActive','ScreenSaveTimeOut','ScreenSaverIsSecure')){$v=if($null -eq $key){$null}else{$key.GetValue($name,$null,[Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)};$kind=if($null -eq $v){$null}else{$key.GetValueKind($name).ToString()};$r[$name]=[ordered]@{exists=($null -ne $v);kind=$kind;value=$v}};$r|ConvertTo-Json -Depth 4 -Compress}finally{if($null -ne $key){$key.Dispose()};$base.Dispose()}";return JSON.parse(cp.execFileSync('powershell.exe',['-NoProfile','-Command',script],{encoding:'utf8',windowsHide:true,timeout:10000}));}
function stopChild(){if(child&&child.exitCode===null&&child.pid)cp.spawnSync('taskkill.exe',['/PID',String(child.pid),'/T','/F'],{windowsHide:true,stdio:'ignore'});}
terminal.on('close',()=>{if(!closing){failed=true;stopChild();if(pending){const resolve=pending;pending=null;resolve();}}});
process.on('SIGINT',()=>{failed=true;stopChild();terminal.close();});
async function confirm(stage){console.log(`\n下一阶段：${stage}。保持单屏、桌面解锁，避免重任务。`);console.log(stage.startsWith('native')?'请实际按回车开始；本组最多8分钟，开始后停止一切键鼠操作，直到显示结束。':'请实际按回车开始；运行期间约每4分钟会提醒你在本终端回车。不要操作自动测试页面。');events('waiting-for-human',{stage});let timer;try{await new Promise((resolve,reject)=>{pending=resolve;timer=setTimeout(()=>{pending=null;reject(new Error('No human confirmation within 20 minutes'));},1200000);});}finally{clearTimeout(timer);}assert(!failed,'Terminal closed before stage');}
(async()=>{let error=null,before=null,after=null;try{before=policy();write('policy-before.json',before);
 for(const stage of stages){
  assert(!fs.existsSync(path.join(out,stage)),'Stage already attempted; preserve evidence and request reviewed recovery: '+stage);
  currentStage=stage;await confirm(stage);const log=fs.openSync(path.join(d,stage+'.log'),'wx');events('stage-start',{stage});
  console.log(`[${stage}] 已开始。详细结果会保存到 ${path.join(out,stage)}。`);
  if(!stage.startsWith('native'))reminder=setInterval(()=>console.log('\x07值守提醒：请在本终端实际按一次回车，保持 Windows 解锁；不要操作测试页面。'),240000);
  let result;
  try{result=await new Promise(resolve=>{child=cp.spawn(process.execPath,[path.join(root,['package','native','native157','native1511'].includes(stage)?'scripts/run-mac1511-package.cjs':'scripts/run-mac1511-tests.cjs'),'--out',out,'--stage',stage],{cwd:root,stdio:['ignore',log,log],windowsHide:true});child.once('error',e=>resolve({exit:null,error:String(e)}));child.once('close',(exit,signal)=>resolve({exit,signal}));});}finally{clearInterval(reminder);fs.closeSync(log);child=null;}
  results.push({stage,...result});events('stage-finished',{stage,...result});write(stage+'.exit.json',result);
  assert.equal(result.exit,0,stage+' failed; no automatic retry');assert(!failed);
  const receipt=JSON.parse(fs.readFileSync(path.join(out,stage,'result.json'),'utf8'));assert.equal(receipt.passed,true);
  console.log(`[${stage}] 通过。`);
 }
}catch(e){error=String(e.stack||e);console.error(error);failed=true;}finally{stopChild();clearInterval(reminder);closing=true;terminal.close();try{after=policy();write('policy-after.json',after);assert.deepEqual(after,before);}catch(e){failed=true;error=(error||'')+'\n'+String(e.stack||e);}write('result.json',{passed:!failed,error,results,scope:'Only explicitly requested stages, all with preserved original receipts',lockPolicyChanged:JSON.stringify(before)!==JSON.stringify(after),syntheticAttendance:false});console.log('本窗口阶段结束；证据：'+d);process.exitCode=failed?1:0;}})();
