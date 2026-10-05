import {test,expect} from "@playwright/test";
import type {JobResult} from "../../src/engine/jobs";

test("6 MP document completes the real GPU/worker path beyond the CPU reference limit",async({page})=>{
  await page.goto("/");await expect(page.getByTestId("engine-ready")).toBeVisible();
  const r=await page.evaluate(async()=>{
    const path="/src/canvas/gpu-warp.ts",{GpuWarpStroke}:typeof import("../../src/canvas/gpu-warp")=await import(/* @vite-ignore */path);
    const api=(window as any).__compositor,e=api.engine,jobs=api.store.getState().jobs;
    const doc=e.newDocument(3000,2000,true),bytes=new Uint8Array(20*16*4);
    for(let i=0;i<bytes.length;i+=4){bytes[i+3]=255;if(i/4%20===3)bytes.fill(255,i,i+4);}
    e.pastePixels(doc,20,16,bytes.buffer,[0,0]);const layer=e.state(doc).activeLayerId,depth=e.state(doc).undoDepth;
    const warp={mode:"Smudge" as const,diameter:2,hardness:0.5,strength:0.5,points:[[3,8],[6,8]] as [number,number][]};
    let refused="";try{e.execute(doc,{type:"WarpStroke",id:layer,mask:false,warp});}catch(err){refused=String(err);}
    const original=await e.jobInputAsync(doc,layer),input=original.input;
    const source:JobResult=await jobs.run("warp",{kind:"warpSource",...original,maskTarget:false});
    const shape=JSON.parse(source.header!),gl=document.createElement("canvas").getContext("webgl2")!;
    const stroke=new GpuWarpStroke(gl,shape.width,shape.height,new Uint8Array(source.pixels!),warp);
    try{
      warp.points.forEach(p=>stroke.append(p));const tiles=stroke.readTiles();
      const transferred:JobResult=await jobs.run("warp",{kind:"warpResult",input,...source.inputs!,maskTarget:false,warp:JSON.stringify(warp),tiles:tiles.map(t=>({...t,pixels:t.pixels.buffer})),outPerDoc:0.2});
      await e.installJobAsync(doc,layer,input,transferred.header,transferred.pixels,transferred.mask,transferred.display??null);
      const after:ArrayLike<number>=e.layerPixels(doc,layer),delta=e.state(doc).undoDepth-depth;
      const red=[4,5,6].map(x=>after[(8*20+x)*4]);const uploaded=stroke.diagnostics().uploadedBytes;
      e.undo(doc);const undoRed=[4,5,6].map(x=>e.layerPixels(doc,layer)[(8*20+x)*4]);
      return{refused,shape,delta,red,undoRed,uploaded,readback:tiles.reduce((n,t)=>n+t.width*t.height*4,0)};
    }finally{stroke.dispose();gl.getExtension("WEBGL_lose_context")?.loseContext();}
  });
  expect(r.refused).toContain("limited to 4 Mi pixels");expect(r.shape).toEqual({width:3000,height:2000});
  expect(r.red).toEqual([128,64,32]);expect(r.undoRed).toEqual([0,0,0]);expect(r.delta).toBe(1);
  expect(r.uploaded).toBe(24_000_000);expect(r.readback).toBeLessThan(1024*1024);
});
