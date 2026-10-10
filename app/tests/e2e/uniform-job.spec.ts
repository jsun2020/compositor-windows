import {test,expect} from "@playwright/test";

test("translucent uniform staging preserves RGBA, its partial final chunk and exact Undo/Redo",async({page})=>{
  await page.goto("/");await expect(page.getByTestId("engine-ready")).toBeVisible();
  const result=await page.evaluate(async()=>{
    const e=(window as any).__compositor.engine,w=2051,h=517,color=[17,31,43,97];
    const doc=e.newDocument(w,h,true),id=e.state(doc).layers[0].id;
    e.execute(doc,{type:"Fill",id,mask:false,color:[.2,.4,.6]});
    const before=e.layerPixels(doc,id).slice(),depth=e.state(doc).undoDepth;
    const input=e.jobHeader(doc,id),header=JSON.parse(input);
    const output={transform:header.layer.transform,maskPlacement:null,pixels:[w,h],mask:null,regions:[],display:null,uniformPixels:color};
    await e.installJobAsync(doc,id,input,JSON.stringify(output),null,null);
    const exact=()=>{const p=e.layerPixels(doc,id);return p.length===w*h*4&&p.every((v:number,i:number)=>v===color[i%4]);};
    const applied=exact(),oneStep=e.state(doc).undoDepth===depth+1;
    e.undo(doc);const p=e.layerPixels(doc,id),undo=p.length===before.length&&p.every((v:number,i:number)=>v===before[i]);
    e.redo(doc);const redo=exact();e.closeDocument(doc);
    return{applied,oneStep,undo,redo};
  });
  expect(result).toEqual({applied:true,oneStep:true,undo:true,redo:true});
});

test("uniform and selected Levels worker results match every in-place byte with one Undo/Redo", async ({page}) => {
  test.setTimeout(120_000);
  await page.goto("/"); await expect(page.getByTestId("engine-ready")).toBeVisible();
  const rows = await page.evaluate(async () => {
    const api = (window as any).__compositor, e = api.engine, s = () => api.store.getState(), rows = [];
    api.store.setState({jobPixels: 0});
    for (const selected of [false, true]) {
      const make = () => {
        const doc = e.newDocument(2048, 513, true), layer = e.state(doc).layers[0].id;
        e.execute(doc, {type: "Fill", id: layer, mask: false, color: [.2, .4, .6]});
        if (selected) e.execute(doc, {type: "SelectShape", kind: "Rectangle", points: [[0,0],[1024,0],[1024,513],[0,513]], mode: "Replace", antialiased: false});
        return {doc, layer};
      };
      const actual = make(), reference = make(); s().openDocument(actual.doc);
      const before = new Uint8Array(e.layerPixels(actual.doc, actual.layer)).slice(), depth = e.state(actual.doc).undoDepth;
      const client = s().jobs, run = client.run.bind(client); let receipt: any = null;
      client.run = async (...args: any[]) => { const result = await run(...args); if (args[1].kind === "edit") receipt = {header: JSON.parse(result.header), bytes: result.pixels?.byteLength ?? 0}; return result; };
      try {
        s().beginAdjust({kind: "Levels"});
        while (s().adjustEdit?.histogram === null) await new Promise<void>(r => requestAnimationFrame(() => r()));
        const adjustment = JSON.parse(JSON.stringify(s().adjustEdit.adjustment)); adjustment.levels.ranges[0].outputWhite = 190;
        s().updateAdjust({adjustment});
        while (s().previewSettling()) await new Promise<void>(r => requestAnimationFrame(() => r()));
        s().commitAdjust();
        while (s().working) await new Promise<void>(r => requestAnimationFrame(() => r()));
        e.execute(reference.doc, {type: "ApplyAdjustment", id: reference.layer, adjustment});
        const expected = e.layerPixels(reference.doc, reference.layer).slice() as Uint8Array;
        const equals = (bytes: Uint8Array) => { const p = e.layerPixels(actual.doc, actual.layer); return p.length === bytes.length && p.every((v: number, i: number) => v === bytes[i]); };
        const exact = equals(expected), oneStep = e.state(actual.doc).undoDepth === depth + 1;
        s().undo(); const undoExact = equals(before); s().redo();
        rows.push({selected, receipt, exact, oneStep, undoExact, redoExact: equals(expected), error: s().error});
      } finally { client.run = run; s().closeDocument(actual.doc); e.closeDocument(reference.doc); }
    }
    return rows;
  });
  for (const row of rows) {
    expect(row.error).toBeNull(); expect(row.exact).toBe(true); expect(row.oneStep).toBe(true); expect(row.undoExact).toBe(true); expect(row.redoExact).toBe(true);
    if (row.selected) { expect(row.receipt.header.uniformPixels).toBeUndefined(); expect(row.receipt.header.palettePixels).toHaveLength(8); expect(row.receipt.bytes).toBe(2048*513); }
    else { expect(row.receipt.header.uniformPixels).toHaveLength(4); expect(row.receipt.bytes).toBe(0); }
  }
});

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
