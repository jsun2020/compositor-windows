import {test,expect} from "@playwright/test";
test.skip(!process.env.PERF,"set PERF=1 with release WASM");
test.use({channel:"msedge",viewport:{width:1440,height:900}});
for(const [name,width,height] of [["24 MP",6000,4000],["100 MP",10000,10000]] as const){
  test(`brush, blur, healing and fill keep the UI responsive on ${name}`,async({page})=>{
    test.setTimeout(600_000);await page.goto("/");await expect(page.getByTestId("engine-ready")).toBeVisible();
    const result=await page.evaluate(async([width,height])=>{
      const a=(window as any).__compositor,s=a.store.getState();
      const gl=(document.querySelector('[data-testid="canvas-view"] canvas') as HTMLCanvasElement).getContext("webgl2")!;
      const info=gl.getExtension("WEBGL_debug_renderer_info"),gpu=info?String(gl.getParameter(info.UNMASKED_RENDERER_WEBGL)):"unknown";
      const settle=()=>new Promise<void>(r=>requestAnimationFrame(()=>requestAnimationFrame(()=>r())));
      const make=(w:number,h:number)=>{const d=a.engine.newDocument(10,10,false);a.engine.execute(d,{type:"CanvasSize",width:w,height:h,anchor:4,fill:[0.6,0.4,0.2]});const id=a.engine.state(d).layers[0].id;a.engine.execute(d,{type:"SetActiveLayer",id});s.openDocument(d);return d;};
      // Warm the worker/module on a small document; do not charge compilation to an edit.
      const warm=make(64,64);let id=a.engine.state(warm).activeLayerId;
      const brush={diameter:40,hardness:0.5,opacity:0.5,color:[1,0,0,1],erasing:false,points:[[32,32]],operation:{kind:"Paint"}};
      if(!await a.store.getState().runEditJob({type:"BrushStroke",id,brush},id))throw Error(a.store.getState().error||"warm edit refused");await settle();
      a.engine.closeDocument(warm);const d=make(width,height);id=a.engine.state(d).activeLayerId;await settle();
      const measures:Record<string,{total:number;maxGap:number}>={};
      async function measure(name:string,command:any){let running=true,gap=0,last=performance.now();const tick=()=>{const now=performance.now();gap=Math.max(gap,now-last);last=now;if(running)requestAnimationFrame(tick);};requestAnimationFrame(tick);await settle();gap=0;const t=performance.now();const ok=await a.store.getState().runEditJob(command,id);await settle();running=false;if(!ok)throw Error(a.store.getState().error||"edit refused");measures[name]={total:performance.now()-t,maxGap:gap};}
      for(const [label,operation] of [["Paint",{kind:"Paint"}],["Blur",{kind:"Blur",radius:5}],["Heal",{kind:"Heal",mode:"Content-Aware",seed:1}]] as const){await measure(label,{type:"BrushStroke",id,brush:{...brush,points:[[width/2,height/2],[width/2+40,height/2]],operation}});}
      a.engine.execute(d,{type:"SelectShape",kind:"Rectangle",points:[[width/2,height/2],[width/2+12,height/2],[width/2+12,height/2+12],[width/2,height/2+12]],mode:"Replace",antialiased:false});a.store.getState().refresh(d);
      await measure("ContentFill",{type:"ContentAwareFill",id});return{gpu,measures,memory:a.engine.wasmBytes()};
    },[width,height]);
    console.log(JSON.stringify({size:name,...result}));expect(result.gpu).not.toMatch(/SwiftShader|Basic Render|unknown/i);
    for(const measure of Object.values(result.measures)){expect(measure.total).toBeLessThan(120_000);expect(measure.maxGap).toBeLessThan(100);}
  });
}
