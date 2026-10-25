import {test,expect,type Page} from "@playwright/test";
import {clickMenu} from "./helpers";
async function setup(page:Page){
 await page.goto("/");await expect(page.getByTestId("engine-ready")).toBeVisible();
 await page.evaluate(()=>{const api=(window as any).__compositor,id=api.engine.newCanvas(32,32,72,1);api.engine.markSaved(id,null);api.store.getState().openDocument(id);api.setCheckerboard(false);
  const lut:number[]=[];for(let b=0;b<33;b++)for(let g=0;g<33;g++)for(let r=0;r<33;r++)lut.push(0,Math.round(g*255/32),Math.round(b*255/32));
  api.bridge.listColorProfiles=async()=>[{name:"Test print",path:"test.icc"}];api.bridge.proofColorProfile=async()=>({profile:{name:"Test print",path:"test.icc"},lut});
  api.bridge.exportCmyk=async(png:Uint8Array,profile:string,format:string,quality:number,dpi:number)=>{(window as any).__print={png:[...png.slice(0,8)],profile,format,quality,dpi};return new Uint8Array([1,2,3]);};
 });
}
test("CMYK proof and gamut warnings change only the displayed canvas",async({page})=>{
 await setup(page);await clickMenu(page,"View","color-proof");await expect(page.getByLabel("CMYK profile")).toHaveValue("test.icc");
 await page.getByLabel("Proof Colors").check();await page.getByRole("button",{name:"Done",exact:true}).click();
 await expect.poll(()=>page.evaluate(()=>Array.from((window as any).__compositor.readDocumentPixels().slice(0,4)))).toEqual([0,255,255,255]);
 expect(await page.evaluate(()=>{const api=(window as any).__compositor,s=api.store.getState(),d=s.documents[s.activeId];return {dirty:d.isModified,undo:d.undoDepth,pixel:[...api.engine.layerPixels(d.id,d.layers[0].id).slice(0,4)]};})).toEqual({dirty:false,undo:0,pixel:[255,255,255,255]});
 await clickMenu(page,"View","color-proof");await page.getByLabel("Gamut Warning").check();await page.getByRole("button",{name:"Done",exact:true}).click();
 await expect.poll(()=>page.evaluate(()=>Array.from((window as any).__compositor.readDocumentPixels().slice(0,4)))).toEqual([128,128,128,255]);
 await clickMenu(page,"View","color-proof");await page.getByLabel("Proof Colors").uncheck();await page.getByLabel("Gamut Warning").uncheck();await page.getByRole("button",{name:"Done",exact:true}).click();
 await expect.poll(()=>page.evaluate(()=>Array.from((window as any).__compositor.readDocumentPixels().slice(0,4)))).toEqual([255,255,255,255]);
});
test("CMYK export sends flattened RGB through the chosen native ICC converter",async({page})=>{
 await setup(page);await clickMenu(page,"File","export-as");await page.getByLabel("Format",{exact:true}).selectOption("TIFF");await page.getByLabel("Export Color Space").selectOption("CMYK");
 await expect(page.getByLabel("CMYK profile")).toHaveValue("test.icc");await expect(page.getByLabel("Export Background")).toHaveValue("White");
 await page.evaluate(()=>(window as any).__compositor.bridge.setNextPick("print.tif"));await page.getByRole("button",{name:"Export",exact:true}).click();
 await expect.poll(()=>page.evaluate(()=>(window as any).__compositor.bridge.fileBytes("print.tif")?.length)).toBe(3);
 expect(await page.evaluate(()=>(window as any).__print)).toEqual({png:[137,80,78,71,13,10,26,10],profile:"test.icc",format:"TIFF",quality:.85,dpi:72});
});
test("converted CMYK package imports all layers into one undo step and adopts its profile",async({page})=>{
 await setup(page);
 await page.evaluate(()=>{const api=(window as any).__compositor,id=api.engine.newDocument(10,20,true),files=api.engine.savePackage(id);api.engine.closeDocument(id);
  api.bridge.importCmykFile=async()=>({files,metadata:JSON.stringify({artboards:[],conversions:[]}),profile:{path:"test.icc",name:"Test print"}});api.bridge.setNextPick("press.psd");
 });
 await clickMenu(page,"File","import");await expect(page.getByTestId("layer-row")).toHaveCount(2);
 expect(await page.evaluate(()=>{const s=(window as any).__compositor.store.getState(),d=s.documents[s.activeId];return[d.width,d.height,d.undoDepth,d.layers[1].transform.origin];})).toEqual([32,32,1,[11,6]]);
 await page.evaluate(()=>(window as any).__compositor.store.getState().undo());await expect(page.getByTestId("layer-row")).toHaveCount(1);
});
