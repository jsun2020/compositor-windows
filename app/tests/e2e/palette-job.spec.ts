import {test,expect} from "@playwright/test";

test("a wide palette gradient matches every in-place byte with one Undo/Redo",async({page})=>{
  test.setTimeout(120_000);
  await page.goto("/");await expect(page.getByTestId("engine-ready")).toBeVisible();
  const result=await page.evaluate(async()=>{
    const a=(window as any).__compositor,e=a.engine,s=()=>a.store.getState(),w=1536,h=1024;
    const make=()=>{const doc=e.newDocument(w,h,true),id=e.state(doc).layers[0].id;e.execute(doc,{type:"Fill",id,mask:false,color:[.2,.4,.6]});return{doc,id};};
    const actual=make(),reference=make();s().openDocument(actual.doc);
    const before=e.layerPixels(actual.doc,actual.id).slice(),depth=e.state(actual.doc).undoDepth;
    const command={type:"Gradient",id:actual.id,mask:false,gradient:{shape:"Linear",start:[0,0],end:[w,h],from:[1,0,0,1],to:[0,1,1,1],opacity:.9}};
    const client=s().jobs,run=client.run.bind(client);let receipt:any=null;
    client.run=async(...args:any[])=>{const r=await run(...args);if(args[1].kind==="edit")receipt={header:JSON.parse(r.header),bytes:r.pixels?.byteLength??0};return r;};
    try{
      const applied=await s().runEditJob(command,actual.id);
      e.execute(reference.doc,{...command,id:reference.id});const expected=e.layerPixels(reference.doc,reference.id).slice();
      const equals=(bytes:Uint8Array)=>{const p=e.layerPixels(actual.doc,actual.id);return p.length===bytes.length&&p.every((v:number,i:number)=>v===bytes[i]);};
      const exact=equals(expected),oneStep=e.state(actual.doc).undoDepth===depth+1;
      s().undo();const undo=equals(before);s().redo();
      return{applied,receipt,exact,oneStep,undo,redo:equals(expected),error:s().error};
    }finally{client.run=run;s().closeDocument(actual.doc);e.closeDocument(reference.doc);}
  });
  expect(result.error).toBeNull();expect(result.applied).toBe(true);expect(result.exact).toBe(true);expect(result.oneStep).toBe(true);expect(result.undo).toBe(true);expect(result.redo).toBe(true);
  expect(result.receipt.header.uniformPixels).toBeUndefined();
  expect(result.receipt.header.palettePixels.length/4).toBeGreaterThan(256);
  expect(result.receipt.header.palettePixels.length/4).toBeLessThanOrEqual(1024);
  expect(result.receipt.bytes).toBe(1536*1024*2);
});

test("malformed, cancelled and stale palette transfers preserve the document and its undo history",async({page})=>{
  await page.goto("/");await expect(page.getByTestId("engine-ready")).toBeVisible();
  const results=await page.evaluate(async()=>{
    const a=(window as any).__compositor,e=a.engine,rows=[];
    for(const kind of ["invalid-index","short-plane","ambiguous","cancelled","stale"]){
      const doc=e.newDocument(2048,513,true),id=e.state(doc).layers[0].id;
      e.execute(doc,{type:"Fill",id,mask:false,color:[.2,.4,.6]});
      const input=e.jobHeader(doc,id),header=JSON.parse(input),palette=[1,2,3,255,4,5,6,255],indices=new Uint8Array(2048*513);
      const output:any={transform:header.layer.transform,maskPlacement:null,pixels:[2048,513],mask:null,regions:[],display:null,palettePixels:palette};
      if(kind==="invalid-index")indices[indices.length-1]=2;
      if(kind==="ambiguous")output.uniformPixels=[1,2,3,255];
      if(kind==="stale")e.execute(doc,{type:"Fill",id,mask:false,color:[.7,.6,.5]});
      const before=e.layerPixels(doc,id).slice(),depth=e.state(doc).undoDepth;let checks=0,error="";
      try{await e.installJobAsync(doc,id,input,JSON.stringify(output),kind==="short-plane"?indices.buffer.slice(0,-1):indices.buffer,null,null,false,()=>kind!=="cancelled"||++checks<3);}catch(err){error=String(err);}
      const p=e.layerPixels(doc,id);
      rows.push({kind,rejected:!!error,unchanged:p.length===before.length&&p.every((v:number,i:number)=>v===before[i]),depthSame:e.state(doc).undoDepth===depth});
      e.closeDocument(doc);
    }
    return rows;
  });
  expect(results.map(r=>r.kind)).toEqual(["invalid-index","short-plane","ambiguous","cancelled","stale"]);
  for(const row of results){expect(row.rejected,row.kind).toBe(true);expect(row.unchanged,row.kind).toBe(true);expect(row.depthSame,row.kind).toBe(true);}
});
