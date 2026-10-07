import {test,expect} from "@playwright/test";

test("a worker's uniform fill restores every pixel and one Undo/Redo",async({page})=>{
  test.setTimeout(120_000);
  await page.goto("/");await expect(page.getByTestId("engine-ready")).toBeVisible();
  const result=await page.evaluate(async()=>{
    const api=(window as any).__compositor,e=api.engine,s=()=>api.store.getState();
    const doc=e.newDocument(2048,2049,true);s().openDocument(doc);
    const layer=e.state(doc).layers[0].id,client=s().jobs,run=client.run.bind(client);
    let receipt:any=null;
    client.run=async(...args:any[])=>{const r=await run(...args);receipt={header:JSON.parse(r.header),bytes:r.pixels?.byteLength??0};return r;};
    s().setPaletteColor({red:.2,green:.4,blue:.6},false);
    window.dispatchEvent(new KeyboardEvent("keydown",{key:"Backspace",altKey:true}));
    const wentToWorker=s().working;
    while(s().working)await new Promise<void>(r=>requestAnimationFrame(()=>r()));
    const expected=[51,102,153,255];
    const exact=()=>{const p=e.layerPixels(doc,layer,0);return p?.length===2048*2049*4&&p.every((v:number,i:number)=>v===expected[i%4]);};
    const beforeUndo={exact:exact(),depth:e.state(doc).undoDepth,receipt};
    s().undo();const blank=e.storedPixels(doc,layer)===0;s().redo();
    return {wentToWorker,...beforeUndo,blank,redoExact:exact(),depthAfterRedo:e.state(doc).undoDepth,error:s().error};
  });
  expect(result.error).toBeNull();expect(result.wentToWorker).toBe(true);
  expect(result.receipt.header.uniformPixels).toEqual([51,102,153,255]);expect(result.receipt.bytes).toBe(0);
  expect(result.exact).toBe(true);expect(result.blank).toBe(true);expect(result.redoExact).toBe(true);
  expect(result.depth).toBe(1);expect(result.depthAfterRedo).toBe(1);
});

test("a partial fill of a stored layer keeps the ordinary payload and every unselected pixel",async({page})=>{
  test.setTimeout(120_000);
  await page.goto("/");await expect(page.getByTestId("engine-ready")).toBeVisible();
  const result=await page.evaluate(async()=>{
    const api=(window as any).__compositor,e=api.engine,s=()=>api.store.getState();
    const w=2048,h=2049,doc=e.newDocument(w,h,true);s().openDocument(doc);const layer=e.state(doc).layers[0].id;
    s().setPaletteColor({red:.2,green:.4,blue:.6},false);
    window.dispatchEvent(new KeyboardEvent("keydown",{key:"Backspace",altKey:true}));
    while(s().working)await new Promise<void>(r=>requestAnimationFrame(()=>r()));
    e.execute(doc,{type:"SelectShape",kind:"Rectangle",points:[[0,0],[1024,0],[1024,h],[0,h]],mode:"Replace",antialiased:false});s().refresh(doc);
    const client=s().jobs,run=client.run.bind(client);let receipt:any=null;
    client.run=async(...args:any[])=>{const r=await run(...args);receipt={header:JSON.parse(r.header),bytes:r.pixels?.byteLength??0};return r;};
    s().setPaletteColor({red:.8,green:.6,blue:.4},false);
    window.dispatchEvent(new KeyboardEvent("keydown",{key:"Backspace",altKey:true}));
    const wentToWorker=s().working;
    while(s().working)await new Promise<void>(r=>requestAnimationFrame(()=>r()));
    const red=[204,153,102,255],blue=[51,102,153,255];
    const exact=()=>{const p=e.layerPixels(doc,layer,0);return p?.length===w*h*4&&p.every((v:number,i:number)=>v===((Math.floor(i/4)%w)<1024?red:blue)[i%4]);};
    const first=exact();s().undo();const restored=e.layerPixels(doc,layer,0).every((v:number,i:number)=>v===blue[i%4]);s().redo();
    return {wentToWorker,receipt,first,restored,redoExact:exact(),error:s().error};
  });
  expect(result.error).toBeNull();expect(result.wentToWorker).toBe(true);expect(result.receipt.header.uniformPixels).toBeUndefined();
  expect(result.receipt.bytes).toBe(2048*2049*4);expect(result.first).toBe(true);expect(result.restored).toBe(true);expect(result.redoExact).toBe(true);
});
