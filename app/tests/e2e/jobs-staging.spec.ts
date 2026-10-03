import {test,expect} from "@playwright/test";

test("chunked result installation rejects incomplete planes and preserves exact pixels and undo across edits",async({page})=>{
  await page.goto("/");await expect(page.getByTestId("engine-ready")).toBeVisible();
  const result=await page.evaluate(async()=>{
    const a=(window as any).__compositor,w=a.engine.wasm;
    let overflow=false,incomplete=false;
    try{w.begin_staged_install(8,0,0);w.append_staged_install(0,new Uint8Array(9));}catch(e){overflow=String(e).includes("overflow");}finally{w.cancel_staged_install();}
    // A truncated transfer must be refused before any document mutation.
    const doc=a.engine.newDocument(10,10,false);
    a.engine.execute(doc,{type:"CanvasSize",width:1536,height:1024,anchor:4,fill:[.2,.4,.6]});
    const id=a.engine.state(doc).layers[0].id;
    a.engine.execute(doc,{type:"SetActiveLayer",id});
    a.engine.execute(doc,{type:"Fill",id,mask:false,color:[.2,.4,.6]});
    const input=a.engine.jobHeader(doc,id),h=JSON.parse(input);
    try{w.begin_staged_install(8,0,0);w.append_staged_install(0,new Uint8Array(4));w.finish_staged_install(doc,id,JSON.stringify(h.stamp),JSON.stringify({transform:h.layer.transform,maskPlacement:null,pixels:[2,1],mask:null,regions:[],display:null}),false);}catch(e){incomplete=String(e).includes("incomplete");}finally{w.cancel_staged_install();}
    a.store.getState().openDocument(doc);const before=a.engine.state(doc).undoDepth;
    const exact=(color:number[])=>{const b=a.engine.layerPixels(doc,id);for(let i=0;i<b.length;i++)if(b[i]!==color[i%4])return false;return true;};
    const first=await a.store.getState().runEditJob({type:"Fill",id,mask:false,color:[0,1,0]},id);
    const green=exact([0,255,0,255]),oneUndo=a.engine.state(doc).undoDepth===before+1;
    a.engine.undo(doc);const undo=exact([51,102,153,255]);a.engine.redo(doc);const redo=exact([0,255,0,255]);
    window.dispatchEvent(new Event("pagehide")); // Discarding a spare must not change committed pixels.
    const second=await a.store.getState().runEditJob({type:"Fill",id,mask:false,color:[1,0,0]},id);
    return{overflow,incomplete,first,green,oneUndo,undo,redo,second,red:exact([255,0,0,255]),error:a.store.getState().error};
  });
  expect(result).toEqual({overflow:true,incomplete:true,first:true,green:true,oneUndo:true,undo:true,redo:true,second:true,red:true,error:null});
});

test("blank fills and gradients reuse transferred output capacity and preserve every pixel and undo/redo",async({page})=>{
  await page.goto("/");await expect(page.getByTestId("engine-ready")).toBeVisible();
  const result=await page.evaluate(async()=>{
    const a=(window as any).__compositor,width=1536,height=1024,size=width*height*4;
    const make=()=>{const doc=a.engine.newDocument(width,height,false);a.engine.execute(doc,{type:"AddBlankLayer"});return doc;};
    const donor=make();a.store.getState().openDocument(donor);
    const donorLayer=a.engine.state(donor).layers[0].id;
    await a.store.getState().runEditJob({type:"Fill",id:donorLayer,mask:false,color:[.2,.4,.6]},donorLayer);
    const jobs=a.store.getState().jobs,worker=jobs.worker,original=worker.postMessage.bind(worker);
    const transfers:{pixelsNull:boolean;size:number;included:boolean;detached:boolean}[]=[];
    worker.postMessage=(message:any,buffers:any[])=>{
      if(message.type!=="job"||message.request.kind!=="edit")return original(message,buffers);
      const spare=message.request.outputPixels;
      const record={pixelsNull:message.request.pixels===null,size:spare?.byteLength??0,included:buffers.includes(spare),detached:false};
      original(message,buffers);record.detached=spare?.byteLength===0;transfers.push(record);
    };
    const checks=[];
    for(const type of ["Fill","Gradient"]){
      const doc=make(),reference=make();a.store.getState().openDocument(doc);
      const id=a.engine.state(doc).layers[0].id,other=a.engine.state(reference).layers[0].id;
      const command=type==="Fill"?{type,id,mask:false,color:[1,0,0]}:{type,id,mask:false,gradient:{shape:"Linear",start:[0,0],end:[width,height],from:[0,1,0,1],to:[0,0,1,1],opacity:.9}};
      const depth=a.engine.state(doc).undoDepth;
      const applied=await a.store.getState().runEditJob(command,id);
      a.engine.execute(reference,{...command,id:other});
      const expected=a.engine.layerPixels(reference,other).slice();
      const exact=()=>{const actual=a.engine.layerPixels(doc,id);return actual.length===expected.length&&actual.every((b:number,i:number)=>b===expected[i]);};
      const same=exact(),oneUndo=a.engine.state(doc).undoDepth===depth+1;
      a.engine.undo(doc);const blank=a.engine.layerPixels(doc,id)===null;
      a.engine.redo(doc);checks.push({applied,same,oneUndo,blank,redo:exact()});
    }
    return{checks,transfers,size,error:a.store.getState().error};
  });
  expect(result.error).toBeNull();
  expect(result.checks).toEqual(Array(2).fill({applied:true,same:true,oneUndo:true,blank:true,redo:true}));
  expect(result.transfers).toEqual(Array(2).fill({pixelsNull:true,size:result.size,included:true,detached:true}));
});
