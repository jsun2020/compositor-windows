/// <reference types="vite/client" />
import {test,expect} from "@playwright/test";
test("large warp preview refreshes the rendered layer and cancellation restores its metadata",async({page})=>{
  await page.goto("/");await expect(page.getByTestId("engine-ready")).toBeVisible();
  await page.evaluate(async()=>{
    const a=(window as any).__compositor,e=a.engine,s=a.store.getState();
    const bytes=new Uint8Array(3000*2000*4);new Uint32Array(bytes.buffer).fill(0xff000000);
    for(let y=0;y<2000;y++)bytes.fill(255,(y*3000+1500)*4,(y*3000+1501)*4);
    const doc=e.newDocument(3000,2000,true);e.pastePixels(doc,3000,2000,bytes.buffer,[0,0]);
    s.openDocument(doc);s.setTool("blur");s.setBrushOptions({smearMode:"Liquify",diameter:40,hardness:0.5,strength:0.5});
    const frame=()=>new Promise<void>(r=>requestAnimationFrame(()=>r()));await frame();await frame();
    const vp=a.store.getState().viewports[doc];vp.setZoom(1,vp.center,e.state(doc));s.invalidate();await frame();await frame();
    const canvas=document.querySelector('[data-testid="canvas-view"] canvas') as HTMLCanvasElement;
    (window as any).warpRenderBaseline={doc,layer:e.state(doc).activeLayerId,state:e.state(doc),png:canvas.toDataURL()};
    const path="/src/tools/brush.ts",tools:typeof import("../../src/tools/brush")=await import(/* @vite-ignore */path);
    if(!tools.beginBrush([1500,1000],false))throw Error("Could not start stroke");tools.moveBrush([1530,1000]);
  });
  await expect.poll(()=>page.evaluate(()=>{
    const a=(window as any).__compositor,b=(window as any).warpRenderBaseline;
    return a.engine.state(b.doc).layers.find((l:any)=>l.id===b.layer).pixelsWidth;
  })).toBeLessThanOrEqual(1024);
  await expect.poll(()=>page.evaluate(()=>{
    const a=(window as any).__compositor,b=(window as any).warpRenderBaseline;
    return a.store.getState().documents[b.doc].layers.find((l:any)=>l.id===b.layer).pixelsWidth;
  })).toBeLessThanOrEqual(1024);
  await expect.poll(()=>page.evaluate(()=>{
    const canvas=document.querySelector('[data-testid="canvas-view"] canvas') as HTMLCanvasElement;
    return canvas.toDataURL()!==(window as any).warpRenderBaseline.png;
  })).toBe(true);
  await page.keyboard.press("Escape");
  const restored=await page.evaluate(()=>{
    const a=(window as any).__compositor,b=(window as any).warpRenderBaseline,s=a.store.getState().documents[b.doc];
    return{layer:s.layers.find((l:any)=>l.id===b.layer),depth:s.undoDepth,original:b.state.layers.find((l:any)=>l.id===b.layer),originalDepth:b.state.undoDepth};
  });
  expect(restored.layer).toEqual(restored.original);expect(restored.depth).toBe(restored.originalDepth);
  await expect.poll(()=>page.evaluate(()=>{
    const canvas=document.querySelector('[data-testid="canvas-view"] canvas') as HTMLCanvasElement;
    return canvas.toDataURL()===(window as any).warpRenderBaseline.png;
  })).toBe(true);
});
test("6 MP production stroke uses a bounded live preview and full-resolution guarded commit",async({page})=>{
  await page.goto("/");await expect(page.getByTestId("engine-ready")).toBeVisible();
  const ids=await page.evaluate(async()=>{
    const a=(window as any).__compositor,e=a.engine,s=a.store.getState();
    const bytes=new Uint8Array(3000*2000*4);for(let i=0;i<bytes.length;i+=4){bytes[i+3]=255;if(i/4%3000===1000)bytes.fill(255,i,i+4);}
    const doc=e.newDocument(3000,2000,true);e.pastePixels(doc,3000,2000,bytes.buffer,[0,0]);const layer=e.state(doc).activeLayerId;
    s.openDocument(doc);s.setTool("blur");s.setBrushOptions({smearMode:"Smudge",diameter:2,hardness:0.5,strength:0.5});
    const previews:number[]=[];const keep=e.keepJobPreview.bind(e);e.keepJobPreview=(...args:any[])=>{previews.push(args[4].byteLength);return keep(...args);};(window as any).warpLiveSizes=previews;
    const path="/src/tools/brush.ts",tools:typeof import("../../src/tools/brush")=await import(/* @vite-ignore */path);
    if(!tools.beginBrush([1000,1000],false))throw Error("Could not start production stroke");tools.moveBrush([1003,1000]);
    return{doc,layer,depth:e.state(doc).undoDepth};
  });
  await expect.poll(()=>page.evaluate(()=>(window as any).warpLiveSizes.length)).toBeGreaterThan(0);
  const preview=await page.evaluate(ids=>{const e=(window as any).__compositor.engine;return{sizes:(window as any).warpLiveSizes,stored:Array.from(new Uint8Array(e.jobInput(ids.doc,ids.layer).pixels).slice((1000*3000+1001)*4,(1000*3000+1001)*4+4)),depth:e.state(ids.doc).undoDepth};},ids);
  expect(Math.max(...preview.sizes)).toBeLessThanOrEqual(4*1024*1024);expect(preview.stored).toEqual([0,0,0,255]);expect(preview.depth).toBe(ids.depth);
  await page.evaluate(async()=>{const path="/src/tools/brush.ts",tools:typeof import("../../src/tools/brush")=await import(/* @vite-ignore */path);tools.finishBrush();});
  await expect.poll(()=>page.evaluate(()=>(window as any).__compositor.store.getState().working)).toBe(false);
  await expect(page.getByTestId("error-banner")).toHaveCount(0);
  const result=await page.evaluate(ids=>{const e=(window as any).__compositor.engine;const read=()=>Array.from(new Uint8Array(e.jobInput(ids.doc,ids.layer).pixels).slice((1000*3000+1001)*4,(1000*3000+1001)*4+4));const after=read(),state=e.state(ids.doc);e.undo(ids.doc);return{after,undo:read(),depth:state.undoDepth,layer:state.layers.find((l:any)=>l.id===ids.layer)};},ids);
  expect(result.after).toEqual([128,128,128,255]);expect(result.undo).toEqual([0,0,0,255]);expect(result.depth).toBe(ids.depth+1);expect(result.layer).toMatchObject({pixelsWidth:3000,pixelsHeight:2000});
});
test("large canvases refuse missing float GPU targets before source-plane allocation",async({page})=>{
  await page.goto("/");await expect(page.getByTestId("engine-ready")).toBeVisible();
  const error=await page.evaluate(async()=>{
    const path="/src/engine/warp-client.ts",{WarpClient}:typeof import("../../src/engine/warp-client")=await import(/* @vite-ignore */path);
    const e=(window as any).__compositor.engine,doc=e.newDocument(3000,2000,true);e.pastePixels(doc,1,1,new Uint8Array([255,255,255,255]).buffer,[1000,1000]);
    const snapshot=e.jobInput(doc,e.state(doc).activeLayerId);
    const script=`OffscreenCanvas.prototype.getContext=()=>null; await import(${JSON.stringify(new URL("/src/engine/warp-worker.ts",location.href).href)});postMessage({type:"boot"});`;
    const url=URL.createObjectURL(new Blob([script],{type:"text/javascript"})),worker=new Worker(url,{type:"module"});
    await new Promise<void>((resolve,reject)=>{worker.onmessage=()=>resolve();worker.onerror=e=>reject(Error(e.message));});
    const c=new WarpClient(worker,e.module,snapshot,{mode:"Smudge",diameter:2,hardness:0.5,strength:0.5},()=>{},()=>{});
    try{c.append([1000,1000]);await c.finish(1);return "unexpected success";}catch(error){return String(error);}
    finally{c.dispose();URL.revokeObjectURL(url);}
  });
  expect(error).toContain("WebGL2 float render targets");expect(error).toContain("4 Mi pixels");
});
