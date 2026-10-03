import { test, expect, type Page } from "@playwright/test";
import { clickMenu } from "./helpers";
async function setup(page:Page, source=false){
  await page.setViewportSize({width:1280,height:720});await page.goto("/");await expect(page.getByTestId("engine-ready")).toBeVisible();
  await page.evaluate(async(source)=>{const api=(window as any).__compositor,s=api.store.getState();const doc=api.engine.newDocument(80,64,true);s.openDocument(doc);await api.setZoom(5);s.setPaletteColor({red:1,green:0,blue:0},false);s.setBrushOptions({diameter:12,hardness:1,opacity:0.5});if(source){const c=document.createElement("canvas");c.width=c.height=32;const x=c.getContext("2d")!;x.fillStyle="rgb(60,120,180)";x.fillRect(0,0,32,32);x.fillStyle="#f00";x.fillRect(14,14,4,4);const b=await new Promise<Blob>(r=>c.toBlob(v=>r(v!)));api.engine.importImage(doc,new Uint8Array(await b.arrayBuffer()),"Source",{x:32,y:32});s.refresh(doc);}},source);
}
const state=(page:Page)=>page.evaluate(()=>{const s=(window as any).__compositor.store.getState();return s.documents[s.activeId];});
const pixel=(page:Page,x:number,y:number)=>page.evaluate(([x,y])=>{const a=(window as any).__compositor,s=a.store.getState();return Array.from(a.engine.composite(s.activeId,{x,y,width:1,height:1},1,1));},[x,y]);
const client=(page:Page,p:[number,number])=>page.evaluate(([x,y])=>{const s=(window as any).__compositor.store.getState(),d=s.documents[s.activeId],v=s.viewports[s.activeId].viewPoint({x,y},d),r=document.querySelector('[data-testid="canvas-view"]')!.getBoundingClientRect();return{x:r.left+v.x,y:r.top+v.y};},p);
async function idle(page:Page){await expect.poll(()=>page.evaluate(()=>(window as any).__compositor.store.getState().working)).toBe(false);await expect(page.getByTestId("error-banner")).toHaveCount(0);}
async function stroke(page:Page,a:[number,number],b?:[number,number]){const p=await client(page,a);await page.mouse.move(p.x,p.y);await page.mouse.down();if(b){const q=await client(page,b);await page.mouse.move(q.x,q.y,{steps:6});}await page.mouse.up();await idle(page);}
test("Brush and Eraser use one undo per stroke; Escape discards a draft; Shift connects clicks",async({page})=>{
  await setup(page);await page.keyboard.press("b");const before=await state(page);await stroke(page,[20,20],[45,20]);expect(await pixel(page,30,20)).toEqual([128,0,0,128]);expect((await state(page)).undoDepth).toBe(before.undoDepth+1);
  const p=await client(page,[50,40]);await page.mouse.move(p.x,p.y);await page.mouse.down();await page.keyboard.press("Escape");await page.mouse.up();expect(await pixel(page,50,40)).toEqual([0,0,0,0]);expect((await state(page)).undoDepth).toBe(before.undoDepth+1);
  await page.keyboard.down("Shift");await stroke(page,[45,40]);await page.keyboard.up("Shift");expect((await pixel(page,45,30))[3]).toBe(128);
  await page.keyboard.press("e");await page.getByLabel("Brush opacity",{exact:true}).fill("100");await page.getByLabel("Brush opacity",{exact:true}).blur();await stroke(page,[30,20]);expect(await pixel(page,30,20)).toEqual([0,0,0,0]);await page.keyboard.press("Control+z");expect(await pixel(page,30,20)).toEqual([128,0,0,128]);
});
test("Clone Stamp can sample visible layers onto a blank layer through Alt-click",async({page})=>{
  await setup(page);await page.keyboard.press("b");await page.getByLabel("Brush opacity",{exact:true}).fill("100");await page.getByLabel("Brush opacity",{exact:true}).blur();await stroke(page,[20,20]);
  await page.evaluate(()=>{const a=(window as any).__compositor,s=a.store.getState();a.engine.execute(s.activeId,{type:"AddBlankLayer"});s.refresh(s.activeId);});
  await page.keyboard.press("s");await page.getByLabel("Sample all layers").check();const source=await client(page,[20,20]);await page.keyboard.down("Alt");await page.mouse.click(source.x,source.y);await page.keyboard.up("Alt");await stroke(page,[50,20]);expect(await pixel(page,50,20)).toEqual([255,0,0,255]);await page.keyboard.press("Control+z");expect(await pixel(page,50,20)).toEqual([0,0,0,0]);
});
for(const mode of ["Content-Aware","Create Texture","Proximity Match"]){test(`Spot Healing ${mode} uses the original kernel in WASM and is reversible`,async({page})=>{
  await setup(page,true);const before=await pixel(page,32,32);await page.keyboard.press("j");await page.getByLabel("Healing mode").selectOption(mode);await page.getByLabel("Brush size",{exact:true}).fill("5");await page.getByLabel("Brush opacity",{exact:true}).fill("100");await page.getByLabel("Brush opacity",{exact:true}).blur();await stroke(page,[32,32]);expect(await pixel(page,32,32)).not.toEqual(before);expect(await pixel(page,20,20)).toEqual([60,120,180,255]);await page.keyboard.press("Control+z");expect(await pixel(page,32,32)).toEqual(before);
});}
test("Content-Aware Fill previews and cancels without history, then applies once beyond the source edge",async({page})=>{
  await setup(page,true);await page.evaluate(()=>{const a=(window as any).__compositor,s=a.store.getState();a.engine.execute(s.activeId,{type:"SelectShape",kind:"Rectangle",points:[[45,24],[51,24],[51,36],[45,36]],mode:"Replace",antialiased:false});s.refresh(s.activeId);});const before=await state(page);
  await clickMenu(page,"Edit","content-fill");await idle(page);await expect(page.getByRole("button",{name:"Apply",exact:true})).toBeEnabled();expect(await pixel(page,50,30)).toEqual([60,120,180,255]);expect((await state(page)).undoDepth).toBe(before.undoDepth);
  await page.getByLabel("Preview fill",{exact:true}).uncheck();expect(await pixel(page,50,30)).toEqual([0,0,0,0]);await page.getByLabel("Preview fill",{exact:true}).check();expect(await pixel(page,50,30)).toEqual([60,120,180,255]);await page.keyboard.press("Escape");expect(await pixel(page,50,30)).toEqual([0,0,0,0]);expect((await state(page)).undoDepth).toBe(before.undoDepth);
  await clickMenu(page,"Edit","content-fill");await idle(page);await page.keyboard.press("Enter");expect((await state(page)).undoDepth).toBe(before.undoDepth+1);expect(await pixel(page,50,30)).toEqual([60,120,180,255]);await page.keyboard.press("Control+z");expect(await pixel(page,50,30)).toEqual([0,0,0,0]);
});
