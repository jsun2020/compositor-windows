/// <reference types="vite/client" />
import {test,expect,type Page} from "@playwright/test";
async function setup(page:Page,mode="Smudge",styled=false){
  await page.goto("/");await expect(page.getByTestId("engine-ready")).toBeVisible();
  const ids=await page.evaluate(async styled=>{
    const a=(window as any).__compositor,e=a.engine,s=a.store.getState();
    const b=new Uint8Array(60*40*4);for(let i=0;i<b.length;i+=4){b[i+3]=255;if(i/4%60===3)b.fill(255,i,i+4);}
    const doc=e.newDocument(60,40,true);e.pastePixels(doc,60,40,b.buffer,[0,0]);const layer=e.state(doc).activeLayerId;
    if(styled){e.execute(doc,{type:"AddMask",id:layer,revealing:true});e.execute(doc,{type:"SetLayerOpacity",id:layer,opacity:0.5});e.execute(doc,{type:"SetLayerEffects",id:layer,effects:{stroke:{red:0,green:0,blue:1,opacity:1,size:1,inside:false}}});}
    s.openDocument(doc);await a.setZoom(8);return{doc,layer,depth:e.state(doc).undoDepth,before:Array.from(e.jobInput(doc,layer).pixels?new Uint8Array(e.jobInput(doc,layer).pixels):[])};
  },styled);
  await page.keyboard.press("r");await page.getByLabel("Smear mode").selectOption(mode);
  await page.getByLabel("Brush size",{exact:true}).fill("2");await page.getByLabel("Brush hardness",{exact:true}).fill("50");await page.getByLabel("Smear strength",{exact:true}).fill("50");await page.getByLabel("Smear strength",{exact:true}).blur();
  return ids;
}
async function at(page:Page,x:number,y:number){return page.evaluate(([x,y])=>{
  const s=(window as any).__compositor.store.getState(),p=s.viewports[s.activeId].viewPoint({x,y},s.documents[s.activeId]),r=document.querySelector('[data-testid="canvas-view"]')!.getBoundingClientRect();return{x:r.left+p.x,y:r.top+p.y};
},[x,y]);}
async function down(page:Page){const p=await at(page,3,20);await page.mouse.move(p.x,p.y);await page.mouse.down();}
async function move(page:Page){const p=await at(page,6,20);await page.mouse.move(p.x,p.y,{steps:3});}
const sample=(page:Page,doc:string)=>page.evaluate(doc=>Array.from((window as any).__compositor.engine.composite(doc,{x:4,y:20,width:1,height:1},1,1)),doc);
const stored=(page:Page,ids:{doc:string;layer:string})=>page.evaluate(ids=>Array.from(new Uint8Array((window as any).__compositor.engine.jobInput(ids.doc,ids.layer).pixels)),ids);
async function idle(page:Page){await expect.poll(()=>page.evaluate(()=>{const s=(window as any).__compositor.store.getState();return !s.working&&!s.brushDraft;})).toBe(true);await expect(page.getByTestId("error-banner")).toHaveCount(0);}
for(const mode of ["Liquify","Smudge"])test(`${mode} production controls show real live pixels and commit one reversible edit`,async({page})=>{
  const ids=await setup(page,mode,true),before=await sample(page,ids.doc);
  await expect(page.getByLabel("Blur radius",{exact:true})).toHaveCount(0);await expect(page.getByLabel("Brush opacity",{exact:true})).toHaveCount(0);
  await down(page);await move(page);
  await expect.poll(()=>sample(page,ids.doc)).not.toEqual(before);
  expect(await stored(page,ids)).toEqual(ids.before);
  const expected=await page.evaluate(ids=>{
    const a=(window as any).__compositor,e=a.engine,s=a.store.getState(),d=s.brushDraft;
    const copy=e.openPackage(e.savePackage(ids.doc),null);const layer=e.state(copy).activeLayerId;
    e.execute(copy,{type:"WarpStroke",id:layer,mask:false,warp:{...d.warp,points:d.points}});
    return{pixels:Array.from(e.layerPixels(copy,layer)),composite:Array.from(e.composite(copy,{x:0,y:0,width:60,height:40},60,40)),depth:e.state(ids.doc).undoDepth};
  },ids);
  expect(expected.depth).toBe(ids.depth);
  await page.mouse.up();await idle(page);expect(await stored(page,ids)).toEqual(expected.pixels);
  const actual=await page.evaluate(ids=>{const e=(window as any).__compositor.engine;return{image:Array.from(e.composite(ids.doc,{x:0,y:0,width:60,height:40},60,40)),depth:e.state(ids.doc).undoDepth};},ids);
  expect(actual.image).toEqual(expected.composite);expect(actual.depth).toBe(ids.depth+1);
  await page.keyboard.press("Control+z");expect(await stored(page,ids)).toEqual(ids.before);
  await page.keyboard.press("Control+Shift+z");expect(await stored(page,ids)).toEqual(expected.pixels);
});
for(const cancel of ["escape","tab","tool","mode","capture"] as const)test(`live warp cancellation by ${cancel} discards preview and asynchronous completion`,async({page})=>{
  const ids=await setup(page),before=await sample(page,ids.doc);await down(page);await move(page);
  await expect.poll(()=>sample(page,ids.doc)).not.toEqual(before);
  if(cancel==="escape")await page.keyboard.press("Escape");
  if(cancel==="tool")await page.keyboard.press("b");
  if(cancel==="mode")await page.getByLabel("Smear mode").selectOption("Liquify");
  if(cancel==="tab")await page.evaluate(()=>{const a=(window as any).__compositor;a.store.getState().openDocument(a.engine.newDocument(20,20,true));});
  if(cancel==="capture")await page.evaluate(()=>document.querySelector('[data-testid="canvas-view"]')!.dispatchEvent(new Event("lostpointercapture")));
  await page.mouse.up();await idle(page);expect(await stored(page,ids)).toEqual(ids.before);expect(await sample(page,ids.doc)).toEqual(before);
  expect(await page.evaluate(ids=>(window as any).__compositor.engine.state(ids.doc).undoDepth,ids)).toBe(ids.depth);
});
test("pickup and cancellation during preparation keep redo; masks refuse before worker startup",async({page})=>{
  const ids=await setup(page);
  await page.evaluate(ids=>{const e=(window as any).__compositor.engine;e.execute(ids.doc,{type:"InvertPixels",id:ids.layer,mask:false});e.undo(ids.doc);},ids);
  await down(page);await page.mouse.up();await idle(page);
  expect(await stored(page,ids)).toEqual(ids.before);
  expect(await page.evaluate(ids=>(window as any).__compositor.engine.state(ids.doc).canRedo,ids)).toBe(true);
  await down(page);await move(page);await page.keyboard.press("Escape");await page.mouse.up();await idle(page);expect(await stored(page,ids)).toEqual(ids.before);
  await page.evaluate(ids=>{const a=(window as any).__compositor;a.engine.execute(ids.doc,{type:"AddMask",id:ids.layer,revealing:true});a.store.getState().refresh(ids.doc);a.store.setState({maskSelected:true});},ids);
  await down(page);await page.mouse.up();await expect(page.getByTestId("error-banner")).toContainText("not its mask");expect(await stored(page,ids)).toEqual(ids.before);
});
test("production completion refuses the original stamp after a source transform changes",async({page})=>{
  const ids=await setup(page),before=await sample(page,ids.doc);await down(page);await move(page);await expect.poll(()=>sample(page,ids.doc)).not.toEqual(before);
  await page.evaluate(ids=>{const a=(window as any).__compositor;a.engine.execute(ids.doc,{type:"NudgeLayers",ids:[ids.layer],dx:1,dy:0});a.store.getState().refresh(ids.doc);},ids);
  await page.mouse.up();await expect(page.getByTestId("error-banner")).toContainText("layer changed");
  await expect.poll(()=>page.evaluate(()=>(window as any).__compositor.store.getState().working)).toBe(false);
  expect(await stored(page,ids)).toEqual(ids.before);
  expect(await page.evaluate(ids=>(window as any).__compositor.engine.state(ids.doc).undoDepth,ids)).toBe(ids.depth+1);
});
test("real dedicated worker falls back to bounded CPU when float GPU targets are unavailable",async({page})=>{
  const ids=await setup(page);
  const result=await page.evaluate(async ids=>{
    const path="/src/engine/warp-client.ts",{WarpClient}:typeof import("../../src/engine/warp-client")=await import(/* @vite-ignore */path);
    const api=(window as any).__compositor,e=api.engine;
    const script=`OffscreenCanvas.prototype.getContext=()=>null; await import(${JSON.stringify(new URL("/src/engine/warp-worker.ts",location.href).href)}); postMessage({type:"boot"});`;
    const url=URL.createObjectURL(new Blob([script],{type:"text/javascript"}));const w=new Worker(url,{type:"module"});
    await new Promise<void>((resolve,reject)=>{w.onmessage=()=>resolve();w.onerror=e=>reject(Error(e.message));});
    const c=new WarpClient(w,e.module,e.jobInput(ids.doc,ids.layer),{mode:"Smudge",diameter:2,hardness:0.5,strength:0.5},()=>{},()=>{});
    try{c.append([3,20]);c.append([6,20]);const r=await c.finish(1);return{backend:c.backend,pixels:Array.from(new Uint8Array(r!.pixels!))};}
    finally{c.dispose();URL.revokeObjectURL(url);}
  },ids);
  expect(result.backend).toBe("CPU");expect(result.pixels.slice((20*60+4)*4,(20*60+4)*4+4)).toEqual([128,128,128,255]);
});
for(const action of ["cancel","commit"] as const)test(`pointer-up/${action} during delayed snapshot preparation preserves the entire gesture`,async({page})=>{
  const ids=await setup(page);
  await page.evaluate(()=>{
    const a=(window as any).__compositor,e=a.engine,original=e.jobInputAsync.bind(e);
    e.jobInputAsync=(doc:string,layer:string)=>{const snapshot=original(doc,layer);return new Promise(resolve=>{(window as any).releaseWarpSnapshot=()=>resolve(snapshot);});};
  });
  await down(page);await move(page);
  if(action==="cancel")await page.keyboard.press("Escape");
  await page.mouse.up();
  if(action==="commit")expect(await page.evaluate(()=>(window as any).__compositor.store.getState().working)).toBe(true);
  await page.evaluate(()=>(window as any).releaseWarpSnapshot());await idle(page);
  if(action==="cancel")expect(await stored(page,ids)).toEqual(ids.before);
  else expect((await stored(page,ids)).slice((20*60+4)*4,(20*60+4)*4+4)).toEqual([128,128,128,255]);
  expect(await page.evaluate(ids=>(window as any).__compositor.engine.state(ids.doc).undoDepth,ids)).toBe(ids.depth+(action==="commit"?1:0));
});
test("sparse GPU readback refuses its cap before allocation and disposes the stroke",async({page})=>{
  await setup(page);
  const r=await page.evaluate(async()=>{
    const path="/src/canvas/gpu-warp.ts",{GpuWarpStroke}:typeof import("../../src/canvas/gpu-warp")=await import(/* @vite-ignore */path);
    const gl=document.createElement("canvas").getContext("webgl2",{antialias:false})!;
    const source=new Uint8Array(20*16*4);for(let i=3;i<source.length;i+=4)source[i]=255;
    const stroke=new GpuWarpStroke(gl,20,16,source,{mode:"Smudge",diameter:2,hardness:0.5,strength:0.5});
    try{stroke.append([3,8]);stroke.append([6,8]);let error="";try{stroke.readTiles(0);}catch(e){error=String(e);}
      return{error,diagnostic:stroke.diagnostics()};
    }finally{stroke.dispose();gl.getExtension("WEBGL_lose_context")?.loseContext();}
  });
  expect(r.error).toContain("readback limit");expect(r.diagnostic).toMatchObject({disposed:true,allocatedBytes:0,liveTextures:0,liveFramebuffers:0,livePrograms:0});
});
