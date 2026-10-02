import { test, expect, type Page } from "@playwright/test";
import { solidPngBase64 } from "./helpers";

async function setup(page: Page) {
  await page.setViewportSize({ width: 1280, height: 720 }); await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const png = await page.evaluate(solidPngBase64, { width: 32, height: 24, color: "#ff0000" });
  await page.evaluate(async (png) => {
    const api = (window as any).__compositor, s = api.store.getState();
    const doc = api.engine.newDocument(64,48,false);
    api.engine.importImage(doc,Uint8Array.from(atob(png),(c: string)=>c.charCodeAt(0)),"Red",{x:24,y:20});
    s.openDocument(doc); await api.setZoom(4);
    api.engine.execute(doc,{type:"SelectShape",kind:"Rectangle",points:[[8,8],[24,8],[24,24],[8,24]],mode:"Replace",antialiased:false});
    api.store.getState().refresh(doc);
  }, png);
}
const state = (page: Page) => page.evaluate(() => { const s=(window as any).__compositor.store.getState();return s.documents[s.activeId]; });
async function idle(page: Page) { await expect.poll(()=>page.evaluate(()=>(window as any).__compositor.store.getState().working)).toBe(false); }
const pixel = (page: Page,x: number,y: number) => page.evaluate(([x,y]) => { const api=(window as any).__compositor,s=api.store.getState();return Array.from(api.engine.composite(s.activeId,{x,y,width:1,height:1},1,1)); },[x,y]);
test("Copy is read-only; Paste in another project retains PNG pixels and original document-space origin",async({page})=>{
  await setup(page); const before=await state(page);
  await page.keyboard.press("Control+c"); await idle(page);
  expect((await state(page)).undoDepth).toBe(before.undoDepth);
  await page.evaluate(()=>{const api=(window as any).__compositor;api.store.getState().openDocument(api.engine.newDocument(90,70,false));});
  await page.keyboard.press("Control+v"); await expect.poll(async()=> (await state(page)).layers.length).toBe(1); await idle(page);
  const pasted=await state(page); expect(pasted.layers[0].transform.origin).toEqual([8,8]); expect(pasted.layers[0].pixelsWidth).toBe(16);
  expect(await pixel(page,9,9)).toEqual([255,0,0,255]); expect(await pixel(page,25,9)).toEqual([0,0,0,0]);
  expect(pasted.selection).toBeNull(); await page.keyboard.press("Control+z"); expect((await state(page)).layers).toHaveLength(0);
});
test("a failed clipboard write leaves Cut pixels and history intact",async({page})=>{
  await setup(page); const before=await state(page);
  await page.evaluate(()=>(window as any).__compositor.bridge.failNextWrite("Clipboard occupied"));
  await page.keyboard.press("Control+x"); await idle(page); await expect(page.getByTestId("error-banner")).toContainText("Clipboard occupied");
  expect(await pixel(page,9,9)).toEqual([255,0,0,255]); expect((await state(page)).undoDepth).toBe(before.undoDepth);
});
test("Cut and Layer via Copy act through the selection with one undo each",async({page})=>{
  await setup(page); const before=await state(page);
  await page.keyboard.press("Control+j"); await expect.poll(async()=> (await state(page)).layers.length).toBe(2); await idle(page);
  const via=await state(page); expect(via.layers[1].pixelsWidth).toBe(16); expect(via.selection.bounds.x).toBe(8); expect(via.undoDepth).toBe(before.undoDepth+1);
  await page.keyboard.press("Control+z"); await page.keyboard.press("Control+x");
  await expect.poll(async()=>await pixel(page,9,9)).toEqual([0,0,0,0]); await idle(page);
  expect(await pixel(page,30,9)).toEqual([255,0,0,255]); expect((await state(page)).undoDepth).toBe(before.undoDepth+1);
  await page.keyboard.press("Control+z"); expect(await pixel(page,9,9)).toEqual([255,0,0,255]);
});
test("Free Transform floats only the selected pixels; Escape is exact and Enter is one undo",async({page})=>{
  await setup(page); const before=await state(page);
  await page.keyboard.press("Control+t"); expect((await state(page)).layers).toHaveLength(2);
  await page.keyboard.press("Shift+ArrowRight"); await page.keyboard.press("Escape");
  const cancelled=await state(page); expect(cancelled.layers).toHaveLength(1);expect(cancelled.undoDepth).toBe(before.undoDepth);expect(await pixel(page,9,9)).toEqual([255,0,0,255]);
  await page.keyboard.press("Control+t"); await page.keyboard.press("Shift+ArrowRight"); await page.keyboard.press("Enter");
  const moved=await state(page);expect(moved.layers).toHaveLength(1);expect(moved.selection.bounds.x).toBe(18);expect(moved.undoDepth).toBe(before.undoDepth+1);
  expect(await pixel(page,9,9)).toEqual([0,0,0,0]); expect(await pixel(page,19,9)).toEqual([255,0,0,255]);
  await page.keyboard.press("Control+z");expect(await pixel(page,9,9)).toEqual([255,0,0,255]);
});
test("Copy without a selection preserves an entire folder, masks and metadata across projects",async({page})=>{
  await setup(page);
  await page.evaluate(()=>{const a=(window as any).__compositor,s=a.store.getState(),d=s.documents[s.activeId],id=d.activeLayerId;a.engine.execute(s.activeId,{type:"Deselect"});a.engine.execute(s.activeId,{type:"AddMask",id,revealing:false});a.engine.execute(s.activeId,{type:"SetLayerOpacity",id,opacity:0.4});a.engine.execute(s.activeId,{type:"GroupLayers",ids:[id]});s.refresh(s.activeId);});
  const original=await state(page);await page.keyboard.press("Control+c");await idle(page);
  await page.evaluate(()=>{const a=(window as any).__compositor;a.store.getState().openDocument(a.engine.newDocument(100,100,false));});await page.keyboard.press("Control+v");await expect.poll(async()=>(await state(page)).layers.length).toBe(2);await idle(page);
  const d=await state(page);expect(d.layers[0].isGroup).toBe(true);expect(d.layers[1].parentId).toBe(d.layers[0].id);expect(d.layers[1].hasMask).toBe(true);expect(d.layers[1].opacity).toBe(0.4);expect(d.layers[1].pixelsWidth).toBe(original.layers[1].pixelsWidth);expect(d.layers[1].id).not.toBe(original.layers[1].id);
  await page.keyboard.press("Control+z");expect((await state(page)).layers).toHaveLength(0);
});
