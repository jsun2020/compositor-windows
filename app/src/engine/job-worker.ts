// The job worker (Phase 4b-1): a second instance of the engine's wasm module, compiled once by the
// main thread and posted here, running one job at a time on transferred buffers (engine `jobs.rs`).
import { initSync, WasmEngine } from "./pkg/compositor_engine.js";
import type { FromWorker, JobRequest, JobResult, ToWorker } from "./jobs";

let engine: WasmEngine | null = null;
let memory: WebAssembly.Memory | null = null;

/** A copy of one of the engine's kept job buffers, out of wasm memory, ready to transfer. */
function kept(mask: boolean): ArrayBuffer | null {
  const len = engine!.job_buffer_len(mask);
  if (len === 0) return null;
  const ptr = engine!.job_buffer_ptr(mask);
  return new Uint8Array(memory!.buffer, ptr, len).slice().buffer;
}
const bytes = (b: ArrayBuffer | null) => (b ? new Uint8Array(b) : undefined);

function run(request: JobRequest): JobResult {
  switch (request.kind) {
    case "edit": {
      const header = engine!.run_edit_job(request.input, bytes(request.pixels), bytes(request.mask), bytes(request.points), request.command);
      const result = { header, pixels: kept(false), mask: kept(true) };
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
    post({ type: "done", id: message.id, result, memory: memory!.buffer.byteLength }, [result.pixels, result.mask].filter((b): b is ArrayBuffer => b !== null));
  } catch (err) {
    post({ type: "failed", id: message.id, error: String(err instanceof Error ? err.message : err), memory: memory!.buffer.byteLength });
  }
};
