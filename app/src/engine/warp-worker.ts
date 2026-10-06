import {initSync,WasmEngine} from "./pkg/compositor_engine.js";
import {GpuWarpStroke} from "../canvas/gpu-warp";
import type {WarpMessage,WarpReply} from "./warp-client";
import type {WarpSpec} from "./types";
import type {JobResult} from "./jobs";
let engine:WasmEngine|null=null,memory:WebAssembly.Memory|null=null,stroke:GpuWarpStroke|null=null,spec:WarpSpec|null=null,cpu=false;
const post=(m:WarpReply,transfer:ArrayBuffer[]=[])=> (self as unknown as Worker).postMessage(m,transfer);
function copy(mask:boolean):ArrayBuffer|null{
  const length=engine!.job_buffer_len(mask);return length?new Uint8Array(memory!.buffer,engine!.job_buffer_ptr(mask),length).slice().buffer:null;
}
self.onmessage=(e:MessageEvent<WarpMessage>)=>{
  try{
    const m=e.data;
    if(m.type==="init"){
      memory=initSync({module:m.module}).memory;engine=new WasmEngine();spec={...m.settings,points:[]};
      const input=JSON.parse(m.snapshot.input) as {width:number;height:number;pixels:[number,number]|null};
      const small=input.width*input.height<=4_194_304&&(!input.pixels||input.pixels[0]*input.pixels[1]<=4_194_304);
      let gl:WebGL2RenderingContext|null=null;
      try{gl=new OffscreenCanvas(1,1).getContext("webgl2",{antialias:false});}catch{/* Bounded CPU fallback below. */}
      if((!gl||!gl.getExtension("EXT_color_buffer_float"))&&!small)throw Error("Smudge and Liquify need WebGL2 float render targets; CPU fallback is limited to 4 Mi pixels");
      const b=(buffer:ArrayBuffer|null)=>buffer?new Uint8Array(buffer):undefined;
      const size=JSON.parse(engine.begin_warp_session(m.snapshot.input,b(m.snapshot.pixels),b(m.snapshot.mask),b(m.snapshot.points))) as {width:number;height:number};
      try{
        if(!gl)throw Error("Smudge and Liquify need WebGL2 float render targets on this canvas");
        stroke=new GpuWarpStroke(gl,size.width,size.height,new Uint8Array(memory.buffer,engine.job_buffer_ptr(false),engine.job_buffer_len(false)),m.settings);
      }catch(error){
        if(!small)throw error;
        cpu=true;
      }finally{engine.release_job();}
      post({type:"ready",backend:cpu?"CPU":"GPU"});return;
    }
    if(!engine||!spec)throw Error("No warp session");
    for(const point of m.points){stroke?.append(point);spec.points.push(point);}
    const tiles=stroke?.readTiles()??[];
    const header=engine.update_warp_session(JSON.stringify(spec),JSON.stringify(tiles.map(({x,y,width,height})=>({x,y,width,height}))),tiles.map(t=>t.pixels),m.finish,m.outPerDoc,cpu)??null;
    let result:JobResult|null=null;
    if(header){const length=engine.job_display_len();result={header,pixels:copy(false),mask:copy(true),display:length?new Uint8Array(memory!.buffer,engine.job_display_ptr(),length).slice().buffer:null};}
    engine.release_job();
    if(m.finish){stroke?.dispose();stroke=null;}
    post({type:m.finish?"done":"preview",result},[result?.pixels,result?.mask,result?.display].filter((b):b is ArrayBuffer=>b!=null));
  }catch(error){stroke?.dispose();stroke=null;post({type:"failed",error:error instanceof Error?error.message:String(error)});}
};
