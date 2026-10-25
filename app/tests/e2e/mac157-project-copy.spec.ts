import {test,expect,type Page} from "@playwright/test";
import {noisePngBase64} from "./helpers";
async function setup(page:Page){
  await page.goto("/");await expect(page.getByTestId("engine-ready")).toBeVisible();const png=await page.evaluate(noisePngBase64);
  return page.evaluate(data=>{const api=(window as any).__compositor,id=api.engine.newDocument(500,400,false);
    api.engine.importImage(id,Uint8Array.from(atob(data),c=>c.charCodeAt(0)),"Dragged layer",{x:200,y:150});
    let d=api.engine.state(id),layer=d.layers[0];api.engine.execute(id,{type:"SetLayerTransform",id:layer.id,transform:{...layer.transform,rotation:30}});
    api.store.getState().openDocument(id);return {source:id,layer:layer.id};},png);
}
test("dragging a rotated layer to new canvas fits its bounds and leaves the source intact",async({page})=>{
  const {source,layer}=await setup(page);
  await page.getByTestId("layer-row").dragTo(page.getByRole("button",{name:"New canvas",exact:true}));
  await expect(page.getByTestId("project-tab")).toHaveCount(2);
  const result=await page.evaluate(({source,layer})=>{const api=(window as any).__compositor,s=api.store.getState(),d=s.documents[s.activeId];return {size:[d.width,d.height],angle:d.layers[0].transform.rotation,
    selected:s.selectedLayerIds,layer:d.layers[0].id,sourceExists:api.engine.state(source).layers[0].id===layer,
    same:Array.from(api.engine.layerPixels(d.id,d.layers[0].id)).join()===Array.from(api.engine.layerPixels(source,layer)).join(),depth:d.undoDepth};},{source,layer});
  expect(result.size).toEqual([88,88]);expect(result.angle).toBe(30);expect(result.sourceExists&&result.same).toBe(true);expect(result.selected).toEqual([result.layer]);expect(result.depth).toBe(1);
  await page.evaluate(()=>(window as any).__compositor.store.getState().undo());await expect(page.getByTestId("layer-row")).toHaveCount(0);
});
test("drop onto another tab copies into that project; dropping onto its own tab makes no new tab",async({page})=>{
  const {source}=await setup(page);const target=await page.evaluate(()=>{const api=(window as any).__compositor,id=api.engine.newDocument(200,100,false);api.store.getState().openDocument(id);return id;});
  await page.locator(`[data-doc-id="${source}"]`).click();
  await page.getByTestId("layer-row").dragTo(page.locator(`[data-doc-id="${source}"]`));await expect(page.getByTestId("project-tab")).toHaveCount(2);
  await page.getByTestId("layer-row").dragTo(page.locator(`[data-doc-id="${target}"]`));
  await expect.poll(()=>page.evaluate(()=>(window as any).__compositor.store.getState().activeId)).toBe(target);
  expect(await page.evaluate(()=>{const s=(window as any).__compositor.store.getState(),d=s.documents[s.activeId];return[d.width,d.height,d.layers.length,d.layers[0].transform.origin];})).toEqual([200,100,1,[68,18]]);
});
