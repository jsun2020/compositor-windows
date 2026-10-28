// Final local package gate. Publication and Git operations are intentionally separate.
const fs = require('node:fs');
const path = require('node:path');
const cp = require('node:child_process');
const crypto = require('node:crypto');
const assert = require('node:assert/strict');
const {pathToFileURL} = require('node:url');
const root=path.resolve(__dirname,'..');
const arg=k=>{const i=process.argv.indexOf(k);assert(i>=0&&process.argv[i+1],'Missing '+k);return process.argv[i+1];};
const out=path.resolve(root,arg('--out')),acceptance=out;
const read = p => JSON.parse(fs.readFileSync(p, 'utf8').replace(/^\uFEFF/, ''));
const sha = p => crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const save = (p, value) => fs.writeFileSync(p, JSON.stringify(value, null, 2), {flag: 'wx'});
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
let env;

function manifest(){return read(path.join(out,'inputs.json'));}
function frozen(){cp.execFileSync(process.execPath,[path.join(root,'scripts/run-mac1511-tests.cjs'),'--out',out,'--stage','verify'],{cwd:root,stdio:'pipe'});}
function prerequisites() {
  frozen();
  for (const phase of ['source','functional','cpu','gpu']) {
    const result = read(path.join(acceptance, phase, 'result.json'));
    assert.equal(result.passed, true, `${phase} acceptance must pass first.`);
    assert.equal(result.manifestSHA256, sha(path.join(acceptance, 'inputs.json')));
  }
  const source=read(path.join(out,'source/result.json'));
  for(const row of source.runtime){const file=path.join(root,row.path);assert.equal(fs.statSync(file).size,row.bytes);assert.equal(sha(file),row.sha256);}
  const raw = manifest().assets.rawFile;
  assert.equal(sha(raw), '775c806358fedddec7622113a5e399a3330bd5a65e35e37ae1108d8bf58067be');
}

async function run(exe, args, log) {
  const fd = fs.openSync(log, 'wx');let child,timer;
  try {
    const code = await new Promise((resolve, reject) => {
      child = cp.spawn(exe, args, {cwd: root, env, windowsHide: true, stdio: ['ignore', fd, fd]});
      child.once('error', reject); child.once('close', resolve);
      timer=setTimeout(()=>{if(child.pid&&child.exitCode===null)cp.spawnSync('taskkill.exe',['/PID',String(child.pid),'/T','/F'],{stdio:'ignore',windowsHide:true});reject(new Error('Command exceeded 30 minute controller bound'));},1800000);
    });
    assert.equal(code, 0, `Command failed (${code}); retained log: ${log}`);
  } finally { clearTimeout(timer);fs.closeSync(fd); }
}

async function build() {
  prerequisites();
  const d = path.join(out, 'package');
  assert(!fs.existsSync(d), 'Preserve previous package attempt; do not retry automatically.');
  // Validate independent readers before spending time on a production build.
  cp.execFileSync(manifest().assets.python, ['-c', 'import PIL, fitz'], {env, stdio: 'pipe'});
  fs.mkdirSync(d);
  const started = new Date().toISOString();
  let error = null, receipt = null, portableDirectory = null;
  try {
    console.log('[package] Building production Tauri portable with the validated WASM.');
    // Tauri may reformat this JSON. Preserve both byte forms, accept no semantic changes.
    const configPath=path.join(root,'src-tauri/tauri.conf.json'),configBefore=fs.readFileSync(configPath);
    fs.writeFileSync(path.join(d,'tauri-config-before.json'),configBefore,{flag:'wx'});
    try{
    await run('powershell.exe', ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', path.join(root, 'scripts', 'build-windows-x64.ps1'), '-SkipWasm', '-NoSmoke'], path.join(d, 'build.log'));
    }finally{
      const configAfter=fs.readFileSync(configPath);
      if(!configAfter.equals(configBefore)){
        fs.writeFileSync(path.join(d,'tauri-config-after.json'),configAfter,{flag:'wx'});
        assert.deepEqual(JSON.parse(configAfter),JSON.parse(configBefore),'Build changed configuration semantics; preserve and stop');
        fs.writeFileSync(configPath,configBefore);
        save(path.join(d,'tauri-format-restored.json'),{formatOnly:true,before:crypto.createHash('sha256').update(configBefore).digest('hex'),after:crypto.createHash('sha256').update(configAfter).digest('hex')});
      }
    }

    frozen();
    const matches = [...fs.readFileSync(path.join(d, 'build.log'), 'utf8').matchAll(/Portable zip: (.+\.zip) \(/g)];
    assert.equal(matches.length, 1, 'Expected exactly one new portable path in the build log.');
    const zip = matches[0][1].trim(); portableDirectory = zip.slice(0, -4);
    assert(path.resolve(zip).startsWith(path.join(root, 'build-artifacts/windows-x64') + path.sep));
    await run(process.execPath, ['scripts/verify-portable-release.mjs', zip, path.join(d, 'integrity')], path.join(d, 'integrity.log'));
    receipt = read(path.join(d, 'integrity/release-manifest.json'));
    const {inspectZip} = await import(pathToFileURL(path.join(root, 'scripts/verify-portable-release.mjs')));
    assert.equal(receipt.sourceCommit, read(path.join(out, 'inputs.json')).sourceCommit);
    assert.equal(receipt.sourceDirty, true, 'This is a local dirty-source candidate, not a clean published release.');
    assert.equal(receipt.wasm.sha256, sha(path.join(root, 'app/src/engine/pkg/compositor_engine_bg.wasm')));
    const copiedZip = path.join(d, 'integrity', receipt.zip.name);
    assert.equal(fs.statSync(copiedZip).size, receipt.zip.bytes); assert.equal(sha(copiedZip), receipt.zip.sha256);
    assert.deepEqual(inspectZip(fs.readFileSync(copiedZip)).map(e => ({name: e.name, bytes: e.bytes.length, sha256: crypto.createHash('sha256').update(e.bytes).digest('hex')})), receipt.entries);
    const receiptPath = path.join(d, 'integrity/release-manifest.json');
    assert.equal(fs.readFileSync(path.join(d, 'integrity/SHA256SUMS.txt'), 'utf8'), `${receipt.zip.sha256}  ${receipt.zip.name}\n${sha(receiptPath)}  release-manifest.json\n`);
    console.log('[package] ZIP integrity and source binding passed. Testing real sensor decoding.');
    const helper = path.join(portableDirectory, 'CompositorRaw.exe');
    const raw = manifest().assets.rawFile;
    await run(manifest().assets.python, ['-X', 'utf8', 'scripts/verify-mac157-raw.py', helper, raw, path.join(d, 'real-raw')], path.join(d, 'real-raw.log'));
    await run(manifest().assets.python, ['-X', 'utf8', 'scripts/make-phase7-dng-probe.py', path.join(d, 'no-preview.dng')], path.join(d, 'synthetic-input.log'));
    await run(manifest().assets.python, ['-X', 'utf8', 'scripts/verify-mac157-raw.py', helper, path.join(d, 'no-preview.dng'), path.join(d, 'synthetic-raw')], path.join(d, 'synthetic-raw.log'));
    assert.equal(read(path.join(d, 'real-raw/receipt.json')).status, 'PASS');
    assert.equal(read(path.join(d, 'synthetic-raw/receipt.json')).status, 'PASS');
    frozen();
  } catch (e) { error = String(e.stack || e); }
  save(path.join(d, 'result.json'), {scope: 'CURRENT_LOCAL_PRODUCTION_PORTABLE_AND_RAW', started, finished: new Date().toISOString(), passed: error === null, error, receipt, portableDirectory, manifestSHA256: sha(path.join(out,'inputs.json')), published: false, nativeGui: 'NOT_RUN'});
  if (error) throw Error(error);
  console.log('[package] PASS. Native production GUI is next.');
}

async function native(kind) {
  const increment=kind==='native157',latest=kind==='native1511';
  if(kind!=='native')assert.equal(read(path.join(out,latest?'native157':'native','result.json')).passed,true);
  prerequisites();
  const pkg = read(path.join(out, 'package/result.json')); assert.equal(pkg.passed, true);
  for (const e of pkg.receipt.entries) assert.equal(sha(path.join(pkg.portableDirectory, e.name)), e.sha256, e.name);
  const gate = manifest().assets.displayGate;
  const preflight = JSON.parse(cp.execFileSync(gate, ['check'], {encoding: 'utf8'}).trim());
  assert.equal(preflight.success, true); assert.equal(preflight.unlocked, true); assert.equal(preflight.displayCount, 1);
  const d = path.join(out,kind); assert(!fs.existsSync(d), 'Preserve prior native evidence.'); fs.mkdirSync(d);
  const started = new Date().toISOString(), monitorFile = path.join(d, 'display-session.jsonl');
  const monitor = cp.spawn(gate, ['monitor', monitorFile, '1800'], {stdio: 'ignore', windowsHide: true});
  let monitorEnded = false;
  const monitorDone = new Promise(resolve => { monitor.once('exit', code => {monitorEnded = true; resolve(code);}); monitor.once('error', () => {monitorEnded = true; resolve(null);}); });
  let nativeChild, exit = null, error = null, invalid = null, timer, watchdog, workloadEnd = null;
  const killOwned = () => { if (nativeChild && nativeChild.exitCode === null) cp.spawnSync('taskkill.exe', ['/PID', String(nativeChild.pid), '/T', '/F'], {stdio: 'ignore', windowsHide: true}); };
  const sample = () => {
    if (!fs.existsSync(monitorFile)) return [];
    const f = fs.openSync(monitorFile, 'r'); let text;
    try { const size = fs.fstatSync(f).size, start = Math.max(0, size - 8192), b = Buffer.alloc(size - start); fs.readSync(f, b, 0, b.length, start); text = b.toString('utf8'); } finally {fs.closeSync(f);}
    return text.replace(/^\uFEFF/, '').split(/\r?\n/).flatMap(x => {try {return [JSON.parse(x)];} catch {return [];}});
  };
  const inspect = () => {
    if (invalid) return;
    const rows = sample(), last = rows.at(-1), bad = rows.find(s => s.success !== true || s.unlocked !== true || s.displayCount !== 1);
    if (bad || monitorEnded || Date.now() - Date.parse(last?.utc ?? started) > 5000) { invalid = {sample: bad ?? last ?? null, monitorEnded}; killOwned(); }
  };
  const fd = fs.openSync(path.join(d, 'controller.log'), 'wx');
  try {
    await sleep(700); inspect(); assert.equal(invalid, null);
    console.log(`[${kind}] Running production WebView2 controls and real Windows dialogs; do not use keyboard or mouse.`);
    const args=['-NoProfile','-ExecutionPolicy','Bypass','-File',path.join(root,'scripts',latest?'verify-mac1511-native.ps1':increment?'verify-mac157-increment-bounded.ps1':'verify-mac157-regression-bounded.ps1'),'-PortableDirectory',pkg.portableDirectory,'-Marker',pkg.receipt.marker,'-Probes',(increment||latest)?manifest().assets.native157Fixtures:manifest().assets.mac149Probes,'-OutputDirectory',path.join(d,'gui'),'-RawFile',manifest().assets.rawFile,'-DesktopGate',gate,'-Port',latest?'19719':increment?'19718':'19717'];
    nativeChild = cp.spawn('powershell.exe', args, {cwd: root, env, windowsHide: true, stdio: ['ignore', fd, fd]});
    timer = setInterval(inspect, 500);
    watchdog=setTimeout(()=>{invalid={reason:'Native GUI group exceeded eight minutes'};killOwned();},480000);
    exit = await new Promise((resolve, reject) => { nativeChild.once('exit', resolve); nativeChild.once('error', reject); });
    workloadEnd = Date.now();clearTimeout(watchdog);
    const deadline = Date.now() + 5000;
    while (Date.now() < deadline && !(Date.parse(sample().at(-1)?.utc ?? '') >= workloadEnd)) await sleep(100);
    inspect(); assert.equal(exit, 0, 'Native GUI protocol failed.'); assert.equal(invalid, null);
    assert(Date.parse(sample().at(-1)?.utc ?? '') >= workloadEnd, 'Observer missed workload end.');
    const gui=read(path.join(d,'gui/result.json'));assert.equal(gui.status,'PASS');
    if(!latest){const transport=fs.readFileSync(path.join(d,'gui/cdp-transport.jsonl'),'utf8').trim().split('\n').map(JSON.parse);
    assert(transport.some(r=>r.type==='cdp-post-data-cap'));assert(transport.filter(r=>r.type==='cdp-post-data-cap').every(r=>r.maxPostDataSize===65536));assert(!transport.some(r=>r.type==='cdp-transport-error'));}
    if(!increment&&!latest){
      assert.equal(gui.nativeDialogCount,25);
      await run(manifest().assets.python,['-X','utf8',manifest().assets.native149Reader,path.join(d,'gui'),manifest().assets.native149Expected],path.join(d,'independent-readers.log'));
      assert.equal(read(path.join(d,'gui/independent-reader-receipt.json')).status,'PASS');
    }else if(increment){
      assert(gui.nativeDialogCount>=10);assert(gui.rows.length>=5);
      await run(manifest().assets.python,['-X','utf8','scripts/verify-mac157-native-output.py',path.join(d,'gui'),manifest().assets.native157Fixtures],path.join(d,'independent-readers.log'));
      assert.equal(read(path.join(d,'gui/independent-reader.json')).passed,true);
    }
    if(latest){assert.equal(gui.rows.length,6);assert(gui.nativeDialogCount>=18);assert.equal(gui.identity.testApi,'undefined');assert.equal(gui.identity.native,true);}
    frozen();
    for (const e of pkg.receipt.entries) assert.equal(sha(path.join(pkg.portableDirectory, e.name)), e.sha256, e.name);
  } catch (e) {error = String(e.stack || e); killOwned();}
  finally {clearInterval(timer);clearTimeout(watchdog); fs.closeSync(fd); fs.writeFileSync(monitorFile + '.stop', 'Stop owned read-only observer');}
  const observerExit = await Promise.race([monitorDone, sleep(10000).then(() => 'TIMEOUT')]);
  if (observerExit === 'TIMEOUT') monitor.kill();
  const summary = fs.existsSync(monitorFile + '.summary.json') ? read(monitorFile + '.summary.json') : null;
  const passed = !error && !invalid && exit === 0 && observerExit === 0 && summary?.samples > 0 && summary.allSamplesSingleDisplay === true && summary.allSamplesUnlocked === true;
  save(path.join(d, 'result.json'), {scope: 'CURRENT_PRODUCTION_GUI_DIALOGS_AND_INDEPENDENT_READERS', started, finished: new Date().toISOString(), passed, exit, error, invalid, preflight, observerExit, summary, workloadEnd, marker: pkg.receipt.marker, manifestSHA256: sha(path.join(out,'inputs.json')), published: false});
  assert(passed, `Native gate incomplete. Inspect ${d}`);
  console.log(`[${kind}] PASS. Production dialogs and independent readers completed.`);
}

async function main(){
 process.chdir(root);const mode=arg('--stage');assert(['package','native','native157','native1511','verify'].includes(mode));frozen();if(mode==='verify')return;assert.equal(process.platform,'win32');
 const m=manifest();assert.equal(read(m.codecsFile).passed,true,'Real codec acceptance required');
 env={...process.env,RUSTUP_TOOLCHAIN:'1.95.0',CARGO_INCREMENTAL:'0',VITE_TEST_API:'',VITE_BRIDGE:'',PERF:'',GPU_WARP_PERF:'',GPU_WARP_UI_PERF:'',GITHUB_SHA:m.sourceCommit,RELEASE_VERSION:'0.12.0'};
 env.PSModulePath=`${process.env.USERPROFILE}\\Documents\\WindowsPowerShell\\Modules;C:\\Program Files\\WindowsPowerShell\\Modules;C:\\Windows\\system32\\WindowsPowerShell\\v1.0\\Modules`;
 const k=Object.keys(env).find(k=>k.toLowerCase()==='path')||'Path';env[k]=path.dirname(m.assets.python)+';'+process.env.USERPROFILE+'\\.cargo\\bin;C:\\Program Files\\LLVM\\bin;'+env[k];
 const lock=path.join(root,'build-artifacts/.mac1511-stage.lock');save(lock,{pid:process.pid,stage:mode,out});
 try{if(mode==='package')await build();else await native(mode);}finally{if(read(lock).pid===process.pid)fs.unlinkSync(lock);}
}
main().catch(e=>{console.error(e.stack||e);process.exitCode=2;});
