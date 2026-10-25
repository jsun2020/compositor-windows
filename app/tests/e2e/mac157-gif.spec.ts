import {test,expect,type Page} from "@playwright/test";
import {readFileSync} from "node:fs";
import {clickMenu} from "./helpers";
const fixture=readFileSync("engine/tests/fixtures/mac157/animation.gif").toString("base64");
async function setup(page:Page){
 await page.goto("/");await expect(page.getByTestId("engine-ready")).toBeVisible();
 await page.evaluate(data=>{const api=(window as any).__compositor,bytes=Uint8Array.from(atob(data),c=>c.charCodeAt(0)),id=api.engine.importImage(null,bytes,"Animation",null);api.engine.markSaved(id,null);api.store.getState().openDocument(id);},fixture);
 await expect(page.getByRole("region",{name:"Animation timeline"})).toBeVisible();
}
test("GIF timeline edits preserve source frames, undo and project v17 roundtrip",async({page})=>{
 await setup(page);await page.getByLabel("Next frame",{exact:true}).click();
 await expect.poll(()=>page.evaluate(()=>{const api=(window as any).__compositor,p=api.readDocumentPixels();let green=0,red=0;for(let i=0;i<p.length;i+=4){if(p[i+1]>p[i]+50)green++;if(p[i]>p[i+1]+50)red++;}return green>0&&red===0;})).toBe(true);
 expect(await page.evaluate(()=>{const s=(window as any).__compositor.store.getState();return s.documents[s.activeId].isModified;})).toBe(false);
 await page.getByLabel("Frame delay",{exact:true}).fill("450");await page.getByLabel("Frame delay",{exact:true}).press("Enter");
 await page.getByRole("button",{name:"Duplicate frame",exact:true}).click();
 await expect(page.getByRole("listitem")).toHaveCount(4);await expect(page.getByLabel("Animation frame",{exact:true})).toHaveValue("3");
 await clickMenu(page,"Image","image-invert");
 await expect.poll(()=>page.evaluate(()=>{const s=(window as any).__compositor.store.getState();return s.documents[s.activeId].layers[0].gif.frames.filter((f:any)=>f.edited).length;})).toBe(1);
 const report=await page.evaluate(()=>{const api=(window as any).__compositor,s=api.store.getState(),d=s.documents[s.activeId],saved=api.engine.savePackage(d.id),other=api.engine.openPackage(saved,null),original=d.layers[0].gif,loaded=api.engine.state(other).layers[0].gif;return{version:JSON.parse(saved.manifest).version,same:JSON.stringify(original)===JSON.stringify(loaded),durations:loaded.frames.map((f:any)=>f.duration),source:saved.images.find((i:any)=>i.name.endsWith('.gif')).bytes.length};});
 expect(report).toEqual({version:17,same:true,durations:[.1,.45,.45,.3],source:Buffer.from(fixture,"base64").length});
 await page.evaluate(()=>(window as any).__compositor.store.getState().undo());
 expect(await page.evaluate(()=>{const s=(window as any).__compositor.store.getState();return s.documents[s.activeId].layers[0].gif.frames.some((f:any)=>f.edited);})).toBe(false);
 await page.getByRole("button",{name:"Delete frame",exact:true}).click();await expect(page.getByRole("listitem")).toHaveCount(3);
});
test("GIF playback follows elapsed time without recording history and pauses before edits",async({page})=>{
 await setup(page);await page.getByRole("button",{name:"Play animation",exact:true}).click();
 await expect.poll(()=>page.evaluate(()=>{const s=(window as any).__compositor.store.getState();return s.documents[s.activeId].layers[0].gif.selectedFrameIndex;})).not.toBe(0);
 await page.getByRole("button",{name:"Pause animation",exact:true}).click();
 expect(await page.evaluate(()=>{const s=(window as any).__compositor.store.getState(),d=s.documents[s.activeId];return [d.undoDepth,d.isModified];})).toEqual([0,false]);
 await page.getByRole("button",{name:"Play animation",exact:true}).click();await page.getByRole("button",{name:"Brush",exact:true}).click();
 await expect(page.getByRole("button",{name:"Play animation",exact:true})).toBeVisible();
});
test("GIF export preview animates and writes a GIF through the file bridge",async({page})=>{
 await setup(page);await clickMenu(page,"File","export-as");await page.getByLabel("Format",{exact:true}).selectOption("GIF");
 await expect.poll(()=>page.getByTestId("export-preview").evaluate((img:HTMLImageElement)=>img.complete&&img.naturalWidth===64)).toBe(true);
 await page.evaluate(()=>(window as any).__compositor.bridge.setNextPick("animation.gif"));
 await page.getByRole("button",{name:"Export",exact:true}).click();
 await expect.poll(()=>page.evaluate(()=>{const bytes=(window as any).__compositor.bridge.fileBytes("animation.gif");return bytes?String.fromCharCode(...bytes.slice(0,6)):null;})).toBe("GIF89a");
 expect(await page.evaluate(()=>{const api=(window as any).__compositor,id=api.engine.importImage(null,api.bridge.fileBytes("animation.gif"),"Exported",null);return api.engine.state(id).layers[0].gif.frames.map((f:any)=>f.duration);})).toEqual([.1,.2,.3]);
});
