import {test,expect,type Page} from "@playwright/test";
import {clickMenu} from "./helpers";
const u16=(n:number)=>{const b=Buffer.alloc(2);b.writeUInt16BE(n&65535);return b;};
const u32=(n:number)=>{const b=Buffer.alloc(4);b.writeUInt32BE(n);return b;};
const concat=(values:Uint8Array[])=>Buffer.concat(values);
function photoshop(big:boolean,unsupported=false):Buffer{
  const length=(n:number)=>{if(!big)return u32(n);const b=Buffer.alloc(8);b.writeBigUInt64BE(BigInt(n));return b;};
  const block=(key:string,data:Buffer)=>concat([Buffer.from("8BIM"+key),u32(data.length),data,Buffer.alloc(data.length%2)]);
  const records:Buffer[]=[],planes:Buffer[]=[];
  for(const [index,name] of ["Top 中文","Bottom"].entries()){
    const channels=[-1,0,1,2,...(index===0&&!unsupported?[-2]:[])];
    const data=channels.map(id=>concat([u16(0),Buffer.from(id===-2?[0,255,128,255]:Array(4).fill(id===-1?128:80+id*30))]));
    let mask=Buffer.alloc(0);if(channels.includes(-2))mask=concat([[0,0,2,2].map(u32).reduce((a,b)=>concat([a,b])),Buffer.from([255,0,0,0])]);
    const text=Buffer.from(name,"utf16le").swap16();
    const extra=concat([u32(mask.length),mask,u32(0),Buffer.alloc(4),block("luni",concat([u32(text.length/2),text])),...(unsupported&&index===0?[block("post",u16(3))]:[])]);
    records.push(concat([[0,0,2,2].map(u32).reduce((a,b)=>concat([a,b])),u16(channels.length),...channels.map((id,i)=>concat([u16(id),length(data[i].length)])),Buffer.from(index===0?"8BIMscrn":"8BIMnorm"),Buffer.from([200,index===0&&!unsupported?1:0,index===0?2:0,0]),u32(extra.length),extra]));planes.push(...data);
  }
  let layers=concat([u16(2),...records,...planes]);layers=concat([layers,Buffer.alloc(layers.length%2)]);
  const section=concat([length(layers.length),layers,u32(0)]);
  return concat([Buffer.from("8BPS"),u16(big?2:1),Buffer.alloc(6),u16(3),u32(2),u32(2),u16(8),u16(3),u32(0),u32(0),length(section.length),section,u16(0),Buffer.alloc(12,90)]);
}
async function setup(page:Page,bytes:Buffer,path="layers.psd"){
  await page.goto("/");await expect(page.getByTestId("engine-ready")).toBeVisible();
  await page.evaluate(({data,path})=>{const api=(window as any).__compositor;api.bridge.setNextPick(path);api.bridge.seedFile(path,Uint8Array.from(atob(data),(c:string)=>c.charCodeAt(0)));},{data:bytes.toString("base64"),path});
}
test("PSD and PSB file routes retain layers, Unicode, masks, visibility, opacity, clipping and blend",async({page})=>{for(const big of [false,true]){await setup(page,photoshop(big),big?"layers.psb":"layers.psd");await clickMenu(page,"File","import");await expect.poll(()=>page.evaluate(()=>{const s=(window as any).__compositor.store.getState();return s.activeId?s.documents[s.activeId].layers.length:0;})).toBe(2);const doc=await page.evaluate(()=>{const s=(window as any).__compositor.store.getState();return s.documents[s.activeId];});expect(doc.width).toBe(2);expect(doc.height).toBe(2);const [bottom,top]=doc.layers;expect(top.name).toBe("Top 中文");expect(top.visible).toBe(false);expect(top.opacity).toBeCloseTo(200/255);expect(top.blendMode).toBe("Screen");expect(top.hasMask).toBe(true);expect(top.maskSourceId).toBe(bottom.id);}});
test("Photoshop import into an existing document is one undo step and malformed import is atomic",async({page})=>{await setup(page,photoshop(false));const before=await page.evaluate(()=>{const api=(window as any).__compositor,id=api.engine.newDocument(8,8,true);api.store.getState().openDocument(id);return api.engine.state(id);});await clickMenu(page,"File","import");await expect.poll(()=>page.evaluate(()=>{const s=(window as any).__compositor.store.getState();return s.documents[s.activeId].layers.length;})).toBe(before.layers.length+2);await page.keyboard.press("Control+z");const undone=await page.evaluate(()=>{const s=(window as any).__compositor.store.getState();return s.documents[s.activeId];});expect(undone.undoDepth).toBe(before.undoDepth);expect(undone.layers).toHaveLength(before.layers.length);await page.evaluate(()=>{const b=(window as any).__compositor.bridge;b.seedFile("invalid.psd",new Uint8Array([56,66,80,83,0]));b.setNextPick("invalid.psd");});await clickMenu(page,"File","import");await expect.poll(()=>page.evaluate(()=>(window as any).__compositor.store.getState().error)).toContain("Photoshop import");const after=await page.evaluate(()=>{const s=(window as any).__compositor.store.getState();return s.documents[s.activeId];});expect(after.layers).toEqual(undone.layers);expect(after.undoDepth).toBe(undone.undoDepth);});
test("unsupported Photoshop adjustment has a visible conversion report that keeps the import guard until dismissal",async({page})=>{await setup(page,photoshop(false,true));await clickMenu(page,"File","import");const report=page.getByRole("dialog",{name:"Photoshop import conversions",exact:true});await expect(report).toBeVisible();await expect(report).toContainText("unsupported and was skipped");await expect(report).toContainText("Top 中文");expect(await page.evaluate(()=>(window as any).__compositor.store.getState().busy)).toBe(true);await report.getByRole("button",{name:"Close",exact:true}).click();await expect(report).toHaveCount(0);await expect.poll(()=>page.evaluate(()=>(window as any).__compositor.store.getState().busy)).toBe(false);const count=await page.evaluate(()=>{const s=(window as any).__compositor.store.getState();return s.documents[s.activeId].layers.length;});expect(count).toBe(1);});
