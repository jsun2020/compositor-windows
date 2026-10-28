// Prepare an explicit immutable acceptance snapshot. Listing tests is not a test pass.
import fs from 'node:fs';import path from 'node:path';import cp from 'node:child_process';import crypto from 'node:crypto';import assert from 'node:assert/strict';import {fileURLToPath} from 'node:url';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');process.chdir(root);
const arg=k=>{const i=process.argv.indexOf(k);assert(i>=0&&process.argv[i+1]&&!process.argv[i+1].startsWith('--'),'Missing '+k);return process.argv[i+1];};
const outRel=arg('--out').replaceAll('\\','/');assert(/^build-artifacts\/[a-zA-Z0-9_-]+$/.test(outRel),'One named directory under build-artifacts required');
const out=path.join(root,outRel),freeze=process.argv.includes('--freeze');
const sha=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const read=p=>JSON.parse(fs.readFileSync(p,'utf8').replace(/^\uFEFF/,''));
const write=(p,x)=>fs.writeFileSync(p,JSON.stringify(x,null,2)+'\n',{flag:'wx'});
const cpu=['perf-4-5','perf-4b1','perf-brush-retouch','perf-selection','perf-spatial'].map(n=>'app/tests/e2e/'+n+'.spec.ts');
const gpu=['perf-gpu-warp','perf-warp-live'].map(n=>'app/tests/e2e/'+n+'.spec.ts');
const cleanEnv={...process.env,PERF:'',GPU_WARP_PERF:'',GPU_WARP_UI_PERF:'',VITE_TEST_API:'',VITE_BRIDGE:''};
if(!freeze){
 assert(!fs.existsSync(out),'Preserve previous evidence; choose a fresh directory');fs.mkdirSync(out,{recursive:true});
 for(const[name,port]of[['functional',19720],['cpu',19721],['gpu',19722]]){
  const config={testDir:'../../app/tests/e2e',timeout:30000,workers:1,retries:0,forbidOnly:true,fullyParallel:false,use:{baseURL:`http://127.0.0.1:${port}`,headless:true},webServer:{cwd:root,command:`node node_modules/vite/bin/vite.js preview --host 127.0.0.1 --port ${port} --strictPort --outDir "${path.join(out,'dist')}"`,url:`http://127.0.0.1:${port}`,reuseExistingServer:false}};
  if(name!=='cpu'){config.webServer.command=`node node_modules/vite/bin/vite.js --host 127.0.0.1 --port ${port} --strictPort`;config.webServer.env={VITE_BRIDGE:'mock',VITE_TEST_API:'1'};}
  fs.writeFileSync(path.join(out,name+'.config.ts'),'import {defineConfig} from "@playwright/test";\nexport default defineConfig('+JSON.stringify(config)+');\n',{flag:'wx'});
 }
 console.log('Prepared configs only: '+out);process.exit(0);
}
assert(!fs.existsSync(path.join(out,'inputs.json')),'Snapshot already frozen');
const assetsFile=path.resolve(arg('--assets')),sourceFile=path.resolve(arg('--source-receipt'));
const codecsFile=path.resolve(arg('--codecs-receipt')),codecs=read(codecsFile);assert.equal(codecs.passed,true);assert.equal(codecs.sourceReceiptSHA256,sha(sourceFile));
const assets=read(assetsFile),source=read(sourceFile);assert.equal(source.passed,true,'Candidate component checks must pass first');assert.equal(source.version,'0.12.0');
for(const x of assets.expectedFiles||[])assert.equal(sha(x.path),x.sha256.toLowerCase(),'Reviewed artifact drift '+x.path);
const externalPaths=[assetsFile,sourceFile,codecsFile,path.join(path.dirname(sourceFile),'inputs-before.json'),assets.displayGate,assets.sessionGate,assets.captureEnvironment,...(assets.additionalInputs||[]),...(assets.expectedFiles||[]).map(x=>x.path)];
for(const p of externalPaths)assert(p&&path.isAbsolute(p)&&fs.statSync(p).isFile(),'Missing external artifact '+p);
const sourceInputs=read(path.join(path.dirname(sourceFile),'inputs-before.json'));
for(const x of sourceInputs)assert.equal(sha(path.join(root,x.path)),x.sha256,'Source receipt no longer matches '+x.path);
// --list statistics treat EVERY case as skipped; use declared per-case expectedStatus instead.
const counts={};
for(const[name,files,env]of[['functional',[],{}],['cpu',cpu,{PERF:'1'}],['gpu',gpu,{GPU_WARP_PERF:'1',GPU_WARP_UI_PERF:'1'}]]){
 const json=path.join(out,'list-'+name+'.json'),log=path.join(out,'list-'+name+'.log');assert(!fs.existsSync(json));
 const fd=fs.openSync(log,'wx');let result;
 try{result=cp.spawnSync(process.execPath,['node_modules/@playwright/test/cli.js','test','--list',...files,'--config',path.join(out,name+'.config.ts'),'--reporter=json'],{cwd:root,env:{...cleanEnv,...env,PLAYWRIGHT_JSON_OUTPUT_NAME:json},stdio:['ignore',fd,fd],timeout:60000,windowsHide:true});}finally{fs.closeSync(fd);}
 write(path.join(out,'list-'+name+'.exit.json'),{exit:result.status,signal:result.signal,error:result.error?String(result.error):null});assert.equal(result.status,0,'List failed '+name);
 const d=read(json),cases=[];const walk=s=>{for(const sp of s.specs||[])for(const t of sp.tests||[])cases.push({file:sp.file,line:sp.line,title:sp.title,expectedStatus:t.expectedStatus});for(const child of s.suites||[])walk(child);};walk(d);
 assert(cases.length>0);assert(cases.every(t=>['passed','skipped'].includes(t.expectedStatus)),'No expected failures allowed');
 counts[name]={passed:cases.filter(t=>t.expectedStatus==='passed').length,skipped:cases.filter(t=>t.expectedStatus==='skipped').length,total:cases.length,cases};
 if(name!=='functional')assert.equal(counts[name].skipped,0);assert(counts[name].passed>0);
}
assert.equal(counts.cpu.passed,29);assert.equal(counts.gpu.passed,4);assert.equal(counts.functional.skipped,33);
const roots=['app/src','app/tests','engine/src','engine/tests','engine/native','engine-wasm/src','src-tauri/src','src-tauri/native','src-tauri/icons','src-tauri/capabilities','scripts'];
const paths=cp.execFileSync('rg',['--files',...roots],{encoding:'utf8'}).trim().split(/\r?\n/).filter(p=>!p.includes('__pycache__/'));
paths.push('.github/workflows/ci.yml','.github/workflows/release.yml','Cargo.toml','Cargo.lock','engine/Cargo.toml','engine/build.rs','engine-wasm/Cargo.toml','src-tauri/Cargo.toml','src-tauri/build.rs','src-tauri/tauri.conf.json','package.json','pnpm-lock.yaml','tsconfig.json','vite.config.ts','vitest.config.ts','playwright.config.ts');
for(const name of fs.readdirSync(path.join(root,'app/src/engine/pkg')))if(fs.statSync(path.join(root,'app/src/engine/pkg',name)).isFile())paths.push('app/src/engine/pkg/'+name);
for(const name of ['functional','cpu','gpu'])paths.push(outRel+'/'+name+'.config.ts');
const inputs=[...new Set(paths.map(p=>p.replaceAll('\\','/')))].filter(p=>fs.existsSync(path.join(root,p))).sort().map(p=>({path:p,bytes:fs.statSync(path.join(root,p)).size,sha256:sha(path.join(root,p))}));
const external=[...new Set(externalPaths)].map(p=>({path:p,bytes:fs.statSync(p).size,sha256:sha(p)}));
write(path.join(out,'inputs.json'),{schema:1,scope:'Mac1.5.11 Windows0.12.0 acceptance inputs; not a passing claim',sourceCommit:cp.execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).trim(),upstreamCommit:'9959c32d77330ec8d7b42a49353d903283ecb8e6',version:'0.12.0',workers:1,retries:0,created:new Date().toISOString(),counts,functionalCases:counts.functional.passed,performanceSkips:counts.functional.skipped,cpuCases:29,gpuCases:4,assets,sourceFile,codecsFile,inputs,external});
console.log(`Frozen ${inputs.length} project inputs + ${external.length} external inputs; ${counts.functional.passed} functional /29 CPU/4 GPU`);
