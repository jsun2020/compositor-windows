// The job worker (Phase 4b-1): a second instance of the engine's wasm module, compiled once by the
// main thread and posted here, running one job at a time on transferred buffers (engine `jobs.rs`).
import { initSync, WasmEngine } from "./pkg/compositor_engine.js";
import type { FromWorker, JobRequest, JobResult, ToWorker } from "./jobs";
import { renderText } from "../tools/text-raster";
import type { TextStyle } from "../tools/text-style";

let engine: WasmEngine | null = null;
let memory: WebAssembly.Memory | null = null;

/** A copy of one of the engine's kept job buffers, out of wasm memory, ready to transfer. */
function kept(mask: boolean, reuse: ArrayBuffer | null = null): ArrayBuffer | null {
  const len = engine!.job_buffer_len(mask);
  if (len === 0) return null;
  const ptr = engine!.job_buffer_ptr(mask);
  // The request's transferred buffer is no longer needed once WASM finishes.
  // Reuse it when the result has the same size instead of allocating a second
  // full layer (400 MB at 100 MP) while the input is still awaiting collection.
  const out = reuse?.byteLength === len ? new Uint8Array(reuse) : new Uint8Array(len);
  out.set(new Uint8Array(memory!.buffer, ptr, len));
  return out.buffer;
}
const bytes = (b: ArrayBuffer | null) => (b ? new Uint8Array(b) : undefined);
/** A copy of an edit's result halved to the canvas's level (the fourth job buffer, F1), or null. */
function keptDisplay(): ArrayBuffer | null {
  const len = engine!.job_display_len();
  if (len === 0) return null;
  const ptr = engine!.job_display_ptr();
  return new Uint8Array(memory!.buffer, ptr, len).slice().buffer;
}

function run(request: JobRequest): JobResult {
  switch (request.kind) {
    case "text": {const {width,height,pixels}=renderText(JSON.parse(request.input) as TextStyle);return{header:JSON.stringify({width,height}),pixels,mask:null};}
    case "documentEdit": {
      const header=engine!.run_document_edit_job(request.input,request.layers.map(l=>bytes(l.pixels)??null),request.layers.map(l=>bytes(l.mask)??null),bytes(request.points),request.layer,request.command,request.outPerDoc);
      try{return{header,pixels:kept(false),mask:kept(true),display:keptDisplay()};}finally{engine!.release_job();}
    }
    case "clipboard": {
      const png = engine!.run_clipboard_job(request.input, request.layers.map((l) => bytes(l.pixels) ?? null), request.layers.map((l) => bytes(l.mask) ?? null), bytes(request.points), request.png);
      const h = JSON.parse(request.input) as { region: { x: number; y: number; width: number; height: number } };
      return { header: JSON.stringify(h.region), pixels: png.buffer as ArrayBuffer, mask: null };
    }
    case "decodeClipboard": {
      const header = engine!.decode_clipboard(new Uint8Array(request.pixels));
      try { return { header, pixels: kept(false), mask: null }; } finally { engine!.release_job(); }
    }
    case "edit": {
      const header = engine!.run_edit_job(request.input, bytes(request.pixels), bytes(request.mask), bytes(request.points), request.command, request.outPerDoc);
      const result = { header, pixels: kept(false, request.pixels ?? request.outputPixels ?? null), mask: kept(true, request.mask), display: keptDisplay() };
      engine!.release_job();
      return result;
    }
    case "histogram":
      return { header: engine!.run_histogram_job(request.input, bytes(request.pixels), bytes(request.mask), bytes(request.points)), pixels: null, mask: null };
    case "effects": {
      const header = engine!.run_effects_job(request.input, new Uint8Array(request.pixels), bytes(request.mask), request.factor, request.edit ?? undefined) ?? null;
      const result = { header, pixels: header ? kept(false) : null, mask: null };
      engine!.release_job();
      return result;
    }
  }
}

self.onmessage = (e: MessageEvent<ToWorker>) => {
  const message = e.data;
  const post = (reply: FromWorker, transfer: ArrayBuffer[] = []) => (self as unknown as Worker).postMessage(reply, transfer);
  if (message.type === "init") {
    memory = initSync({ module: message.module }).memory;
    engine = new WasmEngine();
    post({ type: "ready" });
    return;
  }
  try {
    const result = run(message.request);
    // Return inputs that were not reused for the result, including no-op edits.
    // They are already copied into WASM and no longer borrowed by the kernel.
    const recycled=message.request.kind==="histogram"||message.request.kind==="edit"?[message.request.pixels,message.request.mask,message.request.points,message.request.kind==="edit"?message.request.outputPixels??null:null].filter((b):b is ArrayBuffer=>b!==null&&b.byteLength>=4*1024*1024&&b!==result.pixels&&b!==result.mask&&b!==result.display):[];
    post({ type: "done", id: message.id, result, memory: memory!.buffer.byteLength, recycled }, [...[result.pixels, result.mask, result.display ?? null].filter((b): b is ArrayBuffer => b !== null),...recycled]);
  } catch (err) {
    // A wasm trap (an `unreachable` panic, an allocation abort) throws a `WebAssembly.RuntimeError`,
    // not the ordinary `JsError` a refused command throws: it leaves this instance unusable (jobs.ts's
    // `FromWorker` doc), so the client is told to replace the worker rather than just fail this job.
    const fatal = err instanceof WebAssembly.RuntimeError;
    post({ type: "failed", id: message.id, error: String(err instanceof Error ? err.message : err), memory: memory!.buffer.byteLength, fatal });
  }
};
