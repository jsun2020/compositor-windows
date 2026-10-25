import {test,expect,type Page} from "@playwright/test";
import {clickMenu,noisePngBase64} from "./helpers";
async function setup(page:Page){
 await page.goto("/");await expect(page.getByTestId("engine-ready")).toBeVisible();const b64=await page.evaluate(noisePngBase64);
 await page.evaluate(async data=>{const api=(window as any).__compositor,png=Uint8Array.from(atob(data),c=>c.charCodeAt(0)),id=api.engine.newDocument(64,64,false);api.engine.importImage(id,png,"Source",{x:32,y:32});api.store.getState().openDocument(id);await api.setZoom(1);api.setCheckerboard(false);},b64);
}
test("progressive blur canvas handles move, cancel restores pixels and apply is undoable",async({page})=>{
 await setup(page);const original=await page.evaluate(()=>{const api=(window as any).__compositor,s=api.store.getState();return Array.from(api.engine.layerPixels(s.activeId,s.documents[s.activeId].activeLayerId));});
 await clickMenu(page,"Filter","filter-progressive-blur");await page.getByLabel("To",{exact:true}).fill("6");await page.getByLabel("To",{exact:true}).press("Tab");
 const pos=await page.evaluate(()=>{const s=(window as any).__compositor.store.getState(),d=s.documents[s.activeId],v=s.viewports[s.activeId],el=document.querySelector('canvas')!.parentElement!,r=el.getBoundingClientRect();const a=v.viewPoint({x:10,y:15},d),b=v.viewPoint({x:55,y:50},d);return{a:{x:a.x+r.left,y:a.y+r.top},b:{x:b.x+r.left,y:b.y+r.top}};});
 await page.mouse.move(pos.a.x,pos.a.y);await page.mouse.down();await page.mouse.move(pos.b.x,pos.b.y,{steps:3});await page.mouse.up();
 const params=await page.evaluate(()=>(window as any).__compositor.store.getState().adjustEdit.params);
 expect(params.settings.start[0]).toBeCloseTo(10,0);expect(params.settings.end[1]).toBeCloseTo(50,0);
 await page.getByTestId("adjust-cancel").click();
 expect(await page.evaluate(()=>{const api=(window as any).__compositor,s=api.store.getState();return Array.from(api.engine.layerPixels(s.activeId,s.documents[s.activeId].activeLayerId));})).toEqual(original);
 await clickMenu(page,"Filter","filter-progressive-blur");await page.getByTestId("adjust-ok").click();await expect(page.getByTestId("adjust-panel")).toHaveCount(0);
 await page.evaluate(()=>{const s=(window as any).__compositor.store.getState();s.undo();});
 expect(await page.evaluate(()=>{const api=(window as any).__compositor,s=api.store.getState();return Array.from(api.engine.layerPixels(s.activeId,s.documents[s.activeId].activeLayerId));})).toEqual(original);
});
test("all three v16 adjustments preserve sources, reopen, and display the reference compositor",async({page})=>{
 for(const [kind,id] of [["Progressive Blur","progressive-blur"],["Bloom / Glow","bloom-glow"],["RGB Split","rgb-split"]]){
  await setup(page);await clickMenu(page,"Layer",`layer-adjustment-${id}`);await page.getByTestId("adjust-ok").click();
  const result=await page.evaluate(async()=>{const api=(window as any).__compositor,s=api.store.getState(),d=s.documents[s.activeId],saved=api.engine.savePackage(d.id),manifest=JSON.parse(saved.manifest),reopened=api.engine.openPackage(saved,null);const d2=api.engine.state(reopened);await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));const actual=api.readDocumentPixels(),rect=s.viewports[d.id].documentRect(d),dpr=window.devicePixelRatio||1,w=Math.round((rect.x+rect.width)*dpr)-Math.round(rect.x*dpr),h=Math.round((rect.y+rect.height)*dpr)-Math.round(rect.y*dpr),expected=api.engine.compositeEdit(d.id,null,{x:0,y:0,width:d.width,height:d.height},w,h);let max=0;for(let i=0;i<actual.length;i++)max=Math.max(max,Math.abs(actual[i]-expected[i]));return{version:manifest.version,kind:d2.layers[1].adjustment.kind,backend:api.compositionBackend(),lengths:[actual.length,expected.length],max,sourceUnchanged:Array.from(api.engine.layerPixels(d.id,d.layers[0].id)).join()===Array.from(api.engine.layerPixels(reopened,d2.layers[0].id)).join()};});
  expect(result.version).toBe(16);expect(result.kind).toBe(kind);expect(result.sourceUnchanged).toBe(true);expect(result.backend).toBe("cpu-reference");expect(result.lengths[0]).toBe(result.lengths[1]);expect(result.max).toBe(0);
 }
});
test("channel pack preview shows RGB with a zero alpha source",async({page})=>{
 await setup(page);await clickMenu(page,"File","export-as");await page.getByLabel("Format",{exact:true}).selectOption("ChannelPack");await page.getByLabel("Red Source").selectOption({label:"Source"});await page.getByLabel("Alpha Source").selectOption("Black");
 await expect.poll(()=>page.evaluate(()=>{const image=document.querySelector('[data-testid="export-preview"]') as HTMLImageElement|null;if(!image?.complete||!image.naturalWidth)return false;const c=document.createElement('canvas');c.width=image.naturalWidth;c.height=image.naturalHeight;const ctx=c.getContext('2d')!;ctx.drawImage(image,0,0);const p=ctx.getImageData(0,0,c.width,c.height).data;let red=false;for(let i=0;i<p.length;i+=4){if(p[i]>0)red=true;if(p[i+1]!==0||p[i+2]!==0||p[i+3]!==255)return false;}return red;})).toBe(true);
});
