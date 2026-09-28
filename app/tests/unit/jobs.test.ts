import { describe, expect, it } from "vitest";
import { JobClient, WORKER_MEMORY_LIMIT, type FromWorker, type JobRequest, type ToWorker } from "../../src/engine/jobs";

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
const done = (id: number, header: string, memory = 1): FromWorker => ({ type: "done", id, result: { header, pixels: null, mask: null }, memory });

function client() {
  const workers: FakeWorker[] = [];
  const jobs = new JobClient(module, () => { const w = new FakeWorker(); workers.push(w); return w as unknown as Worker; });
  return { jobs, workers };
}

describe("JobClient", () => {
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
    await expect(one).rejects.toThrow(/ran out of memory/);
    expect(workers[0].terminated, "a poisoned worker cannot be reused").toBe(true);
    const two = jobs.run("x", histogram("2"));
    await settle();
    expect(jobs.spawned, "replaced with a fresh worker").toBe(2);
    workers[1].reply({ type: "ready" }); await settle();
    workers[1].reply(done(2, "ok"));
    expect((await two)?.header, "the next job runs normally on the new worker").toBe("ok");
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
});
