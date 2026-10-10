import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { verifyWasmOutput } from '../verify-wasm-output.mjs';
function fixture(t, exports, glue='wasm.wasmengine_new();') {
 const folder=fs.mkdtempSync(path.join(os.tmpdir(),'compositor-wasm-guard-'));
 t.after(()=>fs.rmSync(folder,{recursive:true,force:true}));
 const names=exports.map(name=>{const b=Buffer.from(name);return Buffer.concat([Buffer.from([b.length]),b,Buffer.from([0,0])]);});
 const payload=Buffer.concat([Buffer.from([names.length]),...names]);
 const bytes=Buffer.concat([Buffer.from([0,97,115,109,1,0,0,0,1,4,1,96,0,0,3,2,1,0,7,payload.length]),payload,Buffer.from([10,4,1,2,0,11])]);
 fs.writeFileSync(path.join(folder,'compositor_engine_bg.wasm'),bytes);
 fs.writeFileSync(path.join(folder,'compositor_engine.js'),glue);
 return {folder,bytes};
}
const required=['wasmengine_new','wasmengine_composite','wasmengine_render_plan'];
test('complete WASM and matching JavaScript calls produce a hash receipt',t=>{
 const {folder,bytes}=fixture(t,required,'wasm.wasmengine_new(); wasm.wasmengine_composite();');
 const result=verifyWasmOutput(folder);assert.equal(result.bytes,bytes.length);assert.equal(result.checkedGlueCalls,2);assert.match(result.sha256,/^[a-f0-9]{64}$/);
});
test('a success exit cannot make a truncated WASM package valid',t=>{
 const {folder,bytes}=fixture(t,required);fs.writeFileSync(path.join(folder,'compositor_engine_bg.wasm'),bytes.subarray(0,bytes.length-1));
 assert.throws(()=>verifyWasmOutput(folder),/Invalid or truncated/);
});
test('valid WASM still fails with missing editor exports or mismatched glue',t=>{
 for(const [names,glue] of [[required.slice(0,2),'wasm.wasmengine_new();'],[required,'wasm.an_old_export();'],[required,'// no calls']]){
  const {folder}=fixture(t,names,glue);assert.throws(()=>verifyWasmOutput(folder));
 }
});
