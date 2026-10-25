import {readFileSync} from "node:fs";
import {test,expect,type Page} from "@playwright/test";
import {clickMenu,noisePngBase64} from "./helpers";

async function ready(page:Page){await page.goto("/");await expect(page.getByTestId("engine-ready")).toBeVisible();}
async function photo(page:Page){await ready(page);const data=await page.evaluate(noisePngBase64);await page.evaluate(data=>{const a=(window as any).__compositor;a.store.getState().openDocument(a.engine.importImage(null,Uint8Array.from(atob(data),(c:string)=>c.charCodeAt(0)),"Photo",null));},data);}
async function state(page:Page){return page.evaluate(()=>{const s=(window as any).__compositor.store.getState();return s.documents[s.activeId];});}
async function pixels(page:Page){return page.evaluate(()=>{const a=(window as any).__compositor,s=a.store.getState();return Array.from(a.engine.composite(s.activeId,{x:0,y:0,width:64,height:64},64,64));});}
async function field(page:Page,name:string,value:string){await page.getByLabel(name,{exact:true}).fill(value);await page.getByLabel(name,{exact:true}).press("Enter");}

test("New Canvas converts physical units, retains DPI/background and reuses the lowest free title",async({page})=>{
  await ready(page);await clickMenu(page,"File","new");
  for(const unit of ["in","cm","mm","px"])await page.getByLabel("Units",{exact:true}).selectOption(unit);await expect(page.getByLabel("Width",{exact:true})).toHaveValue("1920");await expect(page.getByLabel("Height",{exact:true})).toHaveValue("1080");await expect(page.getByRole("button",{name:"Create",exact:true})).toBeEnabled();
  await page.getByLabel("Units",{exact:true}).selectOption("in");await page.getByLabel("Resolution",{exact:true}).selectOption("300");
  await page.getByLabel("Width",{exact:true}).fill("1");await page.getByLabel("Height",{exact:true}).fill("2");await page.getByLabel("Background",{exact:true}).selectOption("White");await page.getByRole("button",{name:"Create",exact:true}).click();
  expect(await state(page)).toMatchObject({width:300,height:600,resolution:300,isModified:false,undoDepth:0});
  expect((await pixels(page)).every(x=>x===255)).toBe(true);
  await clickMenu(page,"File","new");await page.getByLabel("Width",{exact:true}).fill("20");await page.getByLabel("Height",{exact:true}).fill("10");await page.getByRole("button",{name:"Create",exact:true}).click();
  await expect(page.getByTestId("project-tab").nth(1)).toContainText("Untitled 2");
  await page.getByTestId("project-tab").first().getByRole("button").click();
  await clickMenu(page,"File","new");await page.getByLabel("Width",{exact:true}).fill("20");await page.getByLabel("Height",{exact:true}).fill("10");await page.getByRole("button",{name:"Create",exact:true}).click();
  await expect(page.getByTestId("project-tab").last()).toContainText("Untitled");await expect(page.getByTestId("project-tab").last()).not.toContainText("Untitled 3");
});

test("Rotate Canvas menu swaps dimensions and remains a single reversible placement edit",async({page})=>{
  await ready(page);await page.evaluate(()=>{const a=(window as any).__compositor;a.store.getState().openDocument(a.engine.newDocument(20,10));});
  const view=()=>page.evaluate(()=>{const s=(window as any).__compositor.store.getState(),v=s.viewports[s.activeId];return {zoom:v.zoom,pan:{...v.pan},followsFit:v.followsFit};});
  const initial=await view();expect(initial.followsFit).toBe(true);
  await clickMenu(page,"Image","rotate-cw");expect(await state(page)).toMatchObject({width:10,height:20,undoDepth:1});
  // 1.5.7 dimension changes keep zoom and recenter, ending automatic fit.
  const rotated=await view();expect(initial.zoom).toBe(1);expect(rotated).toEqual({zoom:1,pan:{width:0,height:0},followsFit:false});
  await page.keyboard.press("Control+z");expect(await state(page)).toMatchObject({width:20,height:10,undoDepth:0});
  await expect.poll(view).toEqual(rotated);await page.keyboard.press("Control+Shift+z");await expect.poll(view).toEqual(rotated);
  await page.evaluate(()=>{const s=(window as any).__compositor.store.getState(),v=s.viewports[s.activeId];v.setZoom(2,v.center,s.documents[s.activeId]);v.translate({width:12,height:9});s.invalidate();});
  expect(await view()).toMatchObject({zoom:2,pan:{width:12,height:9},followsFit:false});
  const recentered={zoom:2,pan:{width:0,height:0},followsFit:false};
  await clickMenu(page,"Image","rotate-ccw");await expect.poll(view).toEqual(recentered);await page.keyboard.press("Control+z");await expect.poll(view).toEqual(recentered);
});

test("command search exposes disabled commands and selects a tool mode with the keyboard",async({page})=>{
  await ready(page);await page.keyboard.press("Control+f");await page.getByRole("textbox",{name:"Search Commands",exact:true}).fill("Camera Raw");
  await expect(page.getByRole("option")).toBeDisabled();await page.keyboard.press("Enter");await expect(page.getByRole("dialog",{name:"Search Commands",exact:true})).toBeVisible();
  await page.getByRole("textbox",{name:"Search Commands",exact:true}).fill("Liquify Mode");await page.keyboard.press("Enter");
  await expect(page.getByRole("dialog",{name:"Search Commands",exact:true})).toHaveCount(0);
  expect(await page.evaluate(()=>{const s=(window as any).__compositor.store.getState();return [s.tool,s.brushOptions.smearMode];})).toEqual(["blur","Liquify"]);
});

test("Scanlines Preview/Cancel and Last Filter preserve successful parameters and undo boundaries",async({page})=>{
  await photo(page);const original=await pixels(page);await clickMenu(page,"Filter","filter-scanlines");await field(page,"Line Spacing","7");
  await expect.poll(()=>pixels(page)).not.toEqual(original);await page.getByTestId("adjust-cancel").click();expect(await pixels(page)).toEqual(original);
  expect(await page.evaluate(()=>(window as any).__compositor.store.getState().lastFilter)).toBeNull();
  await clickMenu(page,"Filter","filter-scanlines");await field(page,"Line Spacing","7");await page.getByTestId("adjust-ok").click();
  const applied=await pixels(page),depth=(await state(page)).undoDepth;
  await clickMenu(page,"Filter","filter-gaussian-blur");await field(page,"Radius","3");await page.getByTestId("adjust-cancel").click();
  expect(await page.evaluate(()=>(window as any).__compositor.store.getState().lastFilter.settings.lineSpacing)).toBe(7);
  await page.keyboard.press("Control+Alt+f");expect((await state(page)).undoDepth).toBe(depth+1);await page.keyboard.press("Control+z");expect(await pixels(page)).toEqual(applied);
});

test("Export As previews requested dimensions and writes JPEG/PDF without modifying the project",async({page})=>{
  await photo(page);const before=await state(page);await clickMenu(page,"File","export-as");await page.getByLabel("Export Width",{exact:true}).fill("32");
  await expect(page.getByLabel("Export Height",{exact:true})).toHaveValue("32");await page.getByLabel("Format",{exact:true}).selectOption("JPEG");await expect(page.getByLabel("Export Background",{exact:true})).toHaveValue("White");
  await expect(page.getByTestId("export-preview")).toBeVisible();await page.evaluate(()=>(window as any).__compositor.bridge.setNextPick("C:/exports/new.jpg"));await page.getByRole("button",{name:"Export",exact:true}).click();
  await expect(page.getByRole("dialog",{name:"Export As",exact:true})).toHaveCount(0);
  const dimensions=await page.evaluate(async()=>{const bytes=await (window as any).__compositor.bridge.readFile("C:/exports/new.jpg");const image=await createImageBitmap(new Blob([bytes],{type:"image/jpeg"}));return [image.width,image.height];});expect(dimensions).toEqual([32,32]);expect(await state(page)).toEqual(before);
  await clickMenu(page,"File","export-as");await page.getByLabel("Format",{exact:true}).selectOption("PDF");await page.evaluate(()=>(window as any).__compositor.bridge.setNextPick("C:/exports/new.pdf"));await page.getByRole("button",{name:"Export",exact:true}).click();
  await expect(page.getByRole("dialog",{name:"Export As",exact:true})).toHaveCount(0);
  const pdf=await page.evaluate(async()=>new TextDecoder("latin1").decode(await (window as any).__compositor.bridge.readFile("C:/exports/new.pdf")));expect(pdf).toContain("%PDF-1.4");expect(pdf).toContain("/MediaBox [0 0 64.00000000 64.00000000]");expect(await state(page)).toEqual(before);
});

test("Navigator and Ctrl+middle zoom move only the viewport, with a fixed zoom anchor",async({page})=>{
  await photo(page);await clickMenu(page,"View","navigator");await page.evaluate(()=>{const a=(window as any).__compositor,s=a.store.getState(),d=s.documents[s.activeId];s.viewports[s.activeId].setZoom(16,{x:250,y:250},d);s.invalidate();});
  await expect(page.getByTestId("navigator")).toBeVisible();const before=await state(page);
  await page.getByTestId("navigator").click({position:{x:20,y:20}});expect(await state(page)).toEqual(before);
  const box=(await page.getByTestId("canvas-view").boundingBox())!,anchor={x:120,y:150};
  const point=()=>page.evaluate(anchor=>{const s=(window as any).__compositor.store.getState();return s.viewports[s.activeId].documentPoint(anchor,s.documents[s.activeId]);},anchor);
  const p=await point();await page.keyboard.down("Control");await page.mouse.move(box.x+anchor.x,box.y+anchor.y);await page.mouse.down({button:"middle"});await page.mouse.move(box.x+anchor.x,box.y+anchor.y-30,{steps:3});await page.mouse.up({button:"middle"});await page.keyboard.up("Control");
  const after=await point();expect(after.x).toBeCloseTo(p.x,5);expect(after.y).toBeCloseTo(p.y,5);expect(await state(page)).toEqual(before);
  await clickMenu(page,"View","navigator");await expect(page.getByTestId("navigator")).toHaveCount(0);
});

test("F respects text inputs; Escape and closing the last tab restore the normal workspace",async({page})=>{
  await ready(page);await page.evaluate(()=>{const a=(window as any).__compositor;a.store.getState().openDocument(a.engine.newDocument(20,10));});
  await page.keyboard.press("f");await expect(page.locator(".app")).toHaveClass(/canvas-only/);await page.keyboard.press("Escape");await expect(page.locator(".app")).not.toHaveClass(/canvas-only/);
  await clickMenu(page,"File","new");await page.getByLabel("Width",{exact:true}).fill("f");await expect(page.locator(".app")).not.toHaveClass(/canvas-only/);await page.keyboard.press("Escape");
  await page.keyboard.press("f");await page.keyboard.press("Control+w");await expect(page.getByTestId("project-tab")).toHaveCount(0);await expect(page.locator(".app")).not.toHaveClass(/canvas-only/);await expect(page.getByText("Open an image or create a new canvas",{exact:true})).toBeVisible();
});


test("lossy/lossless transparent WebP import/drop, save/reopen and header budget",async({page})=>{
  await ready(page);
  for(const [index,name] of ["lossless","lossy"].entries()){
    const source=Array.from(readFileSync('engine/tests/fixtures/mac149-webp/'+name+'.webp'));
    const expected=Array.from(readFileSync('engine/tests/fixtures/mac149-webp/'+name+'-pillow.rgba')).map((v,i,a)=>i%4===3?v:Math.floor((v*a[i-i%4+3]+127)/255));
    const path='C:/imports/'+name+'.webp';
    await page.evaluate(({source,path,index})=>{const a=(window as any).__compositor;a.bridge.seedFile(path,Uint8Array.from(source));if(index===0)a.bridge.setNextPick(path);else a.bridge.simulateDrop([path],null);},{source,path,index});
    if(index===0)await clickMenu(page,'File','import');
    await expect(page.getByTestId('layer-row')).toHaveCount(1);expect(await state(page)).toMatchObject({width:24,height:18});
    const bytes=await page.evaluate(()=>{const a=(window as any).__compositor,s=a.store.getState();return Array.from(a.engine.composite(s.activeId,{x:0,y:0,width:24,height:18},24,18)) as number[];});
    expect(bytes.length).toBe(expected.length);for(let i=0;i<bytes.length;i++){if(i%4===3)expect(bytes[i]).toBe(expected[i]);else expect(Math.abs(bytes[i]-expected[i])).toBeLessThanOrEqual(1);}
    const saved='C:/exports/'+name+'.comp';await page.evaluate(saved=>(window as any).__compositor.bridge.setNextPick(saved),saved);await clickMenu(page,'File','save-as');await expect(page.getByTestId('project-tab')).not.toContainText('•');await clickMenu(page,'File','close');await expect(page.getByTestId('project-tab')).toHaveCount(0);
    await page.evaluate(saved=>(window as any).__compositor.bridge.setNextPick(saved),saved);await clickMenu(page,'File','open');await expect(page.getByTestId('layer-row')).toHaveCount(1);
    expect(await page.evaluate(()=>{const a=(window as any).__compositor,s=a.store.getState();return Array.from(a.engine.composite(s.activeId,{x:0,y:0,width:24,height:18},24,18));})).toEqual(bytes);await clickMenu(page,'File','close');
  }
  const over=Array.from(readFileSync('engine/tests/fixtures/mac149-webp/over-budget.webp'));
  await page.evaluate(over=>{const a=(window as any).__compositor;a.bridge.seedFile('C:/imports/over.webp',Uint8Array.from(over));a.bridge.setNextPick('C:/imports/over.webp');},over);await clickMenu(page,'File','import');await expect(page.getByTestId('error-banner')).toBeVisible();await expect(page.getByTestId('project-tab')).toHaveCount(0);
});
