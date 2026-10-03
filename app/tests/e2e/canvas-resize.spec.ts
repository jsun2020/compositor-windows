import {test,expect} from "@playwright/test";
import {redSquarePngBase64} from "./helpers";

test("tool options height changes reset only canvas height and preserve clipped-group pixels",async({page})=>{
  await page.setViewportSize({width:1280,height:720});
  await page.goto("/");await expect(page.getByTestId("engine-ready")).toBeVisible();
  const b64=await page.evaluate(redSquarePngBase64);
  const result=await page.evaluate(async(b64)=>{
    const api=(window as any).__compositor,bytes=Uint8Array.from(atob(b64),c=>c.charCodeAt(0)),doc=api.engine.newDocument(8,8,false);
    api.engine.importImage(doc,bytes,"Base",{x:2,y:2});const a=api.engine.state(doc).activeLayerId;
    api.engine.importImage(doc,bytes,"Clipped",{x:3,y:3});const b=api.engine.state(doc).activeLayerId;
    for(const id of [a,b]){const t=api.engine.state(doc).layers.find((l:any)=>l.id===id).transform;api.engine.execute(doc,{type:"SetLayerTransform",id,transform:{...t,sampling:"Nearest"}});}
    api.engine.execute(doc,{type:"GroupLayers",ids:[a,b]});
    const folder=api.engine.state(doc).layers.find((l:any)=>l.isGroup).id;
    api.engine.execute(doc,{type:"SetLayerOpacity",id:folder,opacity:.7});
    api.engine.execute(doc,{type:"ToggleClipping",id:b});
    api.store.getState().openDocument(doc);api.store.getState().setTool("eyedropper");await api.setZoom(1);api.setCheckerboard(false);
    const settle=()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(()=>requestAnimationFrame(r))));
    const check=async()=>{
      await settle();const s=api.store.getState(),vp=s.viewports[doc],rect=vp.documentRect({width:8,height:8}),dpr=window.devicePixelRatio||1;
      vp.translate({width:(Math.round(rect.x*dpr)-rect.x*dpr)/dpr,height:(Math.round(rect.y*dpr)-rect.y*dpr)/dpr});
      api.renderer.render(api.engine,s.documents[doc],vp,dpr,{checkerboard:false},null);
      const pixels=api.readDocumentPixels(),cpu=api.engine.composite(doc,{x:0,y:0,width:8,height:8},8,8);
      return{height:api.renderer.canvas.height,length:pixels.length,expectedLength:cpu.length,worst:pixels.reduce((m:number,v:number,i:number)=>Math.max(m,Math.abs(v-cpu[i])),0)};
    };
    const before=await check(),canvas=api.renderer.canvas as HTMLCanvasElement,writes={width:0,height:0};
    for(const axis of ["width","height"] as const){
      const descriptor=Object.getOwnPropertyDescriptor(HTMLCanvasElement.prototype,axis)!;
      Object.defineProperty(canvas,axis,{configurable:true,get:()=>descriptor.get!.call(canvas),set:(value:number)=>{writes[axis]++;descriptor.set!.call(canvas,value);}});
    }
    try{
      api.store.getState().setTool("gradient");const during=await check();api.store.getState().setTool("eyedropper");const after=await check();
      return{kind:api.renderer.kind,heightChanged:during.height!==before.height,heightRestored:after.height===before.height,writes:{...writes},comparisons:[before,during,after].map(r=>({length:r.length,expectedLength:r.expectedLength,worst:r.worst}))};
    }finally{delete (canvas as any).width;delete (canvas as any).height;}
  },b64);
  expect(result.kind).toBe("gl");expect(result.heightChanged).toBe(true);expect(result.heightRestored).toBe(true);expect(result.writes.width).toBe(0);expect(result.writes.height).toBe(2);
  for(const r of result.comparisons){expect(r.length).toBe(r.expectedLength);expect(r.worst).toBeLessThanOrEqual(2);}
});
