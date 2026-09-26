import { describe, expect, it } from "vitest";
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

describe("pixel views survive a wasm memory growth", () => {
  it("maskPixels reads the buffer after the pointer call, not before", () => {
    const client = growingClient();
    const view = client.maskPixels("D", "A");
    expect(view).not.toBeNull();
    expect(view!.length).toBe(4);
    // A view on a detached buffer reports length 0 and cannot be read.
    expect(() => view![0]).not.toThrow();
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
      expect(() => view![0]).not.toThrow();
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
