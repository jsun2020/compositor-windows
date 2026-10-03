import { describe, expect, it } from "vitest";
import { EFFECTS_JOB_DISPLACED, JobClient, WORKER_MEMORY_LIMIT, type FromWorker, type JobRequest, type ToWorker } from "../../src/engine/jobs";
import {clearJobBuffers,releaseJobBuffer,takeJobBuffer,takeSpareJobBuffer} from "../../src/engine/job-buffers";

/** A worker that records what it is sent and answers when told to. */
class FakeWorker {
  sent: { message: ToWorker; transfer: unknown[] }[] = [];
  terminated = false;
  onmessage: ((e: MessageEvent<FromWorker>) => void) | null = null;
  onerror: ((e: ErrorEvent) => void) | null = null;
  postMessage(message: ToWorker, transfer: unknown[] = []) { this.sent.push({ message, transfer }); }
  terminate() { this.terminated = true; }
  reply(message: FromWorker) { this.onmessage!({ data: message } as MessageEvent<FromWorker>); }
  /** The ids of the jobs it was given, in order. */
  jobs(): number[] { return this.sent.flatMap((s) => (s.message.type === "job" ? [s.message.id] : [])); }
}
const settle = () => new Promise((r) => setTimeout(r, 0));
const module = {} as WebAssembly.Module;
// A job's selection (when it has one) travels as a separate `points` buffer (engine jobs.rs); these
// fixtures have no selection, so it is null, as `EngineClient.jobInput` returns for one.
const histogram = (tag: string): JobRequest => ({ kind: "histogram", input: tag, pixels: new ArrayBuffer(8), mask: null, points: null });
const effects = (tag: string): JobRequest => ({ kind: "effects", input: tag, pixels: new ArrayBuffer(8), mask: null, factor: 1, edit: null });
const done = (id: number, header: string, memory = 1): FromWorker => ({ type: "done", id, result: { header, pixels: null, mask: null }, memory });

function client() {
  const workers: FakeWorker[] = [];
  const jobs = new JobClient(module, () => { const w = new FakeWorker(); workers.push(w); return w as unknown as Worker; });
  return { jobs, workers };
}

describe("JobClient", () => {
  it("retires a canceled edit's returned large buffer without exposing it to the caller",async()=>{
    clearJobBuffers();
    try{
      const {jobs,workers}=client();
      const pending=jobs.run("edit",{kind:"edit",input:"edit",pixels:new ArrayBuffer(8),mask:null,points:null,command:"",outPerDoc:1});
      await settle();workers[0].reply({type:"ready"});await settle();jobs.cancel("edit");
      const buffer=new ArrayBuffer(4*1024*1024);new Uint8Array(buffer)[19]=99;
      workers[0].reply({type:"done",id:1,result:{header:"discarded",pixels:buffer,mask:null},memory:1});
      expect(await pending).toBeNull();expect(buffer.byteLength).toBe(0);
      expect(new Uint8Array(takeJobBuffer(4*1024*1024))[19]).toBe(99);
      jobs.dispose();
    }finally{clearJobBuffers();}
  });
  it("reclaims histogram input before its caller prepares the next job, including a canceled histogram",async()=>{
    clearJobBuffers();
    try{
      const {jobs,workers}=client(),pending=jobs.run("hist",histogram("one"));
      await settle();workers[0].reply({type:"ready"});await settle();jobs.cancel("hist");
      const buffer=new ArrayBuffer(4*1024*1024);new Uint8Array(buffer)[19]=77;
      workers[0].reply({...done(1,"bins"),recycled:[buffer]} as FromWorker);
      expect(await pending).toBeNull();expect(buffer.byteLength).toBe(0);
      expect(new Uint8Array(takeJobBuffer(4*1024*1024))[19]).toBe(77);
      jobs.dispose();
    }finally{clearJobBuffers();}
  });
  it("starts the worker with the compiled module, then runs one job at a time in order, transferring its buffers", async () => {
    const { jobs, workers } = client();
    const first = jobs.run("a", histogram("one"));
    const second = jobs.run("b", histogram("two"));
    await settle();
    expect(workers.length).toBe(1);
    expect(workers[0].sent[0].message).toEqual({ type: "init", module });
    workers[0].reply({ type: "ready" });
    await settle();
    expect(workers[0].jobs()).toEqual([1]);
    const sentJob = workers[0].sent[1];
    expect(sentJob.transfer).toEqual([(sentJob.message as { request: JobRequest }).request.pixels]);
    workers[0].reply(done(1, "first"));
    expect((await first)?.header).toBe("first");
    await settle();
    expect(workers[0].jobs()).toEqual([1, 2]);
    workers[0].reply(done(2, "second"));
    expect((await second)?.header).toBe("second");
  });

  it("drops a waiting job its channel superseded, and throws away a running one's result", async () => {
    const { jobs, workers } = client();
    const running = jobs.run("fx", histogram("1"));
    await settle(); workers[0].reply({ type: "ready" }); await settle();
    const waiting = jobs.run("fx", histogram("2"));
    const newest = jobs.run("fx", histogram("3"));
    expect(await waiting).toBeNull();
    workers[0].reply(done(1, "stale"));
    expect(await running, "superseded while it ran").toBeNull();
    await settle();
    expect(workers[0].jobs(), "the waiting one never ran").toEqual([1, 3]);
    workers[0].reply(done(3, "fresh"));
    expect((await newest)?.header).toBe("fresh");
  });

  it("cancels a channel, and a failed job rejects without stopping the next", async () => {
    const { jobs, workers } = client();
    const one = jobs.run("x", histogram("1"));
    const two = jobs.run("y", histogram("2"));
    jobs.cancel("y");
    expect(await two).toBeNull();
    await settle(); workers[0].reply({ type: "ready" }); await settle();
    workers[0].reply({ type: "failed", id: 1, error: "boom", memory: 1, fatal: false });
    await expect(one).rejects.toThrow("boom");
    expect(workers[0].terminated, "an ordinary refusal keeps the worker").toBe(false);
    const three = jobs.run("x", histogram("3"));
    await settle();
    expect(jobs.spawned, "the same worker, not a new one").toBe(1);
    workers[0].reply(done(3, "ok"));
    expect((await three)?.header).toBe("ok");
  });

  it("replaces a worker after a fatal wasm trap, unlike an ordinary refusal", async () => {
    // A panic (`unreachable`) or an allocation abort throws WebAssembly.RuntimeError in the worker,
    // which leaves wasm-bindgen's re-entrancy guard set: every later call on that instance would
    // throw "recursive use of an object..." if it were reused. Fix round 1, issue 2.
    const { jobs, workers } = client();
    const one = jobs.run("x", histogram("1"));
    await settle(); workers[0].reply({ type: "ready" }); await settle();
    workers[0].reply({ type: "failed", id: 1, error: "unreachable executed", memory: 1, fatal: true });
    // A panic's trap is not called a memory failure (final review minor 6).
    await expect(one).rejects.toThrow("The edit failed and was stopped.");
    expect(workers[0].terminated, "a poisoned worker cannot be reused").toBe(true);
    const two = jobs.run("x", histogram("2"));
    await settle();
    expect(jobs.spawned, "replaced with a fresh worker").toBe(2);
    workers[1].reply({ type: "ready" }); await settle();
    workers[1].reply(done(2, "ok"));
    expect((await two)?.header, "the next job runs normally on the new worker").toBe("ok");
  });

  it("warm() starts the worker before any job, once, and the first job then runs on it (final review F2)", async () => {
    const { jobs, workers } = client();
    jobs.warm();
    jobs.warm();
    expect(workers.length, "one worker, started at once").toBe(1);
    expect(workers[0].sent[0].message).toEqual({ type: "init", module });
    workers[0].reply({ type: "ready" });
    const one = jobs.run("x", histogram("1"));
    await settle();
    expect([jobs.spawned, workers[0].jobs()]).toEqual([1, [1]]);
    workers[0].reply(done(1, "ok"));
    expect((await one)?.header).toBe("ok");
  });

  it("says a trap ran out of memory only when the trap says so (final review minor 6)", async () => {
    const { jobs, workers } = client();
    const one = jobs.run("x", histogram("1"));
    await settle(); workers[0].reply({ type: "ready" }); await settle();
    workers[0].reply({ type: "failed", id: 1, error: "WebAssembly.Memory.grow(): Maximum memory size exceeded", memory: 1, fatal: true });
    await expect(one).rejects.toThrow(/ran out of memory/);
  });

  it("replaces a worker whose memory grew past the limit, and one that died", async () => {
    const { jobs, workers } = client();
    const one = jobs.run("x", histogram("1"));
    await settle(); workers[0].reply({ type: "ready" }); await settle();
    workers[0].reply(done(1, "big", WORKER_MEMORY_LIMIT + 1));
    expect((await one)?.header, "its result still arrives").toBe("big");
    expect(workers[0].terminated).toBe(true);
    const two = jobs.run("x", histogram("2"));
    await settle();
    expect(jobs.spawned).toBe(2);
    workers[1].reply({ type: "ready" }); await settle();
    workers[1].onerror!({ message: "out of memory" } as ErrorEvent);
    await expect(two).rejects.toThrow("out of memory");
    const three = jobs.run("x", histogram("3"));
    await settle();
    expect(jobs.spawned).toBe(3);
    workers[2].reply({ type: "ready" }); await settle();
    workers[2].reply(done(3, "again"));
    expect((await three)?.header).toBe("again");
  });

  it("queues an effects job behind a pending edit or histogram job, whichever order they were asked (fix round 1, issue 3)", async () => {
    const { jobs, workers } = client();
    const running = jobs.run("first", histogram("first")); // occupies the worker so both land in the queue
    await settle(); workers[0].reply({ type: "ready" }); await settle();
    const fx = jobs.run("fx:a", effects("fx"));
    const edit = jobs.run("edit:b", histogram("edit"));
    workers[0].reply(done(1, "first-done"));
    await expect(running).resolves.toMatchObject({ header: "first-done" });
    await settle();
    expect(workers[0].jobs(), "the edit job (id 3) runs before the effects job (id 2), though asked second").toEqual([1, 3]);
    workers[0].reply(done(3, "edit-done"));
    await expect(edit).resolves.toMatchObject({ header: "edit-done" });
    await settle();
    expect(workers[0].jobs()).toEqual([1, 3, 2]);
    workers[0].reply(done(2, "fx-done"));
    await expect(fx).resolves.toMatchObject({ header: "fx-done" });
  });

  it("an edit or histogram job preempts a running effects job by replacing the worker, rejecting the effects job with a distinct signal (fix round 1, issue 3)", async () => {
    const { jobs, workers } = client();
    const fx = jobs.run("fx:a", effects("fx"));
    await settle(); workers[0].reply({ type: "ready" }); await settle();
    expect(workers[0].jobs(), "the effects job is running").toEqual([1]);
    const edit = jobs.run("edit:b", histogram("edit"));
    // Rejected, not resolved null: an ordinary supersede or "nothing to draw" must not be confused
    // with "still needs making, ask again once the worker is free" (effects-images.ts's `ask`).
    await expect(fx, "the running effects job is dropped rather than waited out").rejects.toThrow(EFFECTS_JOB_DISPLACED);
    expect(workers[0].terminated, "its worker is replaced, as the fatal-trap path replaces one").toBe(true);
    await settle();
    expect(jobs.spawned, "a fresh worker starts the edit job").toBe(2);
    workers[1].reply({ type: "ready" }); await settle();
    expect(workers[1].jobs()).toEqual([2]);
    workers[1].reply(done(2, "edit-done"));
    expect((await edit)?.header).toBe("edit-done");
  });
});

it("transfers a blank canvas edit's exact-size spare as output capacity, never as pixel input",async()=>{
  clearJobBuffers();const {jobs,workers}=client();
  try{
    const bytes=new ArrayBuffer(4*1024*1024);new Uint8Array(bytes)[31]=61;releaseJobBuffer(bytes);
    const request:JobRequest={kind:"edit",input:JSON.stringify({width:1024,height:1024}),pixels:null,mask:null,points:null,command:JSON.stringify({type:"Gradient",mask:false}),outPerDoc:1};
    const pending=jobs.run("blank",request);await settle();
    const waitingSpare=takeSpareJobBuffer(4*1024*1024);
    expect(waitingSpare, "a waiting job does not take the spare before posting").not.toBeNull();
    releaseJobBuffer(waitingSpare!);
    workers[0].reply({type:"ready"});await settle();
    const sent=workers[0].sent[1];expect(request.pixels).toBeNull();
    expect(request.outputPixels?.byteLength).toBe(4*1024*1024);
    expect(sent.transfer).toEqual([request.outputPixels]);
    expect(takeSpareJobBuffer(4*1024*1024)).toBeNull();
    jobs.cancel("blank");workers[0].reply({type:"done",id:1,result:{header:"cancelled",pixels:request.outputPixels!,mask:null},memory:1});
    expect(await pending).toBeNull();expect(request.outputPixels!.byteLength).toBe(0);
    expect(new Uint8Array(takeJobBuffer(4*1024*1024))[31]).toBe(61);
  }finally{jobs.dispose();clearJobBuffers();}
});

it("does not loan a canvas pixel spare to a mask edit or a histogram",async()=>{
  clearJobBuffers();const {jobs,workers}=client();
  try{
    releaseJobBuffer(new ArrayBuffer(4*1024*1024));
    const request:JobRequest={kind:"edit",input:JSON.stringify({width:1024,height:1024}),pixels:null,mask:null,points:null,command:JSON.stringify({type:"Fill",mask:true}),outPerDoc:1};
    const pending=jobs.run("mask",request);await settle();workers[0].reply({type:"ready"});await settle();
    expect(request.outputPixels).toBeUndefined();expect(workers[0].sent[1].transfer).toEqual([]);
    workers[0].reply(done(1,"mask"));await pending;
    const hist=jobs.run("hist",histogram("bins"));await settle();
    expect(workers[0].sent.at(-1)!.transfer).toHaveLength(1);
    expect(takeSpareJobBuffer(4*1024*1024)?.byteLength).toBe(4*1024*1024);
    workers[0].reply(done(2,"bins"));await hist;
  }finally{jobs.dispose();clearJobBuffers();}
});
