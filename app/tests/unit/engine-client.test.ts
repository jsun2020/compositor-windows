import { describe, expect, it, vi } from "vitest";
import { EngineClient } from "../../src/engine/client";
import type { PreviewEdit } from "../../src/engine/types";

/**
 * A wasm stub whose pointer calls grow linear memory, the way marshalling a JS string through
 * `__wbindgen_malloc` can. Growing detaches every ArrayBuffer taken from `memory.buffer`
 * beforehand, so a client that read the buffer before calling the pointer function would build
 * its view on a detached buffer and throw.
 */
function growingClient(): EngineClient {
  const memory = new WebAssembly.Memory({ initial: 1, maximum: 8 });
  const wasm = {
    layer_pixels_len: () => 4,
    layer_pixels_ptr: () => { memory.grow(1); return 0; },
    mask_pixels_len: () => 4,
    mask_pixels_ptr: () => { memory.grow(1); return 0; },
  };
  // The constructor is private by TypeScript alone; the fields are ordinary properties.
  const client = Object.create(EngineClient.prototype) as Record<string, unknown>;
  client.wasm = wasm;
  client.memory = memory;
  return client as unknown as EngineClient;
}

describe("loading the engine", () => {
  it("says the engine could not be loaded when its file is missing, rather than failing to compile an error page (final review minor 7)", async () => {
    let compiled = 0;
    vi.stubGlobal("fetch", async () => ({ ok: false, status: 404, statusText: "Not Found", arrayBuffer: async () => new ArrayBuffer(8) }));
    const compile = WebAssembly.compile;
    WebAssembly.compile = (async () => { compiled++; throw new Error("CompileError: not wasm"); }) as typeof WebAssembly.compile;
    try {
      await expect(EngineClient.load()).rejects.toThrow("The engine could not be loaded (404 Not Found).");
      expect(compiled, "nothing compiled").toBe(0);
    } finally { WebAssembly.compile = compile; vi.unstubAllGlobals(); }
  });
});

describe("a slow staged allocation leaves time for the next UI frame", () => {
  function setup() {
    const clock = { value: 0 }, calls: string[] = [], frames: FrameRequestCallback[] = [];
    let bytes = new Uint8Array(0), offset = 0;
    vi.spyOn(performance, "now").mockImplementation(() => clock.value);
    vi.stubGlobal("requestAnimationFrame", (callback: FrameRequestCallback) => { frames.push(callback); return frames.length; });
    const client = Object.create(EngineClient.prototype) as Record<string, unknown>;
    client.installQueue = Promise.resolve();
    client.wasm = {
      begin_staged_install: (length: number) => { bytes = new Uint8Array(length); calls.push("begin"); clock.value += 40; },
      append_staged_install: (_plane: number, part: Uint8Array) => { bytes.set(part, offset); offset += part.length; calls.push("append"); clock.value++; },
      finish_staged_install: () => { calls.push("finish"); return "{}"; },
      cancel_staged_install: () => { calls.push("cancel"); },
    };
    const pixels = new ArrayBuffer(4 * 1024 * 1024 + 4);
    new Uint8Array(pixels)[0] = 41; new Uint8Array(pixels)[pixels.byteLength - 1] = 13;
    return { client: client as unknown as EngineClient, clock, calls, frames, pixels, bytes: () => bytes };
  }

  it("lets the UI run after a long reservation, then preserves every byte and counts only CPU work", async () => {
    const state = setup();
    try {
      const result = state.client.installJobAsync("D", "A", '{"stamp":{}}', "{}", state.pixels, null, null, true);
      await vi.waitFor(() => expect(state.frames).toHaveLength(1));
      expect(state.calls).toEqual(["begin"]);
      state.clock.value += 16; state.frames.shift()!(state.clock.value);
      await result;
      expect(state.calls).toEqual(["begin", "append", "append", "finish", "cancel"]);
      const expected = new Uint8Array(state.pixels), actual = state.bytes();
      expect(actual.byteLength).toBe(expected.byteLength);
      expect(actual.every((value, index) => value === expected[index]), "the complete staged plane is preserved").toBe(true);
      expect(state.client.lastInstallCpuMs).toBe(42);
    } finally { vi.restoreAllMocks(); vi.unstubAllGlobals(); }
  });

  it("accepts cancellation during that frame without copying or committing the reserved result", async () => {
    const state = setup(); let valid = true;
    try {
      const result = state.client.installJobAsync("D", "A", '{"stamp":{}}', "{}", state.pixels, null, null, true, () => valid);
      await vi.waitFor(() => expect(state.frames).toHaveLength(1));
      valid = false; state.clock.value += 16; state.frames.shift()!(state.clock.value);
      await expect(result).rejects.toThrow("The preview was cancelled");
      expect(state.calls).toEqual(["begin", "cancel"]);
    } finally { vi.restoreAllMocks(); vi.unstubAllGlobals(); }
  });
});

describe("lossless uniform job installation", () => {
  function setup() {
    const clock={value:0},frames:FrameRequestCallback[]=[],calls:string[]=[];
    let pixels=new Uint8Array(0),offset=0,display=new Uint8Array(0);
    vi.spyOn(performance,"now").mockImplementation(()=>clock.value);
    vi.stubGlobal("requestAnimationFrame",(f:FrameRequestCallback)=>{frames.push(f);return frames.length;});
    const client=Object.create(EngineClient.prototype) as Record<string,unknown>;
    client.installQueue=Promise.resolve();
    client.wasm={
      begin_staged_install:(p:number,_m:number,d:number)=>{pixels=new Uint8Array(p);display=new Uint8Array(d);calls.push("begin");clock.value+=40;},
      repeat_staged_pixels:(r:number,g:number,b:number,a:number,n:number)=>{for(let i=0;i<n;i+=4)pixels.set([r,g,b,a],offset+i);offset+=n;calls.push("repeat");clock.value+=5;},
      append_staged_install:(plane:number,bytes:Uint8Array)=>{expect(plane).toBe(2);display.set(bytes);calls.push("display");clock.value+=2;},
      finish_staged_install:()=>{calls.push("finish");clock.value+=3;return '{"canvas":true}';},
      cancel_staged_install:()=>calls.push("cancel"),
    };
    const output=JSON.stringify({pixels:[1024,1025],uniformPixels:[17,31,43,97]});
    return {client:client as unknown as EngineClient,clock,frames,calls,output,pixels:()=>pixels,display:()=>display};
  }
  async function framesUntilDone(state:ReturnType<typeof setup>,promise:Promise<unknown>) {
    let done=false;promise.finally(()=>{done=true;}).catch(()=>{});
    while(!done){await Promise.resolve();if(state.frames.length){state.clock.value+=16;state.frames.shift()!(state.clock.value);}}
    return promise;
  }
  it("expands a translucent colour exactly, retains the worker display and accounts for all CPU work",async()=>{
    const state=setup();
    try {
      const display=new Uint8Array([3,5,7,11]).buffer;
      const pending=state.client.installJobAsync("D","A",'{"stamp":{}}',state.output,null,null,display,true);
      await framesUntilDone(state,pending);
      expect(state.calls).toEqual(["begin","repeat","repeat","display","finish","cancel"]);
      expect(state.pixels().length).toBe(1024*1025*4);
      expect(state.pixels().every((v,i)=>v===[17,31,43,97][i%4])).toBe(true);
      expect(Array.from(state.display())).toEqual([3,5,7,11]);
      expect(state.client.lastInstallCpuMs).toBe(55);
    }finally{vi.restoreAllMocks();vi.unstubAllGlobals();}
  });
  it("cancellation before expansion releases the reservation without committing",async()=>{
    const state=setup();let valid=true;
    try {
      const pending=state.client.installJobAsync("D","A",'{"stamp":{}}',state.output,null,null,null,true,()=>valid);
      await vi.waitFor(()=>expect(state.frames).toHaveLength(1));valid=false;state.frames.shift()!(0);
      await expect(pending).rejects.toThrow("The preview was cancelled");
      expect(state.calls).toEqual(["begin","cancel"]);
    }finally{vi.restoreAllMocks();vi.unstubAllGlobals();}
  });
  it("rejects malformed or ambiguous encodings before reserving memory",async()=>{
    const state=setup();
    try {
      for(const encoded of [
        {pixels:[1024,1025],uniformPixels:[1,2,3]},
        {pixels:[1024,1025],uniformPixels:[1,2,3,256]},
        {pixels:[1024,1025],uniformPixels:[1,2,3,-1]},
        {pixels:[1024,1025],uniformPixels:[1,2,3,4.5]},
        {pixels:[10001,10000],uniformPixels:[1,2,3,4]},
        {pixels:[0,1],uniformPixels:[1,2,3,4]},
      ])await expect(state.client.installJobAsync("D","A",'{"stamp":{}}',JSON.stringify(encoded),null,null,null,true)).rejects.toThrow("Invalid uniform job pixels");
      await expect(state.client.installJobAsync("D","A",'{"stamp":{}}',state.output,new ArrayBuffer(4),null,null,true)).rejects.toThrow("Invalid uniform job pixels");
      expect(state.calls).toEqual([]);
    }finally{vi.restoreAllMocks();vi.unstubAllGlobals();}
  });
});

describe("pixel views survive a wasm memory growth", () => {
  it("maskPixels reads the buffer after the pointer call, not before", () => {
    const client = growingClient();
    const view = client.maskPixels("D", "A");
    expect(view).not.toBeNull();
    // A view on a detached buffer reports length 0 and reads undefined (indexing never throws, so
    // only these two can fail: final review minor 21).
    expect(view!.length).toBe(4);
    expect(view![0]).toBe(0);
  });

  it("layerPixels does the same", () => {
    const client = growingClient();
    const view = client.layerPixels("D", "A");
    expect(view).not.toBeNull();
    expect(view!.length).toBe(4);
  });

  it("drawPixels makes the raster once, with the pending edit, reads the one the engine kept, and releases it after the upload", () => {
    const memory = new WebAssembly.Memory({ initial: 1, maximum: 8 });
    const calls: unknown[][] = [];
    const wasm = {
      prepare_draw_pixels: (...args: unknown[]) => { calls.push(["prepare", ...args]); return 4; },
      draw_pixels_ptr: (...args: unknown[]) => { calls.push(["ptr", ...args]); memory.grow(1); return 0; },
      release_draw_pixels: (...args: unknown[]) => { calls.push(["release", ...args]); },
    };
    const client = Object.create(EngineClient.prototype) as Record<string, unknown>;
    client.wasm = wasm; client.memory = memory;
    const edit: PreviewEdit = { kind: "mask", id: "A", draft: { origin: [1, 2], size: [3, 4], rotation: 0, flipX: false, flipY: true, sampling: "Smooth" } };
    const length = (client as unknown as EngineClient).drawPixels("D", "A", 2, edit, (view) => {
      calls.push(["upload"]);
      // Read in the upload, where a detached view would already report length 0.
      return view!.length;
    });
    expect(length).toBe(4);
    expect(calls, "one computation; the pointer call takes no arguments; the release follows the upload")
      .toEqual([["prepare", "D", "A", 2, JSON.stringify(edit)], ["ptr"], ["upload"], ["release"]]);
  });

  it("drawPixels releases the raster even when the upload throws, and hands null for none", () => {
    const calls: string[] = [];
    const wasm = {
      prepare_draw_pixels: () => { calls.push("prepare"); return 0; },
      draw_pixels_ptr: () => { throw new Error("must not be called"); },
      release_draw_pixels: () => { calls.push("release"); },
    };
    const client = Object.create(EngineClient.prototype) as Record<string, unknown>;
    client.wasm = wasm; client.memory = new WebAssembly.Memory({ initial: 1 });
    const c = client as unknown as EngineClient;
    expect(c.drawPixels("D", "A", 0, null, (view) => view)).toBeNull();
    expect(() => c.drawPixels("D", "A", 0, null, () => { throw new Error("upload failed"); })).toThrow("upload failed");
    expect(calls).toEqual(["prepare", "release", "prepare", "release"]);
  });

  it("selectionOutline hands on the engine's flat outline as it is, unpacked into nothing", () => {
    // Final review F3: a four-million-point outline as tuple arrays cost more than the engine's
    // work. The very Float64Array comes back (canvas/ants.ts `traceOutline` reads it).
    const flat = new Float64Array([2, 3, 0, 0, 10, 0, 0, 5.5, 2, 7, 8, 9, 10]);
    const client = Object.create(EngineClient.prototype) as Record<string, unknown>;
    client.wasm = { selection_outline: (doc: string, step: number) => { expect([doc, step]).toEqual(["D", 0.25]); return flat; } };
    const c = client as unknown as EngineClient;
    expect(c.selectionOutline("D", 0.25)).toBe(flat);
  });

  it("jobInput copies the kept buffers out after each pointer call, into buffers of their own, then releases them", () => {
    const memory = new WebAssembly.Memory({ initial: 1, maximum: 8 });
    new Uint8Array(memory.buffer).set([1, 2, 3, 4, 9, 8], 0);
    const calls: string[] = [];
    const wasm = {
      prepare_job: (doc: string, layer: string) => { calls.push(`prepare ${doc} ${layer}`); return '{"stamp":{}}'; },
      job_buffer_len: (mask: boolean) => (mask ? 2 : 4),
      // Marshalling can grow memory, detaching any buffer read before the pointer call.
      job_buffer_ptr: (mask: boolean) => { calls.push(`ptr ${mask}`); memory.grow(1); return mask ? 4 : 0; },
      release_job: () => { calls.push("release"); },
    };
    const client = Object.create(EngineClient.prototype) as Record<string, unknown>;
    client.wasm = wasm; client.memory = memory;
    const copy = (client as unknown as EngineClient).jobInput("D", "A");
    expect(Array.from(new Uint8Array(copy.pixels!))).toEqual([1, 2, 3, 4]);
    expect(Array.from(new Uint8Array(copy.mask!))).toEqual([9, 8]);
    expect(copy.pixels!.byteLength, "a buffer of its own, transferable").toBe(4);
    // No "selection" key in the fake input, so no points buffer, and no job_points_* call (the stub
    // has none -- calling one would throw).
    expect(copy.points).toBeNull();
    expect(calls).toEqual(["prepare D A", "ptr false", "ptr true", "release"]);
  });

  it("jobInput copies an empty selection's points as a zero-length buffer, never null", () => {
    const memory = new WebAssembly.Memory({ initial: 1, maximum: 8 });
    const calls: string[] = [];
    const wasm = {
      prepare_job: () => { calls.push("prepare"); return '{"selection":{"contourLengths":[],"antialiased":true,"feather":0}}'; },
      job_buffer_len: () => 0,
      job_buffer_ptr: () => { throw new Error("must not be called"); },
      job_points_len: () => { calls.push("points_len"); return 0; },
      job_points_ptr: () => { calls.push("points_ptr"); return 0; },
      release_job: () => { calls.push("release"); },
    };
    const client = Object.create(EngineClient.prototype) as Record<string, unknown>;
    client.wasm = wasm; client.memory = memory;
    const copy = (client as unknown as EngineClient).jobInput("D", "A");
    expect(copy.pixels).toBeNull();
    expect(copy.points, "an explicit empty selection is Some with zero contours, not null").not.toBeNull();
    expect(copy.points!.byteLength).toBe(0);
    expect(calls).toEqual(["prepare", "points_len", "points_ptr", "release"]);
  });

  it("both return null rather than a zero-length view when there is nothing to read", () => {
    const memory = new WebAssembly.Memory({ initial: 1 });
    const wasm = { layer_pixels_len: () => 0, mask_pixels_len: () => 0, layer_pixels_ptr: () => { throw new Error("must not be called"); }, mask_pixels_ptr: () => { throw new Error("must not be called"); } };
    const client = Object.create(EngineClient.prototype) as Record<string, unknown>;
    client.wasm = wasm; client.memory = memory;
    const c = client as unknown as EngineClient;
    expect(c.maskPixels("D", "A")).toBeNull();
    expect(c.layerPixels("D", "A")).toBeNull();
  });
});
