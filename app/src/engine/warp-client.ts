import type { JobInputCopy } from "./client";
import type { JobResult } from "./jobs";
import type { PointTuple, WarpSpec } from "./types";
export type WarpMessage = {type:"init";module:WebAssembly.Module;snapshot:JobInputCopy;settings:Omit<WarpSpec,"points">}
  | {type:"update";points:PointTuple[];finish:boolean;outPerDoc:number};
export type WarpReply = {type:"ready";backend:"GPU"|"CPU"}
  | {type:"preview"|"done";result:JobResult|null}
  | {type:"failed";error:string};

/** A stroke has its own worker: background effects cannot displace its pinned
 * snapshot, and cancellation terminates both GPU resources and its WASM heap.
 * Pointer positions accumulate in order, while only one preview is in flight. */
export class WarpClient {
  private ready=false;
  private flight=false;
  private ending=false;
  private dead=false;
  private points:PointTuple[]=[];
  private count=0;
  private resolve!:(result:JobResult|null)=>void;
  private reject!:(error:Error)=>void;
  private readonly completed:Promise<JobResult|null>;
  backend:"GPU"|"CPU"|null=null;
  constructor(private readonly worker:Worker,module:WebAssembly.Module,snapshot:JobInputCopy,settings:Omit<WarpSpec,"points">,
    private readonly preview:(result:JobResult|null)=>void,private readonly failed:(error:Error)=>void){
    this.completed=new Promise((resolve,reject)=>{this.resolve=resolve;this.reject=reject;});
    // Failure before pointer-up is reported immediately; finish still rejects.
    void this.completed.catch(()=>{});
    worker.onmessage=(e:MessageEvent<WarpReply>)=>{
      if(this.dead)return;
      const m=e.data;
      if(m.type==="failed"){this.fail(new Error(m.error));return;}
      if(m.type==="ready"){this.ready=true;this.backend=m.backend;this.pump();return;}
      this.flight=false;
      if(m.type==="done"){this.dead=true;worker.terminate();this.resolve(m.result);return;}
      try{this.preview(m.result);}catch(error){this.fail(error instanceof Error?error:new Error(String(error)));return;}
      this.pump();
    };
    worker.onerror=e=>this.fail(new Error(e.message||"The warp worker stopped"));
    const m:WarpMessage={type:"init",module,snapshot,settings};
    worker.postMessage(m,[snapshot.pixels,snapshot.mask,snapshot.points].filter((b):b is ArrayBuffer=>b!==null&&b.byteLength>0));
  }
  append(point:PointTuple):void{
    if(this.dead||this.ending)return;
    if(++this.count>4096){this.fail(new Error("The warp stroke is too long"));return;}
    this.points.push([...point]);this.pump();
  }
  private scale=0;
  finish(outPerDoc:number):Promise<JobResult|null>{this.ending=true;this.scale=outPerDoc;this.pump();return this.completed;}
  private pump():void{
    if(this.dead||!this.ready||this.flight||(!this.ending&&!this.points.length))return;
    this.flight=true;
    const m:WarpMessage={type:"update",points:this.points.splice(0),finish:this.ending,outPerDoc:this.scale};
    this.worker.postMessage(m);
  }
  private fail(error:Error):void{if(this.dead)return;this.dead=true;this.worker.terminate();this.reject(error);this.failed(error);}
  dispose():void{if(this.dead)return;this.dead=true;this.worker.terminate();this.points=[];this.resolve(null);}
}
