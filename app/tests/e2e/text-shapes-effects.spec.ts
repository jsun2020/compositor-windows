import {test,expect,type Page} from "@playwright/test";
import {clickMenu} from "./helpers";
async function setup(page:Page){await page.setViewportSize({width:1440,height:900});await page.goto("/");await expect(page.getByTestId("engine-ready")).toBeVisible();await page.evaluate(async()=>{const a=(window as any).__compositor,s=a.store.getState();s.openDocument(a.engine.newDocument(640,480,true));await a.setZoom(1);});}
const state=(page:Page)=>page.evaluate(()=>{const s=(window as any).__compositor.store.getState();return s.documents[s.activeId];});
const point=(page:Page,p:[number,number])=>page.evaluate(([x,y])=>{const s=(window as any).__compositor.store.getState(),d=s.documents[s.activeId],v=s.viewports[s.activeId].viewPoint({x,y},d),r=document.querySelector('[data-testid="canvas-view"]')!.getBoundingClientRect();return{x:r.left+v.x,y:r.top+v.y};},p);
async function textReady(page:Page){await expect(page.getByTestId("text-panel").getByRole("button",{name:"Apply",exact:true})).toBeEnabled();await expect(page.getByTestId("error-banner")).toHaveCount(0);}
async function createText(page:Page,content="Hello"){await page.keyboard.press("t");const p=await point(page,[30,30]);await page.mouse.click(p.x,p.y);await page.getByLabel("Text content",{exact:true}).fill(content);await page.getByLabel("Font size",{exact:true}).fill("32");await textReady(page);await page.getByTestId("text-panel").getByRole("button",{name:"Apply",exact:true}).click();}
test("Type creates editable text; cancel and preview do not record; edits round-trip with UTF-16 runs",async({page})=>{
  await setup(page);const before=await state(page);await createText(page,"A\u{1f600}\u4e2d\u6587B");let d=await state(page);expect(d.undoDepth).toBe(before.undoDepth+1);const id=d.activeLayerId;expect(d.layers.find((l:any)=>l.id===id).text.content).toBe("A\u{1f600}\u4e2d\u6587B");
  await clickMenu(page,"Layer","layer-edit-text");const box=page.getByLabel("Text content",{exact:true});await box.focus();await page.keyboard.press("Home");await page.keyboard.press("ArrowRight");await page.keyboard.press("Shift+ArrowRight");
  await page.getByLabel("Text color",{exact:true}).fill("#ff0000");await textReady(page);expect((await state(page)).undoDepth).toBe(d.undoDepth);await page.getByLabel("Preview text",{exact:true}).uncheck();await page.getByLabel("Preview text",{exact:true}).check();await page.getByRole("button",{name:"Apply",exact:true}).click();d=await state(page);expect(d.layers.find((l:any)=>l.id===id).text.colorRuns).toEqual([{location:1,length:2,red:1,green:0,blue:0}]);
  const reloaded=await page.evaluate(()=>{const a=(window as any).__compositor,s=a.store.getState(),files=a.engine.savePackage(s.activeId);const id=a.engine.openPackage(files,null);s.openDocument(id);return a.engine.state(id);});expect(reloaded.layers.find((l:any)=>l.text)?.text.colorRuns).toEqual([{location:1,length:2,red:1,green:0,blue:0}]);
  await clickMenu(page,"Layer","layer-edit-text");await box.fill("Replacement");await textReady(page);await page.keyboard.press("Escape");expect((await state(page)).layers.find((l:any)=>l.text)?.text.content).toBe("A\u{1f600}\u4e2d\u6587B");expect((await state(page)).undoDepth).toBe(0);
});
test("Type drag makes a wrapping fixed box; multiline settings apply once and undo restores",async({page})=>{
  await setup(page);await page.keyboard.press("t");const a=await point(page,[20,20]),b=await point(page,[220,150]);await page.mouse.move(a.x,a.y);await page.mouse.down();await page.mouse.move(b.x,b.y);await page.mouse.up();await expect(page.getByLabel("Fixed text box")).toBeChecked();
  await page.getByLabel("Text content",{exact:true}).fill("First line\nSecond line wraps across the text box");await page.getByLabel("Font size",{exact:true}).fill("24");await page.getByLabel("Text alignment").selectOption("Center");await page.getByLabel("Tracking",{exact:true}).fill("2");await textReady(page);await page.keyboard.press("Control+Enter");let d=await state(page);const l=d.layers.find((l:any)=>l.text);expect(l.text.boxSize).toEqual([200,130]);expect([l.pixelsWidth,l.pixelsHeight]).toEqual([200,130]);expect(l.text.alignment).toBe("Center");await page.keyboard.press("Control+z");expect((await state(page)).layers).toHaveLength(1);
});
test("Cancel a new text layer leaves no pixels, layer or history",async({page})=>{
  await setup(page);const before=await state(page);await page.keyboard.press("t");const p=await point(page,[20,20]);await page.mouse.click(p.x,p.y);await page.getByLabel("Text content",{exact:true}).fill("Cancelled");await textReady(page);
  await page.getByLabel("Text content",{exact:true}).evaluate(el=>el.dispatchEvent(new KeyboardEvent("keydown",{key:"Escape",isComposing:true,bubbles:true})));
  await expect(page.getByTestId("text-panel")).toBeVisible();
  await page.keyboard.press("Escape");expect(await state(page)).toEqual(before);
});
test("Stroke and Drop Shadow preview, cancellation and commit preserve editable text and source pixels",async({page})=>{
  await setup(page);await createText(page);const before=await state(page),id=before.activeLayerId;
  const source=await page.evaluate(()=>{const a=(window as any).__compositor,s=a.store.getState();return Array.from(a.engine.layerPixels(s.activeId,s.documents[s.activeId].activeLayerId));});
  await clickMenu(page,"Layer","layer-effects");await page.getByLabel("Enable Stroke",{exact:true}).check();await page.getByLabel("Stroke Size",{exact:true}).fill("6");await page.getByLabel("Enable Drop Shadow",{exact:true}).check();await page.getByLabel("Drop Shadow Distance",{exact:true}).fill("8");await page.getByLabel("Preview effects",{exact:true}).uncheck();await page.getByLabel("Preview effects",{exact:true}).check();expect((await state(page)).undoDepth).toBe(before.undoDepth);await page.keyboard.press("Escape");expect((await state(page)).layers.find((l:any)=>l.id===id).effects).toBeUndefined();
  await clickMenu(page,"Layer","layer-effects");await page.getByLabel("Enable Stroke",{exact:true}).check();await page.getByLabel("Enable Drop Shadow",{exact:true}).check();await page.getByRole("button",{name:"Apply",exact:true}).click();const after=await state(page);expect(after.undoDepth).toBe(before.undoDepth+1);expect(after.layers.find((l:any)=>l.id===id).effects.stroke.enabled).toBe(true);expect(after.layers.find((l:any)=>l.id===id).text.content).toBe("Hello");expect(await page.evaluate(()=>{const a=(window as any).__compositor,s=a.store.getState();return Array.from(a.engine.layerPixels(s.activeId,s.documents[s.activeId].activeLayerId));})).toEqual(source);
  await page.keyboard.press("Control+z");expect((await state(page)).layers.find((l:any)=>l.id===id).effects).toBeUndefined();
});
test("A live rounded shape redraws at its new size; transform cancel restores source and saved radius",async({page})=>{
  await setup(page);await page.evaluate(()=>{const s=(window as any).__compositor.store.getState();s.run({type:"AddShape",shape:{kind:"Rectangle",rect:{x:20,y:20,width:30,height:20},cornerRadius:6},color:[1,0,0]});s.setTool("move");s.beginTransform({persistent:true});const e=s.transformEdit??(window as any).__compositor.store.getState().transformEdit;s.previewTransform({...e.draft,size:[90,60]});});
  const preview=await state(page);expect(preview.undoDepth).toBe(1);await page.keyboard.press("Escape");let d=await state(page);expect(d.layers[1].transform.size).toEqual([30,20]);
  await page.evaluate(()=>{const s=(window as any).__compositor.store.getState();s.beginTransform({persistent:true});const e=(window as any).__compositor.store.getState().transformEdit;s.previewTransform({...e.draft,size:[90,60]});s.commitTransform();});d=await state(page);expect([d.layers[1].pixelsWidth,d.layers[1].pixelsHeight]).toEqual([90,60]);expect(d.layers[1].shape.cornerRadius).toBe(6);expect(await page.evaluate(()=>{const a=(window as any).__compositor,s=a.store.getState(),l=s.documents[s.activeId].layers[1],bytes=a.engine.layerPixels(s.activeId,l.id);return bytes[(6*4)+3];})).toBe(255);await page.keyboard.press("Control+z");expect((await state(page)).layers[1].transform.size).toEqual([30,20]);
});

test("a Mac Verdana-Bold record renders the installed bold face and keeps its saved name",async({page})=>{
  await setup(page);
  const result=await page.evaluate(async()=>{
    const a=(window as any).__compositor,s=a.store.getState();
    const style={content:"Bwm",fontName:"Verdana-Bold",fontSize:36,red:0,green:0,blue:0,alignment:"Left",tracking:0,leading:0};
    const r=await s.jobs.run("font-face-regression",{kind:"text",input:JSON.stringify(style),pixels:null,mask:null});
    const {width,height}=JSON.parse(r.header),canvas=new OffscreenCanvas(width,height),ctx=canvas.getContext("2d",{willReadFrequently:true})!;
    ctx.font='bold 36px "Verdana"';ctx.textBaseline="alphabetic";
    const descent=ctx.measureText("Mg").fontBoundingBoxDescent||36*.2;
    ctx.fillText("Bwm",12,12+36*1.2-descent);
    const expected=ctx.getImageData(0,0,width,height).data,actual=new Uint8Array(r.pixels);
    const different=actual.reduce((n,v,i)=>n+(v!==expected[i]?1:0),0);
    a.engine.installText(s.activeId,null,style,width,height,r.pixels,[40,40],null);
    const saved=a.engine.savePackage(s.activeId),id=a.engine.openPackage(saved,null);
    return {different,fontName:a.engine.state(id).layers.find((l:any)=>l.text)?.text.fontName,ink:actual.some(v=>v>0)};
  });
  expect(result.ink).toBe(true);expect(result.different).toBe(0);expect(result.fontName).toBe("Verdana-Bold");
});
