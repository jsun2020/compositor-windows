import {it,expect,vi} from "vitest";
import {WarpClient,type WarpMessage,type WarpReply} from "../../src/engine/warp-client";
class WorkerFake{
  onmessage:((e:MessageEvent<WarpReply>)=>void)|null=null;
  onerror:((e:ErrorEvent)=>void)|null=null;
  sent:WarpMessage[]=[];terminated=false;
  postMessage(m:WarpMessage){this.sent.push(m);}terminate(){this.terminated=true;}
  reply(m:WarpReply){this.onmessage!({data:m} as MessageEvent<WarpReply>);}
}
function setup(){const worker=new WorkerFake(),preview=vi.fn(),failed=vi.fn();const client=new WarpClient(worker as unknown as Worker,{} as WebAssembly.Module,
  {input:"original stamp",pixels:new ArrayBuffer(16),mask:null,points:new ArrayBuffer(0)},
  {mode:"Smudge",diameter:2,hardness:0.5,strength:0.5},preview,failed);return{client,worker,preview,failed};}
it("buffers early input and pointer-up until the immutable snapshot is ready",async()=>{
  const {client,worker,preview}=setup();client.append([1,1]);client.append([2,1]);const done=client.finish(0.5);
  expect(worker.sent).toHaveLength(1);worker.reply({type:"ready",backend:"GPU"});
  expect(worker.sent[1]).toEqual({type:"update",points:[[1,1],[2,1]],finish:true,outPerDoc:0.5});
  worker.reply({type:"done",result:null});expect(await done).toBeNull();expect(preview).not.toHaveBeenCalled();expect(worker.terminated).toBe(true);
});
it("coalesces previews without dropping or reordering pointer positions",async()=>{
  const {client,worker,preview}=setup();worker.reply({type:"ready",backend:"GPU"});client.append([1,1]);client.append([2,1]);client.append([3,1]);
  expect(worker.sent).toHaveLength(2);worker.reply({type:"preview",result:null});
  expect(worker.sent[2]).toMatchObject({points:[[2,1],[3,1]],finish:false});
  const done=client.finish(1);expect(worker.sent).toHaveLength(3);worker.reply({type:"preview",result:null});
  expect(worker.sent[3]).toEqual({type:"update",points:[],finish:true,outPerDoc:1});
  worker.reply({type:"done",result:null});await done;expect(preview).toHaveBeenCalledTimes(2);
});
it("cancellation settles completion and ignores a late preview or final reply",async()=>{
  const {client,worker,preview,failed}=setup();worker.reply({type:"ready",backend:"CPU"});client.append([1,1]);const done=client.finish(1);client.dispose();
  worker.reply({type:"preview",result:null});worker.reply({type:"done",result:null});
  expect(await done).toBeNull();expect(preview).not.toHaveBeenCalled();expect(failed).not.toHaveBeenCalled();expect(worker.terminated).toBe(true);
});
it("worker failure before pointer-up reports once and rejects completion",async()=>{
  const {client,worker,failed}=setup();worker.reply({type:"failed",error:"GPU context lost"});
  await expect(client.finish(1)).rejects.toThrow("GPU context lost");expect(failed).toHaveBeenCalledTimes(1);expect(worker.terminated).toBe(true);
});
it("excessive queued input refuses before unbounded accumulation",async()=>{
  const {client,worker,failed}=setup();for(let i=0;i<4097;i++)client.append([i,0]);
  expect(worker.sent).toHaveLength(1);expect(worker.terminated).toBe(true);expect(failed).toHaveBeenCalledTimes(1);
  await expect(client.finish(1)).rejects.toThrow("too long");
});
