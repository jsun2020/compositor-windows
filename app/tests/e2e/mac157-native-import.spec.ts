import {test,expect} from "@playwright/test";
import {readFileSync} from "node:fs";
import {clickMenu,noisePngBase64} from "./helpers";
const boards=readFileSync("engine/tests/fixtures/mac157/artboards.psd").toString("base64");
test("PSD artboards open as named independent tabs without changing an existing canvas",async({page})=>{
 await page.goto("/");await expect(page.getByTestId("engine-ready")).toBeVisible();
 const original=await page.evaluate(data=>{const api=(window as any).__compositor,id=api.engine.newDocument(20,30,true);api.store.getState().openDocument(id);api.bridge.seedFile("boards.psd",Uint8Array.from(atob(data),c=>c.charCodeAt(0)));api.bridge.setNextPick("boards.psd");return id;},boards);
 await clickMenu(page,"File","import");await expect(page.getByTestId("project-tab")).toHaveCount(3);
 const result=await page.evaluate(original=>{const s=(window as any).__compositor.store.getState();return{original:s.documents[original].layers.length,boards:s.order.filter((id:string)=>id!==original).map((id:string)=>({name:s.untitledNames[id],size:[s.documents[id].width,s.documents[id].height],layers:s.documents[id].layers.map((l:any)=>l.name)}))};},original);
 expect(result).toEqual({original:1,boards:[{name:"Board A",size:[2,2],layers:["First content"]},{name:"Board B",size:[5,7],layers:["Second content","Nested"]}]});
});
test("native PDF response imports all pages in reverse stacking order at print resolution",async({page})=>{
 await page.goto("/");await expect(page.getByTestId("engine-ready")).toBeVisible();const data=await page.evaluate(noisePngBase64);
 await page.evaluate(data=>{const api=(window as any).__compositor,png=Uint8Array.from(atob(data),c=>c.charCodeAt(0));
  api.bridge.decodeNativeImage=async()=>({resolution:300,images:[{name:"Page 1",bytes:png},{name:"Page 2",bytes:png}]});api.bridge.setNextPick("pages.pdf");
 },data);
 await clickMenu(page,"File","import");await expect(page.getByTestId("layer-row")).toHaveCount(2);
 expect(await page.evaluate(()=>{const s=(window as any).__compositor.store.getState(),d=s.documents[s.activeId];return{names:d.layers.map((l:any)=>l.name),resolution:d.resolution,modified:d.isModified,active:d.activeLayerId===d.layers[1].id};})).toEqual({names:["Page 2","Page 1"],resolution:300,modified:true,active:true});
});
