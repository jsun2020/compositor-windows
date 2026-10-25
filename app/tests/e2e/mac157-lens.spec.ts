import {test,expect,type Page} from "@playwright/test";
import {readFileSync} from "node:fs";
import {clickMenu} from "./helpers";
const input=Array.from(readFileSync("engine/tests/fixtures/mac157/lens-input.rgba"));
async function setup(page:Page){
  await page.goto("/");await expect(page.getByTestId("engine-ready")).toBeVisible();
  await page.evaluate(data=>{
    const api=(window as any).__compositor,c=document.createElement('canvas');c.width=96;c.height=64;
    // Convert the reference's premultiplied bytes for Canvas ImageData's straight RGBA.
    const bytes=new Uint8ClampedArray(data);for(let i=0;i<bytes.length;i+=4)if(bytes[i+3])for(let k=0;k<3;k++)bytes[i+k]=bytes[i+k]*255/bytes[i+3];
    c.getContext('2d')!.putImageData(new ImageData(bytes,96,64),0,0);
    const png=Uint8Array.from(atob(c.toDataURL().split(',')[1]),v=>v.charCodeAt(0));
    const id=api.engine.importImage(null,png,"Lights",null);api.store.getState().openDocument(id);
  },input);
}
async function pixels(page:Page){return page.evaluate(()=>{const api=(window as any).__compositor,s=api.store.getState(),d=s.documents[s.activeId];return Array.from(api.engine.layerPixels(d.id,d.activeLayerId));});}
test("five lens effects expose their own controls; preview cancels and applied bokeh undoes",async({page})=>{
  await setup(page);const before=await pixels(page);
  await clickMenu(page,"Filter","filter-lens-effects");
  await expect(page.getByTestId("adjust-panel").getByLabel("Blue",{exact:true})).toHaveValue("80");
  await page.getByTestId("adjust-panel").getByLabel("Effect",{exact:true}).selectOption("Starburst");
  await page.getByTestId("adjust-panel").getByLabel("Points",{exact:true}).selectOption("14");
  await expect(page.getByTestId("adjust-panel").getByLabel("Angle",{exact:true})).toHaveValue("15");
  await page.getByTestId("adjust-panel").getByLabel("Effect",{exact:true}).selectOption("Mist");
  await expect(page.getByTestId("adjust-panel").getByLabel("Strength",{exact:true})).toHaveValue("50");
  await page.getByTestId("adjust-panel").getByLabel("Effect",{exact:true}).selectOption("Prism");
  await expect(page.getByTestId("adjust-panel").getByLabel("Copies",{exact:true})).toHaveValue("3");
  await page.getByTestId("adjust-panel").getByLabel("Effect",{exact:true}).selectOption("Bokeh");
  await page.getByTestId("adjust-panel").getByLabel("Shape",{exact:true}).selectOption("Cat's Eye");
  await page.getByTestId("adjust-cancel").click();expect(await pixels(page)).toEqual(before);
  await clickMenu(page,"Filter","filter-lens-effects");
  await page.getByTestId("adjust-panel").getByLabel("Effect",{exact:true}).selectOption("Bokeh");
  await page.getByTestId("adjust-panel").getByLabel("Shape",{exact:true}).selectOption("Hexagon");
  await page.getByTestId("adjust-ok").click();await expect(page.getByTestId("adjust-panel")).toHaveCount(0);
  await expect.poll(()=>pixels(page)).not.toEqual(before);
  await page.evaluate(()=>(window as any).__compositor.store.getState().undo());expect(await pixels(page)).toEqual(before);
});
test("zero-strength lens filter closes without a new history entry",async({page})=>{
  await setup(page);const before=await page.evaluate(()=>{const s=(window as any).__compositor.store.getState();return s.documents[s.activeId].undoDepth;});
  await clickMenu(page,"Filter","filter-lens-effects");await page.getByTestId("adjust-panel").getByLabel("Amount",{exact:true}).fill("0");await page.getByTestId("adjust-panel").getByLabel("Amount",{exact:true}).press("Tab");
  await page.getByTestId("adjust-ok").click();await expect(page.getByTestId("adjust-panel")).toHaveCount(0);
  expect(await page.evaluate(()=>{const s=(window as any).__compositor.store.getState();return s.documents[s.activeId].undoDepth;})).toBe(before);
});
