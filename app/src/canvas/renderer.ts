import type { DocumentState, PreviewEdit } from "../engine/types";
import type { EngineClient } from "../engine/client";
import type { Viewport } from "./viewport";
import { GlRenderer } from "./gl-renderer";
import { CpuRenderer } from "./cpu-renderer";

export interface RenderOptions { checkerboard: boolean; }
export interface Renderer {
  readonly kind: "gl" | "cpu";
  /** Uploads whatever it needs and draws. There is no separate sync step: the GL renderer's
   * texture upload depends on the render plan and the zoom, so it happens inside `render`. */
  render(engine: EngineClient, state: DocumentState, viewport: Viewport, dpr: number, options: RenderOptions, edit: PreviewEdit | null): void;
  /** RGBA, top-down, the whole canvas element. */
  readPixels(): Uint8Array;
  dispose(): void;
}

export function createRenderer(canvas: HTMLCanvasElement): Renderer {
  const gl = canvas.getContext("webgl2", { premultipliedAlpha: true, preserveDrawingBuffer: true, antialias: false });
  if (gl) return new GlRenderer(canvas, gl);
  return new CpuRenderer(canvas);
}

/**
 * Layers to draw bottom to top: visible with visible ancestors, groups excluded, in hierarchy
 * order. The array order is not the z-order - a folder's children can sit anywhere in the array
 * and still draw at the folder's place - so this walks the tree, exactly as the engine's
 * `render_ids` and macOS's `renderLayers` do.
 */
export function renderOrder(state: DocumentState): DocumentState["layers"] {
  const children = new Map<string | null, DocumentState["layers"]>();
  for (const l of state.layers) { const list = children.get(l.parentId) ?? []; list.push(l); children.set(l.parentId, list); }
  const out: DocumentState["layers"] = [];
  const visit = (parent: string | null, depth: number, visible: boolean) => {
    if (depth > 64) return;
    for (const layer of children.get(parent) ?? []) {
      const effective = visible && layer.visible;
      if (layer.isGroup) visit(layer.id, depth + 1, effective);
      else if (effective) out.push(layer);
    }
  };
  visit(null, 0, true);
  return out;
}
