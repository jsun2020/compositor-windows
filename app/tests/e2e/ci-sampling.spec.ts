import { test, expect } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
const root=path.resolve("engine/tests/fixtures/cg-ci-sampling-mac-1.4.5");
const cases=JSON.parse(fs.readFileSync(path.join(root,"cases.json"),"utf8")).filter((c:any)=>c.operation==="warp");
const records=JSON.parse(fs.readFileSync(path.join(root,"mac/result.json"),"utf8")).records;
for(const c of cases){
 test("CI distortion applies, undoes, redoes and reopens Mac pixels: "+c.name,async({page})=>{
  const folder=path.join(root,c.package),record=records.find((r:any)=>r.name===c.name);
  const full=fs.readFileSync(path.join(root,"mac",c.name+".rgba8"));
  let x0=record.width,y0=record.height,x1=0,y1=0;
  for(let y=0;y<record.height;y++)for(let x=0;x<record.width;x++)if(full[(y*record.width+x)*4+3]){x0=Math.min(x0,x);y0=Math.min(y0,y);x1=Math.max(x1,x+1);y1=Math.max(y1,y+1);}
  if(x1===0){x0=0;y0=0;x1=record.width;y1=record.height;}
  const crop=(b:Uint8Array,channels:number)=>Array.from(Buffer.concat(Array.from({length:y1-y0},(_,i)=>Buffer.from(b.subarray(((y0+i)*record.width+x0)*channels,((y0+i)*record.width+x1)*channels)))));
  const expected=crop(full,4),maskPath=path.join(root,"mac",c.name+"-mask.gray8");
  const mask=fs.existsSync(maskPath)?crop(fs.readFileSync(maskPath),1):null;
  const input={manifest:fs.readFileSync(path.join(folder,"manifest.json"),"utf8"),images:fs.readdirSync(path.join(folder,"images")).map(name=>({name,bytes:Array.from(fs.readFileSync(path.join(folder,"images",name)))})),corners:c.corners,sampling:c.sampling};
  await page.goto("/");await expect(page.getByTestId("engine-ready")).toBeVisible();
  const result=await page.evaluate(async(input:any)=>{
   const api=(window as any).__compositor,engine=api.engine;
   const doc=engine.openPackage({manifest:input.manifest,images:input.images.map((i:any)=>({name:i.name,bytes:new Uint8Array(i.bytes)}))},null);
   const layer=engine.state(doc).layers[0],id=layer.id;
   const capture=(d:string)=>{const l=engine.state(d).layers[0];return{transform:l.transform,width:l.pixelsWidth,height:l.pixelsHeight,pixels:Array.from(engine.layerPixels(d,id)||[]),mask:Array.from(engine.maskPixels(d,id)||[])};};
   const original=capture(doc);
   engine.execute(doc,{type:"DistortLayer",id,transform:{...layer.transform,sampling:input.sampling},corners:input.corners});
   const applied=capture(doc);engine.undo(doc);const undone=capture(doc);engine.redo(doc);const redone=capture(doc);
   const saved=engine.savePackage(doc),reopened=engine.openPackage(saved,null),restored=capture(reopened);
   api.store.getState().openDocument(reopened);await api.setZoom(1);api.setCheckerboard(false);
   await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));
   const state=api.store.getState(),d=state.documents[reopened],vp=state.viewports[reopened],dpr=window.devicePixelRatio||1;
   const rect=vp.documentRect(d);vp.translate({width:(Math.round(rect.x*dpr)-rect.x*dpr)/dpr,height:(Math.round(rect.y*dpr)-rect.y*dpr)/dpr});
   api.renderer.render(engine,d,vp,dpr,{checkerboard:false},null);
   const gpu=Array.from(api.readDocumentPixels()) as number[],cpu=Array.from(engine.composite(reopened,{x:0,y:0,width:d.width,height:d.height},d.width,d.height)) as number[];
   const worst=gpu.reduce((m,v,i)=>Math.max(m,Math.abs(v-cpu[i])),0);
   const outcome={applied,undoExact:JSON.stringify(original)===JSON.stringify(undone),redoExact:JSON.stringify(applied)===JSON.stringify(redone),reopenExact:JSON.stringify(applied)===JSON.stringify(restored),gpuWorst:worst,gpuLength:gpu.length,cpuLength:cpu.length,renderer:state.rendererKind};
   api.store.getState().closeDocument(reopened);engine.closeDocument(doc);return outcome;
  },input);
  expect(result.applied.width).toBe(x1-x0);expect(result.applied.height).toBe(y1-y0);
  expect(result.applied.transform).toEqual({...record.transform,origin:[record.transform.origin[0]+x0,record.transform.origin[1]+y0],size:[x1-x0,y1-y0]});
  expect(result.applied.pixels).toEqual(expected);
  if(mask){expect(result.applied.mask.length).toBe(mask.length);expect(Math.max(...mask.map((v,i)=>Math.abs(v-(result.applied.mask[i] as number))))).toBeLessThanOrEqual(1);}else expect(result.applied.mask).toEqual([]);
  expect(result.undoExact).toBe(true);expect(result.redoExact).toBe(true);expect(result.reopenExact).toBe(true);
  expect(result.renderer).toBe("gl");expect(result.gpuLength).toBe(result.cpuLength);expect(result.gpuWorst).toBeLessThanOrEqual(2);
 });
}
