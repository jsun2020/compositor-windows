// Windows codec checks against reviewed fixtures, with a candidate source receipt.
const fs=require('node:fs'),path=require('node:path'),cp=require('node:child_process'),crypto=require('node:crypto'),assert=require('node:assert/strict');
const root=path.resolve(__dirname,'..'),arg=k=>{const i=process.argv.indexOf(k);assert(i>=0&&process.argv[i+1],'Missing '+k);return process.argv[i+1];};
assert.equal(process.platform,'win32');
const read=p=>JSON.parse(fs.readFileSync(p,'utf8').replace(/^\uFEFF/,'')),sha=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const assets=read(path.resolve(arg('--assets'))),receiptFile=path.resolve(arg('--source-receipt')),receipt=read(receiptFile),out=path.resolve(arg('--out'));
assert.equal(receipt.passed,true);assert.equal(receipt.version,'0.12.0');assert(!fs.existsSync(out),'Preserve previous evidence');
const inputs=read(path.join(path.dirname(receiptFile),'inputs-before.json')),fixtureDir=assets.native157Fixtures,fixtureReceipt=path.join(fixtureDir,'source-receipt.json'),fixtures=read(fixtureReceipt).files;
assert(path.isAbsolute(assets.python)&&fs.existsSync(assets.python),'Explicit verified Python executable required');assert(fixtures.length>0);
const fixtureHash=sha(fixtureReceipt);
function verify(){assert.equal(sha(fixtureReceipt),fixtureHash);for(const f of inputs)assert.equal(sha(path.join(root,f.path)),f.sha256,f.path);for(const f of fixtures){const p=path.resolve(fixtureDir,f.name);assert(p.startsWith(path.resolve(fixtureDir)+path.sep));assert.equal(fs.statSync(p).size,f.bytes);assert.equal(sha(p),f.sha256.toLowerCase(),f.name);}}
verify();fs.mkdirSync(out,{recursive:true});
const steps=[];let error=null,passed=false,child=null;const env={...process.env,PERF:'',VITE_TEST_API:'',VITE_BRIDGE:'',GPU_WARP_PERF:'',GPU_WARP_UI_PERF:''};
const key=Object.keys(env).find(k=>k.toLowerCase()==='path')||'Path';env[key]=path.join(process.env.USERPROFILE,'.cargo/bin')+';C:\\Program Files\\LLVM\\bin;'+env[key];
async function run(name,exe,args,extra={},limit=1800000){const fd=fs.openSync(path.join(out,name+'.log'),'wx');const started=new Date().toISOString();let timer,timedOut=false;console.log('[codecs]',name);let r;
 try{r=await new Promise(resolve=>{child=cp.spawn(exe,args,{cwd:root,env:{...env,...extra},stdio:['ignore',fd,fd],windowsHide:true});child.once('error',e=>resolve({exit:null,error:String(e)}));child.once('close',(exit,signal)=>resolve({exit,signal}));timer=setTimeout(()=>{timedOut=true;if(child.pid&&child.exitCode===null)cp.spawnSync('taskkill.exe',['/PID',String(child.pid),'/T','/F'],{windowsHide:true,stdio:'ignore'});},limit);});}finally{clearTimeout(timer);fs.closeSync(fd);child=null;}
 const record={name,command:[exe,...args],started,finished:new Date().toISOString(),timedOut,...r};steps.push(record);fs.writeFileSync(path.join(out,name+'.exit.json'),JSON.stringify(record,null,2));assert.equal(r.exit,0,name);assert(!timedOut);verify();
}
(async()=>{try{
 const helper=path.join(out,'heif-runtime');
 await run('heif-build-arguments','powershell.exe',['-NoProfile','-ExecutionPolicy','Bypass','-File','scripts/tests/heif-build-arguments.ps1']);
 await run('heif-build','powershell.exe',['-NoProfile','-ExecutionPolicy','Bypass','-File','scripts/build-heif-helper.ps1','-OutputDirectory',helper]);
 await run('heif-independent-pixels',assets.python,['-X','utf8','scripts/verify-heif-helper.py',path.join(helper,'CompositorHeif.exe'),path.join(out,'heif-pixels')]);assert.equal(read(path.join(out,'heif-pixels/result.json')).passed,true);
 await run('real-windows-codecs','cargo.exe',['test','--locked','-p','compositor-shell','--lib','native_acceptance157::windows157_real_codecs_and_icc','--','--ignored','--exact','--nocapture'],{COMPOSITOR_HEIF_HELPER:path.join(helper,'CompositorHeif.exe'),COMPOSITOR_NATIVE157_FIXTURES:fixtureDir,COMPOSITOR_NATIVE157_OUTPUT:path.join(out,'system-codecs')});assert.equal(read(path.join(out,'system-codecs/result.json')).passed,true);
 await run('independent-color-readers',assets.python,['-X','utf8','scripts/verify-mac157-color-output.py',path.join(out,'system-codecs'),fixtureDir]);
 verify();passed=true;
}catch(e){error=String(e.stack||e);console.error(error);}finally{fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({scope:'Real Windows PDF/HEIF/ICC/CMYK components; production native UI remains separate',passed,error,steps,sourceReceiptSHA256:sha(receiptFile),fixtureReceiptSHA256:fixtureHash,finished:new Date().toISOString()},null,2)+'\n');process.exitCode=passed?0:1;}})();
