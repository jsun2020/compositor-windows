import {test,expect,type Page} from "@playwright/test";
import type {WarpMode} from "../../src/engine/types";
import type {JobResult} from "../../src/engine/jobs";

async function setup(page:Page,selected=false,styled=false){
  await page.goto("/");await expect(page.getByTestId("engine-ready")).toBeVisible();
  return page.evaluate(({selected,styled})=>{
    const api=(window as any).__compositor,engine=api.engine;
    const source=new Uint8Array(20*16*4);for(let i=0;i<source.length;i+=4){source[i+3]=255;if(i/4%20===3)source.fill(255,i,i+4);}
    const make=()=>{const doc=engine.newDocument(20,16,true);engine.pastePixels(doc,20,16,source.slice().buffer,[0,0]);
      const layer=engine.state(doc).activeLayerId;
      if(selected)engine.execute(doc,{type:"SelectShape",kind:"Rectangle",points:[[4,0],[5,0],[5,16],[4,16]],mode:"Replace",antialiased:false});
      if(styled){engine.execute(doc,{type:"AddMask",id:layer,revealing:true});engine.execute(doc,{type:"SetLayerOpacity",id:layer,opacity:0.3});
        engine.execute(doc,{type:"SetLayerBlendMode",id:layer,mode:"Multiply"});
        engine.execute(doc,{type:"SetLayerEffects",id:layer,effects:{stroke:{red:0,green:0,blue:1,opacity:1,size:2,inside:false},shadow:{red:0,green:0,blue:0,opacity:0.5,distance:2,angle:90,blur:1}}});}
      return{doc,layer};};
    const actual=make(),oracle=make();api.store.getState().openDocument(actual.doc);
    return{actual,oracle,depth:engine.state(actual.doc).undoDepth,before:Array.from(engine.layerPixels(actual.doc,actual.layer))};
  },{selected,styled});
}

// Real source worker -> real tiled GL -> real result worker. This deliberately
// does not claim live pointer preview/tool UI acceptance.
async function run(page:Page,ids:Awaited<ReturnType<typeof setup>>,mode:WarpMode,action:"commit"|"preview"|"pickup"|"stale",change=0){
  return page.evaluate(async({ids,mode,action,change})=>{
    const path="/src/canvas/gpu-warp.ts",{GpuWarpStroke}:typeof import("../../src/canvas/gpu-warp")=await import(/* @vite-ignore */path);
    const api=(window as any).__compositor,e=api.engine,jobs=api.store.getState().jobs;
    const original=await e.jobInputAsync(ids.actual.doc,ids.actual.layer),input=original.input;
    const prepared:JobResult=await jobs.run("warp-source",{kind:"warpSource",...original,maskTarget:false});
    const sourceSize=JSON.parse(prepared.header!),snapshot=prepared.inputs!;
    const snapshotEqual=Array.from(new Uint8Array(snapshot.pixels!)).every((b,i)=>b===ids.before[i]);
    const rawStripe=Array.from(new Uint8Array(prepared.pixels!).slice((8*20+3)*4,(8*20+3)*4+4));
    const gl=document.createElement("canvas").getContext("webgl2",{antialias:false})!;
    const settings={mode,diameter:2,hardness:0.5,strength:0.5},points: [number,number][]=action==="pickup"?[[3,8],[3.5,8]]:[[3,8],[6,8]];
    const stroke=new GpuWarpStroke(gl,sourceSize.width,sourceSize.height,new Uint8Array(prepared.pixels!),settings,{sourceTileSide:8,workTileSide:8});
    try{
      points.forEach(p=>stroke.append(p));const tiles=stroke.readTiles(),readback=tiles.reduce((n,t)=>n+t.pixels.byteLength,0);
      const result:JobResult=await jobs.run("warp-result",{kind:"warpResult",input,...snapshot,maskTarget:false,warp:JSON.stringify({...settings,points}),
        tiles:tiles.map(t=>({...t,pixels:t.pixels.buffer})),outPerDoc:0.2});
      const warp={type:"WarpStroke",id:ids.oracle.layer,mask:false,warp:{...settings,points}};
      e.execute(ids.oracle.doc,warp);
      const expected=Array.from(e.layerPixels(ids.oracle.doc,ids.oracle.layer)),beforeState=e.state(ids.actual.doc);
      let refusal="",previewChanged=false,previewStored=true;
      const region={x:0,y:0,width:20,height:16},render=()=>Array.from(e.composite(ids.actual.doc,region,20,16));
      if(action==="preview"){
        const before=render();e.keepJobPreview(ids.actual.doc,ids.actual.layer,input,result.header,result.pixels,result.mask,result.display??null);
        previewChanged=JSON.stringify(render())!==JSON.stringify(before);
        previewStored=JSON.stringify(Array.from(new Uint8Array(e.jobInput(ids.actual.doc,ids.actual.layer).pixels)))===JSON.stringify(ids.before);
        e.setPreview(ids.actual.doc,null);
      }else{
        if(action==="stale"){
          const commands=[{type:"InvertPixels",id:ids.actual.layer,mask:false},{type:"NudgeLayers",ids:[ids.actual.layer],dx:1,dy:0},
            {type:"CanvasSize",width:21,height:16,anchor:0,fill:null},{type:"SelectAll"},{type:"InvertPixels",id:ids.actual.layer,mask:true}];
          e.execute(ids.actual.doc,commands[change]);
        }
        const immediatelyBefore=Array.from(e.layerPixels(ids.actual.doc,ids.actual.layer));
        try{await e.installJobAsync(ids.actual.doc,ids.actual.layer,input,result.header,result.pixels,result.mask,result.display??null);}
        catch(err){refusal=String(err);}
        if(action==="stale"&&JSON.stringify(immediatelyBefore)!==JSON.stringify(Array.from(e.layerPixels(ids.actual.doc,ids.actual.layer))))throw Error("stale result changed pixels");
      }
      const after=Array.from(e.layerPixels(ids.actual.doc,ids.actual.layer)),afterState=e.state(ids.actual.doc);
      const composite=render(),oracleComposite=Array.from(e.composite(ids.oracle.doc,region,20,16));
      let undone:number[]=[],redone:number[]=[];
      if(action==="commit"){e.undo(ids.actual.doc);undone=Array.from(e.layerPixels(ids.actual.doc,ids.actual.layer));e.redo(ids.actual.doc);redone=Array.from(e.layerPixels(ids.actual.doc,ids.actual.layer));}
      const states=afterState.layers.find((l:any)=>l.id===ids.actual.layer);
      stroke.dispose();return{snapshotEqual,rawStripe,originalDetached:original.pixels.byteLength===0,snapshotDetached:snapshot.pixels!.byteLength===0,
        readback,after,expected,composite,oracleComposite,undone,redone,beforeState,afterState,states,refusal,previewChanged,previewStored,diagnostic:stroke.diagnostics()};
    }finally{stroke.dispose();gl.getExtension("WEBGL_lose_context")?.loseContext();}
  },{ids,mode,action,change});
}

for(const mode of ["Liquify","Smudge"] as const)for(const selected of [false,true])test(`${mode}: GPU tiles round-trip through workers with ${selected?"selected":"whole"} replacement and undo`,async({page})=>{
  const ids=await setup(page,selected,true),r=await run(page,ids,mode,"commit");
  expect(r.snapshotEqual).toBe(true);expect(r.rawStripe).toEqual([255,255,255,255]);
  expect(r.originalDetached).toBe(true);expect(r.snapshotDetached).toBe(true);
  expect(r.after).toEqual(r.expected);expect(r.composite).toEqual(r.oracleComposite);
  expect(r.after.slice((8*20+4)*4,(8*20+4)*4+4)).toEqual([128,128,128,255]);
  expect(r.undone).toEqual(ids.before);expect(r.redone).toEqual(r.after);
  expect(r.afterState.undoDepth).toBe(ids.depth+1);expect(r.readback).toBeLessThan(20*16*4);
  expect(r.states).toMatchObject({opacity:0.3,blendMode:"Multiply",hasMask:true});expect(r.states.effects.stroke.size).toBe(2);
  expect(r.diagnostic).toMatchObject({disposed:true,allocatedBytes:0,liveTextures:0});
});

test("GPU transported result previews styled pixels; cancel preserves stored revisions and history",async({page})=>{
  const ids=await setup(page,true,true),r=await run(page,ids,"Smudge","preview");
  expect(r.previewChanged).toBe(true);expect(r.previewStored).toBe(true);expect(r.after).toEqual(ids.before);
  expect(r.afterState).toEqual(r.beforeState);expect(r.afterState.undoDepth).toBe(ids.depth);
});
test("GPU pickup and sub-spacing preserve redo",async({page})=>{
  const ids=await setup(page);await page.evaluate(ids=>{const e=(window as any).__compositor.engine;e.execute(ids.actual.doc,{type:"InvertPixels",id:ids.actual.layer,mask:false});e.undo(ids.actual.doc);},ids);
  const r=await run(page,ids,"Liquify","pickup");expect(r.readback).toBe(0);expect(r.after).toEqual(ids.before);
  expect(r.afterState.undoDepth).toBe(ids.depth);expect(r.afterState.canRedo).toBe(true);
});
for(let change=0;change<5;change++)test(`GPU result refuses stale ${["pixels","transform","canvas","selection","mask"][change]} snapshot`,async({page})=>{
  const ids=await setup(page,false,true),r=await run(page,ids,"Smudge","stale",change);
  expect(r.refusal).toContain("The layer changed");expect(r.afterState.undoDepth).toBe(ids.depth+1);
});
test("mask-target source and malformed tiles refuse cleanly and the real worker remains usable",async({page})=>{
  const ids=await setup(page),r=await page.evaluate(async ids=>{
    const api=(window as any).__compositor,e=api.engine,jobs=api.store.getState().jobs;
    const errors:string[]=[];
    try{await jobs.run("warp",{kind:"warpSource",...e.jobInput(ids.actual.doc,ids.actual.layer),maskTarget:true});}catch(err){errors.push(String(err));}
    const spec={mode:"Smudge",diameter:2,hardness:0.5,strength:0.5,points:[[3,8],[6,8]]};
    for(const tile of [{x:0,y:0,width:1,height:1,pixels:new Uint8Array([255,0,0,0]).buffer},{x:0,y:0,width:1,height:1,pixels:new Uint8Array(3).buffer}]){
      try{await jobs.run("warp",{kind:"warpResult",...e.jobInput(ids.actual.doc,ids.actual.layer),maskTarget:false,warp:JSON.stringify(spec),tiles:[tile],outPerDoc:1});}catch(err){errors.push(String(err));}
    }
    const valid:JobResult=await jobs.run("warp",{kind:"warpSource",...e.jobInput(ids.actual.doc,ids.actual.layer),maskTarget:false});
    return{errors,width:JSON.parse(valid.header!).width,depth:e.state(ids.actual.doc).undoDepth};
  },ids);
  expect(r.errors[0]).toContain("not its mask");expect(r.errors.slice(1)).toEqual([expect.stringContaining("invalid warp tiles"),expect.stringContaining("invalid warp tiles")]);
  expect(r.width).toBe(20);expect(r.depth).toBe(ids.depth);
});
