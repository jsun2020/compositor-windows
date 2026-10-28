// Explicit Windows browser/performance stages. Original assertions, one worker, zero retries.
const fs=require('node:fs'),path=require('node:path'),cp=require('node:child_process'),crypto=require('node:crypto'),assert=require('node:assert/strict');
const root=path.resolve(__dirname,'..');
const arg=k=>{const i=process.argv.indexOf(k);assert(i>=0&&process.argv[i+1],'Missing '+k);return process.argv[i+1];};
const out=path.resolve(root,arg('--out')),stage=arg('--stage');assert(['verify','source','functional','cpu','gpu'].includes(stage));
const read=p=>JSON.parse(fs.readFileSync(p,'utf8').replace(/^\uFEFF/,''));const sha=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');const write=(p,x)=>fs.writeFileSync(p,JSON.stringify(x,null,2)+'\n',{flag:'wx'});const manifest=()=>read(path.join(out,'inputs.json'));const pause=ms=>new Promise(r=>setTimeout(r,ms));
const cpu=['perf-4-5','perf-4b1','perf-brush-retouch','perf-selection','perf-spatial'].map(n=>`app/tests/e2e/${n}.spec.ts`),gpu=['perf-gpu-warp','perf-warp-live'].map(n=>`app/tests/e2e/${n}.spec.ts`);
let env,gates;
function verify(){
 const m=manifest();assert.equal(m.workers,1);assert.equal(m.retries,0);assert.equal(m.upstreamCommit,'9959c32d77330ec8d7b42a49353d903283ecb8e6');
 for(const x of m.inputs){const p=path.resolve(root,x.path);assert(p.startsWith(root+path.sep));assert.equal(fs.statSync(p).size,x.bytes,x.path);assert.equal(sha(p),x.sha256,x.path);}
 for(const x of m.external){assert.equal(fs.statSync(x.path).size,x.bytes,x.path);assert.equal(sha(x.path),x.sha256,x.path);}
 for(const p of [...cpu,...gpu]){const norm=s=>s.replace(/\r\n/g,'\n');assert.equal(norm(fs.readFileSync(path.join(root,p),'utf8')),norm(cp.execFileSync('git',['show',`2b328db1343cfa258058f65600207005becbe487:${p}`],{cwd:root,encoding:'utf8'})),'Original performance test changed: '+p);}
 assert.equal(cp.execFileSync('git',['rev-parse','HEAD'],{cwd:root,encoding:'utf8'}).trim(),m.sourceCommit);
}
function runtimeRows(){const walk=p=>fs.readdirSync(p,{withFileTypes:true}).flatMap(e=>e.isDirectory()?walk(path.join(p,e.name)):[path.join(p,e.name)]);return [...walk(path.join(out,'dist')),...walk(path.join(root,'app/src/engine/pkg'))].map(p=>({path:path.relative(root,p),bytes:fs.statSync(p).size,sha256:sha(p)}));}
function verifyBuilt(){const r=read(path.join(out,'source/result.json'));assert.equal(r.passed,true);for(const x of r.runtime){const p=path.join(root,x.path);assert.equal(fs.statSync(p).size,x.bytes,x.path);assert.equal(sha(p),x.sha256,x.path);}assert.equal(r.componentSHA256,sha(manifest().sourceFile));assert.equal(read(manifest().sourceFile).passed,true);}
async function command(exe,args,log,extra={}){const fd=fs.openSync(log,'wx');let child,timer;try{return await new Promise((resolve,reject)=>{child=cp.spawn(exe,args,{cwd:root,env:{...env,...extra},stdio:['ignore',fd,fd],windowsHide:true});child.once('error',reject);child.once('close',code=>resolve(code));timer=setTimeout(()=>{if(child.pid&&child.exitCode===null)cp.spawnSync('taskkill.exe',['/PID',String(child.pid),'/T','/F'],{stdio:'ignore',windowsHide:true});reject(new Error('Command timeout'));},600000);});}finally{clearTimeout(timer);fs.closeSync(fd);}}
async function source(){verify();const d=path.join(out,'source');assert(!fs.existsSync(d));fs.mkdirSync(d);const started=new Date().toISOString();let error=null,exit=null,runtime=[];try{
 assert.equal(read(manifest().sourceFile).passed,true);
 exit=await command(process.execPath,['node_modules/vite/bin/vite.js','build','--outDir',path.join(out,'dist')],path.join(d,'frontend-build.log'),{VITE_BRIDGE:'mock',VITE_TEST_API:'1'});assert.equal(exit,0);verify();runtime=runtimeRows();
 const wasm=runtime.filter(x=>x.path.endsWith('.wasm')&&x.path.includes('dist'));assert.equal(wasm.length,1);assert.equal(wasm[0].sha256,sha(path.join(root,'app/src/engine/pkg/compositor_engine_bg.wasm')));
 }catch(e){error=String(e.stack||e);}write(path.join(d,'result.json'),{scope:'Verified source-component receipt plus instrumented browser build',started,finished:new Date().toISOString(),exit,error,runtime,componentSHA256:sha(manifest().sourceFile),manifestSHA256:sha(path.join(out,'inputs.json')),passed:error===null,productionPortable:false});if(error)throw Error(error);}
function desktop(g) {
  const s = JSON.parse(cp.execFileSync(g.exe, ['check'], {encoding: 'utf8'}).trim());
  assert.equal(s.success, true); assert.equal(s.unlocked, true, 'Windows desktop must be unlocked.');
  if (g.display) assert.equal(s.displayCount, 1, 'Exactly one display required.');
  return s;
}

async function hardware(kind) {
  verify(); verifyBuilt();
  const d = path.join(out, kind);
  assert(!fs.existsSync(d), `${kind} evidence already exists; no automatic retry.`);
  const preflight = gates.map(desktop);
  const busy = cp.execFileSync('powershell.exe', ['-NoProfile', '-Command', "$ErrorActionPreference='Stop'; @(Get-CimInstance Win32_Process | Where-Object {$_.Name -in @('cargo.exe','rustc.exe','link.exe','clang.exe','llvm-ar.exe')}).Count"], {env, encoding: 'utf8'}).trim();
  assert.equal(busy, '0', 'No concurrent compilers allowed.');
  fs.mkdirSync(d);
  const started = new Date().toISOString();
  let child, invalid = null, timer, watchdog, exit = null, error = null, childFinished = null;
  const monitors = [], summaries = [];
  const fd = fs.openSync(path.join(d, 'run.log'), 'wx');
  const stopTree = () => { if (child && child.exitCode === null) cp.spawnSync('taskkill.exe', ['/PID', String(child.pid), '/T', '/F'], {stdio: 'ignore', windowsHide: true}); };
  const samples = g => {
    if (!fs.existsSync(g.file)) return [];
    const input = fs.openSync(g.file, 'r');
    let text;
    try {
      const size = fs.fstatSync(input).size, start = Math.max(0, size - 8192), buffer = Buffer.alloc(size - start);
      fs.readSync(input, buffer, 0, buffer.length, start); text = buffer.toString('utf8');
    } finally { fs.closeSync(input); }
    return text.replace(/^\uFEFF/, '').split(/\r?\n/).filter(Boolean).flatMap(line => {try { return [JSON.parse(line)]; } catch { return []; }});
  };
  const inspect = () => {
    if (invalid) return;
    for (const g of monitors) {
      const rows = samples(g), last = rows.at(-1);
      const bad = rows.find(s => s.success !== true || s.unlocked !== true || (g.display && s.displayCount !== 1));
      if (bad || g.exited || (Date.now() - Date.parse(last?.utc ?? started) > 5000)) {
        invalid = {monitor: g.name, sample: bad ?? last ?? null, reason: bad ? 'invalid state' : g.exited ? 'observer exited early' : 'observer missing/stale'};
        stopTree(); return;
      }
    }
  };
  console.log(`[${kind}] Starting original ${kind === 'functional' ? manifest().functionalCases : kind === 'cpu' ? 29 : 4} cases; keep one display unlocked.`);
  try {
    const capture = manifest().assets.captureEnvironment;
    const beforeExit = await command('powershell.exe', ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', capture, '-Output', path.join(d, 'environment-before.json')], path.join(d, 'environment-before.log'));
    assert.equal(beforeExit, 0);
    const environment = read(path.join(d, 'environment-before.json'));
    assert.equal(environment.screens.length, 1); assert.equal(environment.activeCompilers.length, 0);
    assert.equal(environment.ownedNativeDebugPortStillListening, false);
    for (const g of gates) {
      const m = {...g, file: path.join(d, `${g.name}.jsonl`), exited: false};
      m.process = cp.spawn(g.exe, ['monitor', m.file, '1800'], {stdio: 'ignore', windowsHide: true});
      m.done = new Promise(resolve => { m.process.once('exit', code => {m.exited = true; resolve(code);}); m.process.once('error', e => {m.exited = true; m.error = String(e); resolve(null);}); });
      monitors.push(m);
    }
    await pause(700); inspect(); assert.equal(invalid, null, 'Observer did not start correctly.');
    const files = kind === 'functional' ? [] : kind === 'cpu' ? cpu : gpu;
    const args = ['node_modules/@playwright/test/cli.js', 'test', ...files, '--config', path.join(out, `${kind}.config.ts`), '--workers', '1', '--retries', '0', '--reporter', 'line,json', '--output', path.join(d, 'test-results')];
    child = cp.spawn(process.execPath, args, {cwd: root, env: {...env, PERF: kind === 'cpu' ? '1' : '', GPU_WARP_PERF: kind === 'gpu' ? '1' : '', GPU_WARP_UI_PERF: kind === 'gpu' ? '1' : '', PLAYWRIGHT_JSON_OUTPUT_NAME: path.join(d, 'details.json')}, stdio: ['ignore', fd, fd], windowsHide: true});
    timer = setInterval(inspect, 500);
    watchdog=setTimeout(()=>{invalid={reason:'Workload exceeded 30 minute controller bound'};stopTree();},1800000);
    exit = await new Promise((resolve, reject) => {child.once('exit', resolve); child.once('error', reject);});
    childFinished = Date.now();
    const deadline = Date.now() + 5000;
    while (Date.now() < deadline && monitors.some(g => Date.parse(samples(g).at(-1)?.utc ?? '') < childFinished || !samples(g).length)) await pause(100);
    inspect();
    for (const g of monitors) assert(Date.parse(samples(g).at(-1)?.utc ?? '') >= childFinished, 'Observer did not cover the end of the workload.');
    const afterExit = await command('powershell.exe', ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', capture, '-Output', path.join(d, 'environment-after.json')], path.join(d, 'environment-after.log'));
    assert.equal(afterExit, 0);
    verify(); verifyBuilt();
  } catch (e) { error = String(e.stack || e); stopTree(); }
  finally {
    clearInterval(timer); clearTimeout(watchdog); fs.closeSync(fd);
    for (const g of monitors) fs.writeFileSync(g.file + '.stop', 'Stop owned read-only observer');
    for (const g of monitors) {
      const code = await Promise.race([g.done, pause(10000).then(() => 'TIMEOUT')]);
      if (code === 'TIMEOUT') g.process.kill();
      const summary = fs.existsSync(g.file + '.summary.json') ? read(g.file + '.summary.json') : null;
      summaries.push({name: g.name, exit: code, summary});
    }
  }
  const stats = fs.existsSync(path.join(d, 'details.json')) ? read(path.join(d, 'details.json')).stats : null;
  const healthy = !error && !invalid && summaries.length === 2 && summaries.every(s => s.exit === 0 && s.summary?.samples > 0 && s.summary.allSamplesUnlocked === true && (s.name !== 'display' || s.summary.allSamplesSingleDisplay === true));
  const passed = healthy && exit === 0 && stats?.expected === (kind === 'functional' ? manifest().functionalCases : kind === 'cpu' ? 29 : 4) && stats.unexpected === 0 && stats.skipped === (kind === 'functional' ? 33 : 0) && stats.flaky === 0;
  const result = {scope: `CURRENT_COVERED_CLEAR_ORIGINAL_${kind.toUpperCase()}`, started, finished: new Date().toISOString(), childFinished, exit, error, invalid, preflight, summaries, stats, healthy, passed, workers: 1, retries: 0, originalBudgetsAndAssertionsUnchanged: true, manifestSHA256: sha(path.join(out, 'inputs.json')), productionPortable: false};
  write(path.join(d, 'result.json'), result);
  console.log(`[${kind}] ${passed ? 'PASS' : 'NOT PASSED'}: ${stats?.expected ?? '?'} passed / ${stats?.unexpected ?? '?'} failed. Evidence: ${d}`);
  if (!healthy) throw Error('Hardware environment or controller failed. Evidence retained; no retry.');
  return passed;
}


async function main(){process.chdir(root);verify();if(stage==='verify'){console.log('Frozen inputs and original performance budgets match');return;}assert.equal(process.platform,'win32');
 env={...process.env,PERF:'',GPU_WARP_PERF:'',GPU_WARP_UI_PERF:'',VITE_BRIDGE:'',VITE_TEST_API:'',npm_package_version:read(path.join(root,'package.json')).version};
 env.PSModulePath=`${process.env.USERPROFILE}\\Documents\\WindowsPowerShell\\Modules;C:\\Program Files\\WindowsPowerShell\\Modules;C:\\Windows\\system32\\WindowsPowerShell\\v1.0\\Modules`;
 const assets=manifest().assets;gates=[{name:'display',display:true,exe:assets.displayGate},{name:'session',display:false,exe:assets.sessionGate}];
 const lease=path.join(root,'build-artifacts','.mac1511-stage.lock');fs.mkdirSync(path.dirname(lease),{recursive:true});write(lease,{pid:process.pid,stage,out,started:new Date().toISOString()});
 try{if(stage==='source')await source();else{const prerequisite=stage==='cpu'?'functional':stage==='gpu'?'cpu':null;if(prerequisite)assert.equal(read(path.join(out,prerequisite,'result.json')).passed,true,'Previous stage must pass');process.exitCode=await hardware(stage)?0:1;}}
 finally{if(read(lease).pid===process.pid)fs.unlinkSync(lease);}
}
if(require.main===module)main().catch(e=>{console.error(e.stack||e);process.exitCode=2;});
