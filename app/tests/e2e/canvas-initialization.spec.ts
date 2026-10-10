import {test,expect} from "@playwright/test";

test("the precise affine shader is linked before a document's first fitted redraw",async({page})=>{
  await page.addInitScript(()=>{
    const p=WebGL2RenderingContext.prototype,link=p.linkProgram;
    (window as any).__canvasLinks=0;
    p.linkProgram=function(program){(window as any).__canvasLinks++;return link.call(this,program);};
  });
  await page.goto("/");await expect(page.getByTestId("engine-ready")).toBeVisible();
  const result=await page.evaluate(async()=>{
    const a=(window as any).__compositor,r=a.renderer,e=a.engine,s=()=>a.store.getState();
    const gl=(document.querySelector('[data-testid="canvas-view"] canvas') as HTMLCanvasElement).getContext("webgl2")!;
    const program=r.programs.enlargedLayer,linked=!!program&&gl.getProgramParameter(program.program,gl.LINK_STATUS);
    const before=(window as any).__canvasLinks,doc=e.newDocument(64,48,true),id=e.state(doc).layers[0].id;
    e.execute(doc,{type:"Fill",id,mask:false,color:[.2,.4,.6]});s().openDocument(doc);
    await a.setZoom(2.3);
    const state=s();r.render(e,state.documents[doc],state.viewports[doc],window.devicePixelRatio||1,{checkerboard:true},null);
    const pixels=r.readPixels(),after=(window as any).__canvasLinks;
    s().closeDocument(doc);
    return{linked,noDrawTimeLink:before===after,picture: pixels.some((v:number)=>v!==0)};
  });
  expect(result).toEqual({linked:true,noDrawTimeLink:true,picture:true});
});
