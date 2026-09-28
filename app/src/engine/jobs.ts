// The job worker's client (Phase 4b-1): a second engine in a Web Worker runs work on one layer off
// the UI thread (engine `jobs.rs`). The main thread copies the layer's buffers out of wasm memory once
// and transfers them; results come back transferred. One job runs at a time.
//
// A job's selection (when it has one) crosses as a separate `points` buffer, not JSON (engine
// `jobs.rs`'s `JobSelection`): the flat i32 contour points, as raw bytes. When a job's JobInput has a
// selection, `points` is always a (possibly zero-length) ArrayBuffer, never null -- an explicit empty
// selection is Some with zero contours, and the engine's `points_of` tells that apart from no
// selection at all (None) only by whether a buffer was passed, not by its length.

/** What a job is asked to do. `input` is the engine's JobInput JSON; the buffers travel beside it. */
export type JobRequest =
  | { kind: "edit"; input: string; pixels: ArrayBuffer | null; mask: ArrayBuffer | null; points: ArrayBuffer | null; command: string }
  | { kind: "histogram"; input: string; pixels: ArrayBuffer | null; mask: ArrayBuffer | null; points: ArrayBuffer | null }
  | { kind: "effects"; input: string; pixels: ArrayBuffer; mask: ArrayBuffer | null; factor: number; edit: string | null };

/** What came back: the engine's JSON answer (an edit's JobOutput, a histogram's bins, an effects
 * image's size and inset; null when an effects image found nothing to draw) and any buffers. */
export interface JobResult { header: string | null; pixels: ArrayBuffer | null; mask: ArrayBuffer | null; }

/** Messages to the worker and back. `fatal` on a failure marks a wasm trap (an `unreachable` panic,
 * an allocation abort): `RuntimeError.prototype instanceof WebAssembly.RuntimeError`, as the worker's
 * catch tells apart from an ordinary `JsError` a command refused with. A trap leaves wasm-bindgen's
 * re-entrancy guard set on that instance, so every later call throws "recursive use of an object..."
 * -- the worker itself is unusable from here on, not just the one job, unlike a clean refusal. */
export type ToWorker = { type: "init"; module: WebAssembly.Module } | { type: "job"; id: number; request: JobRequest };
export type FromWorker = { type: "ready" } | { type: "done"; id: number; result: JobResult; memory: number } | { type: "failed"; id: number; error: string; memory: number; fatal: boolean };

/** A worker that has grown its wasm memory past this is replaced after its job: wasm memory never
 * shrinks, and a second heap of gigabytes would crowd the app's own. */
export const WORKER_MEMORY_LIMIT = 1024 * 1024 * 1024;

/** The buffers a request hands over, for `postMessage`'s transfer list. */
export function transferables(request: JobRequest): ArrayBuffer[] {
  const points = "points" in request ? request.points : null;
  return [request.pixels, request.mask, points].filter((b): b is ArrayBuffer => b !== null && b.byteLength > 0);
}

interface Pending { id: number; channel: string; request: JobRequest; resolve: (r: JobResult | null) => void; reject: (e: Error) => void; }

/** Runs jobs on one worker, one at a time, in the order asked. A job on a channel is superseded by a
 * newer one on the same channel: if it has not started it is dropped, and if it is running its result
 * is thrown away when it lands; either way its promise resolves null (the Mac checks for cancellation
 * at a render's start and end, EffectsPreviewCache.swift:101-110). */
export class JobClient {
  private worker: Worker | null = null;
  private ready: Promise<void> | null = null;
  private queue: Pending[] = [];
  private running: Pending | null = null;
  /** The newest job id asked for on each channel. */
  private newest = new Map<string, number>();
  private nextId = 1;
  /** How many workers have been started (a test watches the respawn). */
  spawned = 0;

  constructor(private readonly module: WebAssembly.Module, private readonly spawn: () => Worker) {}

  /** Whether a job is running or waiting. */
  get busy(): boolean { return this.running !== null || this.queue.length > 0; }

  run(channel: string, request: JobRequest): Promise<JobResult | null> {
    const id = this.nextId++;
    this.newest.set(channel, id);
    return new Promise((resolve, reject) => {
      // A job waiting on this channel is superseded before it ever starts.
      this.queue = this.queue.filter((p) => { if (p.channel !== channel) return true; p.resolve(null); return false; });
      this.queue.push({ id, channel, request, resolve, reject });
      void this.pump();
    });
  }

  /** Drops the channel's waiting job and throws away its running one's result. */
  cancel(channel: string): void {
    this.newest.set(channel, -1);
    this.queue = this.queue.filter((p) => { if (p.channel !== channel) return true; p.resolve(null); return false; });
  }

  private start(): Promise<void> {
    const worker = this.spawn();
    this.spawned++;
    this.worker = worker;
    this.ready = new Promise((resolve) => {
      worker.onmessage = (e: MessageEvent<FromWorker>) => {
        if (e.data.type === "ready") { resolve(); return; }
        this.finish(e.data);
      };
    });
    // A worker that dies (out of memory, say) fails its job and is replaced for the next one.
    worker.onerror = (e: ErrorEvent) => {
      const job = this.running;
      this.running = null;
      worker.terminate();
      if (this.worker === worker) { this.worker = null; this.ready = null; }
      job?.reject(new Error(e.message || "The job worker stopped."));
      void this.pump();
    };
    const init: ToWorker = { type: "init", module: this.module };
    worker.postMessage(init);
    return this.ready;
  }

  private async pump(): Promise<void> {
    if (this.running || this.queue.length === 0) return;
    const job = this.queue.shift()!;
    // Superseded while it waited behind another job.
    if (this.newest.get(job.channel) !== job.id) { job.resolve(null); void this.pump(); return; }
    this.running = job;
    await (this.worker && this.ready ? this.ready : this.start());
    const message: ToWorker = { type: "job", id: job.id, request: job.request };
    this.worker!.postMessage(message, transferables(job.request));
  }

  private finish(message: Exclude<FromWorker, { type: "ready" }>): void {
    const job = this.running;
    if (!job || job.id !== message.id) return;
    this.running = null;
    // A wasm trap poisons the whole instance (see `FromWorker`'s doc on `fatal`), so the worker is
    // replaced outright, the same as one that grew past the memory limit; a clean JsError refusal
    // keeps the worker, as before.
    const fatal = message.type === "failed" && message.fatal;
    if (fatal || message.memory > WORKER_MEMORY_LIMIT) { this.worker?.terminate(); this.worker = null; this.ready = null; }
    if (message.type === "failed") job.reject(new Error(fatal ? "The edit ran out of memory and could not finish. Try again, or on a smaller selection." : message.error));
    else job.resolve(this.newest.get(job.channel) === job.id ? message.result : null);
    void this.pump();
  }

  dispose(): void {
    this.worker?.terminate();
    this.worker = null;
    for (const p of this.queue) p.resolve(null);
    this.queue = [];
    this.running?.resolve(null);
    this.running = null;
  }
}
