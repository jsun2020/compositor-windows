import { test, expect } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

const matrix = process.env.CG_SAMPLING_MATRIX ?? path.resolve("engine/tests/fixtures/cg-ci-sampling-mac-1.4.5");
const inputs: { name: string; folder: string }[] = JSON.parse(fs.readFileSync(path.join(matrix, "cases.json"), "utf8"))
  .filter((row: { operation: string }) => row.operation === "layer")
  .map((row: { name: string; package: string }) => ({ name: row.name, folder: path.resolve(matrix, row.package) }));

for (const { name, folder } of inputs) {
  test("CG sampler GPU and WASM agree for " + name, async ({ page }) => {
    const input = { manifest: fs.readFileSync(path.join(folder, "manifest.json"), "utf8"),
      images: fs.readdirSync(path.join(folder, "images")).map(file => ({ name: file, bytes: Array.from(fs.readFileSync(path.join(folder, "images", file))) })) };
    await page.goto("/"); await expect(page.getByTestId("engine-ready")).toBeVisible();
    const result = await page.evaluate(async input => {
      const api = (window as any).__compositor;
      const doc = api.engine.openPackage({ manifest: input.manifest, images: input.images.map(i => ({ name: i.name, bytes: new Uint8Array(i.bytes) })) }, null);
      api.store.getState().openDocument(doc); await api.setZoom(1); api.setCheckerboard(false);
      await new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)));
      const state = api.store.getState(), d = state.documents[doc], vp = state.viewports[doc], dpr = window.devicePixelRatio || 1;
      const rect = vp.documentRect({ width: d.width, height: d.height });
      vp.translate({ width: (Math.round(rect.x*dpr)-rect.x*dpr)/dpr, height: (Math.round(rect.y*dpr)-rect.y*dpr)/dpr });
      api.renderer.render(api.engine, d, vp, dpr, { checkerboard: false }, null);
      const gpu = Array.from(api.readDocumentPixels()) as number[];
      const cpu = Array.from(api.engine.composite(doc, { x: 0, y: 0, width: d.width, height: d.height }, d.width, d.height)) as number[];
      const mismatches = gpu.flatMap((value, i) => Math.abs(value-cpu[i])>2 ? [{ x: Math.floor(i/4)%d.width, y: Math.floor(i/4/d.width), channel: i%4, gpu: value, cpu: cpu[i] }] : []);
      const record = { kind: state.rendererKind, length: gpu.length, expectedLength: cpu.length, worst: gpu.reduce((m, value, i) => Math.max(m, Math.abs(value-cpu[i])), 0), mismatches: mismatches.slice(0, 12) };
      api.store.getState().closeDocument(doc); return record;
    }, input);
    expect(result.kind).toBe("gl"); expect(result.length).toBe(result.expectedLength);
    expect(result.worst, JSON.stringify(result.mismatches)).toBeLessThanOrEqual(2);
  });
}

// Original order.rs clipping-source regression, exercised through the GPU too.
test("a clipped source keeps both final 2:1 taps after sharp halving", async ({ page }) => {
  await page.goto("/");await expect(page.getByTestId("engine-ready")).toBeVisible();
  const result=await page.evaluate(async()=>{
    const api=(window as any).__compositor,engine=api.engine;
    const png=async(size:number,sparse:boolean)=>{const canvas=document.createElement("canvas");canvas.width=size;canvas.height=size;const ctx=canvas.getContext("2d")!;const image=ctx.createImageData(size,size);for(let y=0;y<size;y++)for(let x=0;x<size;x++){const i=(y*size+x)*4,v=!sparse||(x%4===0&&y%4===0)?255:0;image.data.set(sparse?[v,v,v,v]:[0,0,255,255],i);}ctx.putImageData(image,0,0);const blob=await new Promise<Blob>(r=>canvas.toBlob(b=>r(b!)));return new Uint8Array(await blob.arrayBuffer());};
    const doc=engine.newDocument(64,64,false);engine.importImage(doc,await png(64,true),"source",{x:32,y:32});const base=engine.state(doc).layers[0].id;
    engine.importImage(doc,await png(8,false),"clipped",{x:32,y:32});const top=engine.state(doc).layers[1];engine.execute(doc,{type:"SetLayerTransform",id:top.id,transform:{...top.transform,origin:[0,0],size:[64,64]}});
    engine.execute(doc,{type:"LinkMask",source:base,target:top.id});engine.execute(doc,{type:"SetLayerVisible",id:base,visible:false});
    api.store.getState().openDocument(doc);await api.setZoom(0.25);api.setCheckerboard(false);await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));
    const state=api.store.getState(),d=state.documents[doc],vp=state.viewports[doc],dpr=window.devicePixelRatio||1;const rect=vp.documentRect(d);vp.translate({width:(Math.round(rect.x*dpr)-rect.x*dpr)/dpr,height:(Math.round(rect.y*dpr)-rect.y*dpr)/dpr});
    api.renderer.render(engine,d,vp,dpr,{checkerboard:false},null);const gpu=Array.from(api.readDocumentPixels()) as number[],cpu=Array.from(engine.composite(doc,{x:0,y:0,width:64,height:64},16,16)) as number[];
    const outcome={kind:state.rendererKind,length:gpu.length,expectedLength:cpu.length,worst:gpu.reduce((m,v,i)=>Math.max(m,Math.abs(v-cpu[i])),0),alphas:Array.from({length:16},(_,x)=>cpu[(8*16+x)*4+3])};api.store.getState().closeDocument(doc);return outcome;
  });
  expect(result.kind).toBe("gl");expect(result.length).toBe(result.expectedLength);expect(result.worst).toBeLessThanOrEqual(2);expect(result.alphas.every(a=>a>=8&&a<=32)).toBe(true);
});
