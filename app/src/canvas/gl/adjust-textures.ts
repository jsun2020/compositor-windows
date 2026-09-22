import type { EngineClient } from "../../engine/client";
import type { LayerAdjustment } from "../../engine/types";

interface Entry { lut: WebGLTexture | null; response: WebGLTexture | null; }

/** LUT and hue-response textures, keyed by the adjustment's own JSON: the same settings reuse
 * the same textures frame after frame, and changed settings make new ones. */
export class AdjustTextures {
  private entries = new Map<string, Entry>();
  constructor(private readonly gl: WebGL2RenderingContext) {}

  static key(adjustment: LayerAdjustment): string { return JSON.stringify(adjustment); }

  private entry(engine: EngineClient, adjustment: LayerAdjustment): Entry {
    const key = AdjustTextures.key(adjustment);
    const existing = this.entries.get(key);
    if (existing) return existing;
    const gl = this.gl;
    let lut: WebGLTexture | null = null;
    const bytes = engine.adjustmentLut(adjustment);
    if (bytes.length === 1024) {
      lut = gl.createTexture()!;
      gl.bindTexture(gl.TEXTURE_2D, lut);
      gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, 256, 1, 0, gl.RGBA, gl.UNSIGNED_BYTE, bytes);
      gl.pixelStorei(gl.UNPACK_ALIGNMENT, 4);
      // LINEAR, so a value between two entries interpolates exactly as the CPU's table does.
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    }
    let response: WebGLTexture | null = null;
    const table = engine.hueResponse(adjustment);
    if (table.length === 361 * 4) {
      response = gl.createTexture()!;
      gl.bindTexture(gl.TEXTURE_2D, response);
      // Sampled with texelFetch at whole degrees, so no filtering is needed (and RGBA16F is not
      // filterable in core WebGL2 anyway).
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA16F, 361, 1, 0, gl.RGBA, gl.FLOAT, table);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    }
    const made = { lut, response };
    this.entries.set(key, made);
    return made;
  }
  lut(engine: EngineClient, adjustment: LayerAdjustment): WebGLTexture | null { return this.entry(engine, adjustment).lut; }
  response(engine: EngineClient, adjustment: LayerAdjustment): WebGLTexture | null { return this.entry(engine, adjustment).response; }
  /** Drops every entry whose settings are no longer in the plan. */
  retain(keys: Set<string>): void {
    for (const [key, entry] of [...this.entries]) {
      if (keys.has(key)) continue;
      if (entry.lut) this.gl.deleteTexture(entry.lut);
      if (entry.response) this.gl.deleteTexture(entry.response);
      this.entries.delete(key);
    }
  }
  dispose(): void { this.retain(new Set()); }
}
