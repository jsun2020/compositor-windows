import type { DocumentState } from "../engine/types";
import type { EngineClient } from "../engine/client";
import type { Viewport } from "./viewport";
import { GlRenderer } from "./gl-renderer";
import { CpuRenderer } from "./cpu-renderer";

export interface RenderOptions { checkerboard: boolean; }
export interface Renderer {
  readonly kind: "gl" | "cpu";
  sync(engine: EngineClient, state: DocumentState): void;
  render(state: DocumentState, viewport: Viewport, dpr: number, options: RenderOptions): void;
  /** RGBA, top-down, the whole canvas element. */
  readPixels(): Uint8Array;
  dispose(): void;
}

export function createRenderer(canvas: HTMLCanvasElement, engine: EngineClient): Renderer {
  const gl = canvas.getContext("webgl2", { premultipliedAlpha: true, preserveDrawingBuffer: true, antialias: false });
  if (gl) return new GlRenderer(canvas, gl);
  return new CpuRenderer(canvas, engine);
}

/** Layers to draw bottom to top: visible with visible ancestors, groups excluded. */
export function renderOrder(state: DocumentState): DocumentState["layers"] {
  const byId = new Map(state.layers.map((l) => [l.id, l]));
  return state.layers.filter((layer) => {
    if (layer.isGroup) return false;
    let node: typeof layer | undefined = layer;
    let steps = 0;
    while (node) { if (!node.visible || steps++ > 64) return false; node = node.parentId ? byId.get(node.parentId) : undefined; }
    return true;
  });
}
