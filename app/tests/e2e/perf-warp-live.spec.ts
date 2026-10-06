/// <reference types="vite/client" />
import {test,expect} from "@playwright/test";
// Complete production controller/worker evidence on hardware Edge. Independent
// from the original 29 PERF cases and the two raw GPU kernel cases.
test.skip(!process.env.GPU_WARP_UI_PERF,"set GPU_WARP_UI_PERF=1 on hardware WebGL2");
test.use({channel:"msedge",viewport:{width:1440,height:900}});
for(const [name,width,height] of [["24 MP",6000,4000],["100 MP",10000,10000]] as const)test(`production warp on ${name}: live response, full commit and original history budget`,async({page})=>{
  test.setTimeout(180_000);await page.goto("/");await expect(page.getByTestId("engine-ready")).toBeVisible();
  const measures=await page.evaluate(async({width,height})=>{
    const a=(window as any).__compositor,e=a.engine;
    const path="/src/tools/brush.ts",tools:typeof import("../../src/tools/brush")=await import(/* @vite-ignore */path);
    const frame=()=>new Promise<void>(resolve=>requestAnimationFrame(()=>resolve()));
    const wait=async(valid:()=>boolean)=>{const start=performance.now();while(!valid()){if(a.store.getState().error)throw Error(a.store.getState().error);if(performance.now()-start>30_000)throw Error("Production warp exceeded 30 seconds");await frame();}};
    const measures=[];
    for(const mode of ["Liquify","Smudge"] as const){
      const make=()=>{const bytes=new Uint8Array(width*height*4);new Uint32Array(bytes.buffer).fill(0xff000000);for(let y=0;y<height;y++)bytes.fill(255,(y*width+width/2)*4,(y*width+width/2+1)*4);
        const doc=e.newDocument(width,height,true);e.pastePixels(doc,width,height,bytes.buffer,[0,0]);return doc;};
      const doc=make(),s=a.store.getState();s.openDocument(doc);s.setTool("blur");s.setBrushOptions({smearMode:mode,diameter:40,hardness:0.5,strength:0.5});
      await frame();await frame();const state=e.state(doc),layer=state.activeLayerId,vp=a.store.getState().viewports[doc];vp.setZoom(1,vp.center,state);s.invalidate();await frame();await frame();
      const canvas=document.querySelector('[data-testid="canvas-view"] canvas') as HTMLCanvasElement|null;
      const gl=canvas?.getContext("webgl2")??document.createElement("canvas").getContext("webgl2")!;
      const info=gl.getExtension("WEBGL_debug_renderer_info"),gpu=info?String(gl.getParameter(info.UNMASKED_RENDERER_WEBGL)):"unknown";
      let previews=0,previewBytes=0;const original=e.keepJobPreview.bind(e);e.keepJobPreview=(...args:any[])=>{const value=original(...args);previews++;previewBytes=Math.max(previewBytes,args[4].byteLength);return value;};
      const x=width/2,y=height/2,preparation=performance.now();
      const region={x:x-25,y:y-25,width:80,height:60},baseline=e.composite(doc,region,80,60).slice();
      if(!tools.beginBrush([x,y],false))throw Error("Production stroke did not start");tools.moveBrush([x+1,y]);await wait(()=>previews>0);
      const preparationMs=performance.now()-preparation;
      let maxFrameGapMs=0,lastFrame=performance.now(),running=true;
      const tick=()=>{if(!running)return;const now=performance.now();maxFrameGapMs=Math.max(maxFrameGapMs,now-lastFrame);lastFrame=now;requestAnimationFrame(tick);};requestAnimationFrame(tick);
      let maxInputMs=0,maxPreviewMs=0;
      for(const p of [[x+10,y],[x+20,y+4],[x+30,y],[x+40,y+4]] as [number,number][]){const count=previews,start=performance.now();tools.moveBrush(p);maxInputMs=Math.max(maxInputMs,performance.now()-start);await wait(()=>previews>count);maxPreviewMs=Math.max(maxPreviewMs,performance.now()-start);}
      await frame();running=false;
      const finish=performance.now();tools.finishBrush();await wait(()=>!a.store.getState().working);const finishMs=performance.now()-finish;
      const after=e.state(doc),image=e.composite(doc,region,80,60),changedChannels=image.reduce((n:number,b:number,i:number)=>n+(b!==baseline[i]?1:0),0);
      e.undo(doc);const undo=Array.from(e.composite(doc,{x:x+1,y,width:1,height:1},1,1));
      const undoImage=e.composite(doc,region,80,60),undoPreservesResult=undoImage.every((b:number,i:number)=>b===image[i]);
      measures.push({mode,gpu,preparationMs,maxInputMs,maxPreviewMs,maxFrameGapMs,finishMs,previewBytes,changedChannels,undo,undoPreservesResult,canUndo:after.canUndo,undoDepth:after.undoDepth,depth:after.undoDepth-state.undoDepth,dimensions:after.layers.find((l:any)=>l.id===layer),wasmBytes:e.wasmBytes()});
      e.keepJobPreview=original;a.store.getState().closeDocument(doc);await frame();await frame();
    }return measures;
  },{width,height});
  console.log(JSON.stringify({size:name,measures}));
  for(const r of measures){expect(r.gpu).not.toMatch(/SwiftShader|Basic Render|unknown/i);expect(r.preparationMs).toBeLessThan(30_000);expect(r.maxInputMs).toBeLessThan(100);expect(r.maxPreviewMs).toBeLessThan(250);expect(r.maxFrameGapMs).toBeLessThan(100);expect(r.finishMs).toBeLessThan(30_000);expect(r.previewBytes).toBeLessThanOrEqual(4*1024*1024);expect(r.dimensions).toMatchObject({pixelsWidth:width,pixelsHeight:height});expect(r.changedChannels).toBeGreaterThan(0);
    // Existing Mac-compatible History has a 256 MiB retained-pixel budget.
    // A full 100 MP replacement (400 MB) cannot retain its previous raster.
    if(width*height*4>256*1024*1024){expect(r.canUndo).toBe(false);expect(r.undoDepth).toBe(0);expect(r.undoPreservesResult).toBe(true);}
    else{expect(r.depth).toBe(1);expect(r.undo).toEqual([0,0,0,255]);expect(r.undoPreservesResult).toBe(false);}
  }
});
