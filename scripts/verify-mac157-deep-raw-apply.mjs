// Byte-exact real 10 MP worker regression; native UI keeps its original 30 second budget.
import fs from 'node:fs';import path from 'node:path';import crypto from 'node:crypto';import assert from 'node:assert/strict';import {pathToFileURL} from 'node:url';
const [directory,image]=process.argv.slice(2);assert(directory&&image,'Explicit generated WASM and frozen 16-bit input required');
const hash=b=>crypto.createHash('sha256').update(b).digest('hex');
const png=fs.readFileSync(image);assert.equal(hash(png),'0fe2b3b1d0ba25293e63c2ec11b165c9ddc5d67e8053ef4447212abf731ce0a2');
const {initSync,WasmEngine}=await import(pathToFileURL(path.resolve(directory,'compositor_engine.js')));
const {memory}=initSync({module:fs.readFileSync(path.resolve(directory,'compositor_engine_bg.wasm'))});const main=new WasmEngine(),worker=new WasmEngine();
const keep=e=>new Uint8Array(memory.buffer,e.job_buffer_ptr(false),e.job_buffer_len(false)).slice();
const doc=main.import_image(undefined,new Uint8Array(png),'Canon16'),state=JSON.parse(main.state(doc)),layer=state.activeLayerId;assert.equal(state.layers[0].pixelDepth,'Sixteen');
const input=main.prepare_job(doc,layer),before=keep(main);main.release_job();const start=performance.now();
const result=worker.run_edit_job(input,before,undefined,undefined,JSON.stringify({type:'ApplyFilter',id:layer,params:{filter:'CameraRaw',settings:{exposure:0.5}}}),0),elapsedMs=performance.now()-start,output=keep(worker);worker.release_job();
assert.equal(JSON.parse(result).pixelDepth,'Sixteen');assert.equal(output.byteLength,81348928);assert.equal(hash(output),'7d9c993cb20589912d7ffa2edf53c358819500090ec3c697cf5ad937007834fe');
main.begin_staged_install(output.length,0,0);for(let i=0;i<output.length;i+=1048576)main.append_staged_install(0,output.subarray(i,i+1048576));main.finish_staged_install(doc,layer,JSON.stringify(JSON.parse(input).stamp),result,false);
main.prepare_job(doc,layer);assert.equal(hash(keep(main)),hash(output));main.release_job();main.undo(doc);main.prepare_job(doc,layer);assert.equal(hash(keep(main)),hash(before));main.release_job();main.redo(doc);main.prepare_job(doc,layer);assert.equal(hash(keep(main)),hash(output));main.release_job();main.free();worker.free();
console.log(JSON.stringify({passed:true,scope:'REAL_WASM_10MP_DEEP_RAW_EXPOSURE_AND_UNDO_REDO',elapsedMs,bytes:output.length,sha256:hash(output),nativeUiBudgetUnchanged:true}));
