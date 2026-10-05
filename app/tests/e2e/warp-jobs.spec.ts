import { test, expect, type Page } from "@playwright/test";
import type { WarpMode } from "../../src/engine/types";

// This tests the real WASM job protocol before the Phase 6 tool rail/GL preview
// is exposed. Native reference tests alone cannot prove the worker ABI works.
async function setup(page: Page, selection: boolean) {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  return page.evaluate(async selection => {
    const api = (window as any).__compositor;
    const canvas = document.createElement("canvas"); canvas.width=20;canvas.height=16;
    const ctx=canvas.getContext("2d")!;ctx.fillStyle="#000";ctx.fillRect(0,0,20,16);
    ctx.fillStyle="#fff";ctx.fillRect(3,0,1,16);
    const blob=await new Promise<Blob>(resolve=>canvas.toBlob(b=>resolve(b!),"image/png"));
    const png=new Uint8Array(await blob.arrayBuffer());
    const make=()=>{const doc=api.engine.newDocument(20,16,true);api.engine.importImage(doc,png,"Stripe",{x:10,y:8});return doc;};
    const worker=make(), direct=make();
    const layer=api.engine.state(worker).activeLayerId, other=api.engine.state(direct).activeLayerId;
    if(selection){const box={type:"SelectShape",kind:"Rectangle",points:[[4,0],[5,0],[5,16],[4,16]],mode:"Replace",antialiased:false};api.engine.execute(worker,box);api.engine.execute(direct,box);}
    api.store.getState().openDocument(worker);
    const jobs=api.store.getState().jobs, run=jobs.run.bind(jobs), kinds:string[]=[];
    jobs.run=(channel:string,request:{kind:string})=>{kinds.push(request.kind);return run(channel,request);};
    (window as any).__warpJobKinds=kinds;
    const before=Array.from(api.engine.composite(worker,{x:0,y:0,width:20,height:16},20,16));
    return {worker,direct,layer,other,before,depth:api.engine.state(worker).undoDepth};
  },selection);
}

for(const mode of ["Liquify","Smudge"] as const) for(const selected of [false,true]) {
  test(`${mode}: actual worker command, ${selected?"selected":"whole"} writeback, one undo and redo`,async({page})=>{
    const ids=await setup(page,selected);
    const report=await page.evaluate(async({ids,mode}:{ids:any;mode:WarpMode})=>{
      const api=(window as any).__compositor;
      const warp={mode,diameter:2,hardness:0.5,strength:0.5,points:[[3,8],[6,8]]};
      const installed=await api.store.getState().runEditJob({type:"WarpStroke",id:ids.layer,mask:false,warp},ids.layer);
      api.engine.execute(ids.direct,{type:"WarpStroke",id:ids.other,mask:false,warp});
      const region={x:0,y:0,width:20,height:16};
      const after=Array.from(api.engine.composite(ids.worker,region,20,16));
      const direct=Array.from(api.engine.composite(ids.direct,region,20,16));
      const depth=api.engine.state(ids.worker).undoDepth;
      api.engine.undo(ids.worker);const undone=Array.from(api.engine.composite(ids.worker,region,20,16));
      api.engine.redo(ids.worker);const redone=Array.from(api.engine.composite(ids.worker,region,20,16));
      return {installed,after,direct,depth,undone,redone,kinds:(window as any).__warpJobKinds,error:api.store.getState().error};
    },{ids,mode});
    expect(report.installed).toBe(true);expect(report.error).toBeNull();
    expect(report.kinds).toEqual(["edit"]);
    expect(report.after).toEqual(report.direct);
    expect(report.after.slice((8*20+4)*4,(8*20+4)*4+4)).toEqual([128,128,128,255]);
    if(selected) expect(report.after.slice((8*20+5)*4,(8*20+5)*4+4)).toEqual([0,0,0,255]);
    expect(report.depth).toBe(ids.depth+1);expect(report.undone).toEqual(ids.before);expect(report.redone).toEqual(report.after);
  });
}

test("pickup and sub-spacing worker jobs preserve pixels, history and redo",async({page})=>{
  const ids=await setup(page,false);
  const report=await page.evaluate(async ids=>{
    const api=(window as any).__compositor;
    api.engine.execute(ids.worker,{type:"InvertPixels",id:ids.layer,mask:false});
    api.engine.undo(ids.worker);
    const before=api.engine.state(ids.worker), results=[];
    for(const mode of ["Liquify","Smudge"]) for(const points of [[[3,8]],[[3,8],[3.5,8]]]) {
      const installed=await api.store.getState().runEditJob({type:"WarpStroke",id:ids.layer,mask:false,
        warp:{mode,diameter:2,hardness:0.5,strength:0.5,points}},ids.layer);
      const state=api.engine.state(ids.worker);
      results.push({installed,depth:state.undoDepth,canRedo:state.canRedo,
        pixels:Array.from(api.engine.composite(ids.worker,{x:0,y:0,width:20,height:16},20,16))});
    }
    return {before,results,error:api.store.getState().error};
  },ids);
  expect(report.before.canRedo).toBe(true);expect(report.error).toBeNull();
  for(const result of report.results) {
    expect(result.installed).toBe(true);expect(result.depth).toBe(report.before.undoDepth);
    expect(result.canRedo).toBe(true);expect(result.pixels).toEqual(ids.before);
  }
});

test("warp result from a changed layer is refused by the real worker/store path",async({page})=>{
  const ids=await setup(page,false);
  const report=await page.evaluate(async ids=>{
    const api=(window as any).__compositor, s=api.store.getState(), jobs=s.jobs;
    const original=jobs.run.bind(jobs);
    jobs.run=async(channel:string,request:any)=>{const result=await original(channel,request);api.engine.execute(ids.worker,{type:"NudgeLayers",ids:[ids.layer],dx:1,dy:0});return result;};
    const installed=await s.runEditJob({type:"WarpStroke",id:ids.layer,mask:false,
      warp:{mode:"Smudge",diameter:2,hardness:0.5,strength:0.5,points:[[3,8],[6,8]]}},ids.layer);
    const state=api.engine.state(ids.worker);
    api.engine.execute(ids.direct,{type:"NudgeLayers",ids:[ids.other],dx:1,dy:0});
    const region={x:0,y:0,width:20,height:16};
    const untouched=Array.from(api.engine.composite(ids.worker,region,20,16));
    const onlyNudge=Array.from(api.engine.composite(ids.direct,region,20,16));
    return {installed,depth:state.undoDepth,working:api.store.getState().working,error:api.store.getState().error,untouched,onlyNudge};
  },ids);
  expect(report.installed).toBe(false);expect(report.working).toBe(false);
  expect(report.error).toContain("The layer changed");expect(report.depth).toBe(ids.depth+1);
  expect(report.untouched).toEqual(report.onlyNudge);
});
