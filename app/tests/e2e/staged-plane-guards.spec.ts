import {test,expect} from "@playwright/test";

test("staged pixel, mask and display planes keep exact bytes and reject partial or cancelled transfers",async({page})=>{
  await page.goto("/");await expect(page.getByTestId("engine-ready")).toBeVisible();
  const result=await page.evaluate(()=>{
    const a=(window as any).__compositor,w=a.engine.wasm,doc=a.engine.newDocument(8,8,true);
    const id=a.engine.state(doc).layers[0].id;
    a.engine.execute(doc,{type:"Fill",id,mask:false,color:[1,1,1]});
    a.engine.execute(doc,{type:"AddMask",id,revealing:true});
    const header=JSON.parse(a.engine.jobHeader(doc,id));
    const output=JSON.stringify({transform:header.layer.transform,maskPlacement:null,pixels:[8,8],mask:[8,8],regions:[],display:{level:1,width:4,height:4}});
    const original=a.engine.layerPixels(doc,id).slice(),originalMask=a.engine.maskPixels(doc,id).slice(),depth=a.engine.state(doc).undoDepth;
    const same=(actual:Uint8Array,expected:Uint8Array)=>actual.length===expected.length&&actual.every((b,i)=>b===expected[i]);
    const unchanged=()=>a.engine.state(doc).undoDepth===depth&&same(a.engine.layerPixels(doc,id),original)&&same(a.engine.maskPixels(doc,id),originalMask);
    const rejects=(action:()=>void,text:string)=>{try{action();return false;}catch(error){return String(error).includes(text);}finally{w.cancel_staged_install();}};
    const budget=[0,1,2].map(plane=>rejects(()=>{const sizes=[0,0,0];sizes[plane]=plane===1?100000001:400000001;w.begin_staged_install(...sizes);},"pixel budget"));
    const overflow=[0,1,2].map(plane=>rejects(()=>{w.begin_staged_install(1,1,1);w.append_staged_install(plane,new Uint8Array(2));},"overflow"));
    const invalid=rejects(()=>{w.begin_staged_install(1,1,1);w.append_staged_install(3,new Uint8Array(1));},"invalid staged plane");
    const incomplete=[0,1,2].map(missing=>rejects(()=>{w.begin_staged_install(256,64,64);for(let plane=0;plane<3;plane++)w.append_staged_install(plane,new Uint8Array((plane===0?256:64)-(plane===missing?1:0)));w.finish_staged_install(doc,id,JSON.stringify(header.stamp),output,false);},"incomplete"));
    w.begin_staged_install(5*1024*1024,1024,64);w.append_staged_install(0,new Uint8Array(4*1024*1024));w.append_staged_install(1,new Uint8Array(1024));w.cancel_staged_install();
    const cancelled=rejects(()=>w.append_staged_install(0,new Uint8Array(1)),"no staged edit"),beforeCommit=unchanged();
    const pixels=new Uint8Array(256);for(let i=0;i<pixels.length;i+=4)pixels.set([128,64,192,255],i);
    const mask=new Uint8Array(64).fill(123),display=pixels.slice(0,64),planes=[pixels,mask,display];
    w.begin_staged_install(...planes.map(p=>p.length));
    for(let plane=0;plane<3;plane++)for(let at=0;at<planes[plane].length;at+=7)w.append_staged_install(plane,planes[plane].subarray(at,at+7));
    w.finish_staged_install(doc,id,JSON.stringify(header.stamp),output,false);w.cancel_staged_install();
    const exactPixels=same(a.engine.layerPixels(doc,id),pixels),exactMask=same(a.engine.maskPixels(doc,id),mask),exactDisplay=same(a.engine.layerPixels(doc,id,1),display),oneUndo=a.engine.state(doc).undoDepth===depth+1;
    a.engine.undo(doc);const undo=unchanged();a.engine.redo(doc);
    return{budget,overflow,invalid,incomplete,cancelled,beforeCommit,exactPixels,exactMask,exactDisplay,oneUndo,undo,redo:same(a.engine.layerPixels(doc,id),pixels)&&same(a.engine.maskPixels(doc,id),mask)};
  });
  expect(result).toEqual({budget:[true,true,true],overflow:[true,true,true],invalid:true,incomplete:[true,true,true],cancelled:true,beforeCommit:true,exactPixels:true,exactMask:true,exactDisplay:true,oneUndo:true,undo:true,redo:true});
});
