import type { DocumentState } from "../engine/types";
import type { Viewport, SizeLike, PointLike } from "../canvas/viewport";
import { cropDrag, CropSnap, HANDLES, isValid, snapped, type CropDrag, type CropDragMode, type Rect } from "./crop-geometry";
import type { CropRatio } from "../state/store";

export const HANDLE_HIT_PX = 6;
export const SNAP_SCREEN_PX = 10;

export function hitTest(rect: Rect, view: PointLike, viewport: Viewport, size: SizeLike): CropDragMode {
  const tl = viewport.viewPoint({ x: rect.x, y: rect.y }, size), br = viewport.viewPoint({ x: rect.x + rect.width, y: rect.y + rect.height }, size);
  for (let index = 0; index < HANDLES.length; index++) {
    const h = HANDLES[index]; const hx = tl.x + (br.x - tl.x) * h.x, hy = tl.y + (br.y - tl.y) * h.y;
    if (Math.abs(view.x - hx) <= HANDLE_HIT_PX && Math.abs(view.y - hy) <= HANDLE_HIT_PX) return { kind: "resize", index };
  }
  if (view.x >= tl.x && view.x <= br.x && view.y >= tl.y && view.y <= br.y) return { kind: "move" };
  return { kind: "create" };
}

/** Canvas edges and every visible pixel layer's upright bounds, in whole document pixels. */
export function snapTargets(state: DocumentState): { xs: number[]; ys: number[] } {
  const xs = [0, state.width], ys = [0, state.height];
  const byId = new Map(state.layers.map((l) => [l.id, l]));
  for (const layer of state.layers) {
    if (layer.isGroup || layer.pixelsWidth === 0) continue;
    let node: typeof layer | undefined = layer; let visible = true; let steps = 0;
    while (node && steps++ < 65) { if (!node.visible) { visible = false; break; } node = node.parentId ? byId.get(node.parentId) : undefined; }
    if (!visible) continue;
    const [ox, oy] = layer.transform.origin, [w, h] = layer.transform.size;
    const cx = ox + w / 2, cy = oy + h / 2, rad = (layer.transform.rotation % 360) * Math.PI / 180;
    const cos = Math.cos(rad), sin = Math.sin(rad);
    const corners = [[-w / 2, -h / 2], [w / 2, -h / 2], [w / 2, h / 2], [-w / 2, h / 2]].map(([x, y]) => ({ x: cx + x * cos - y * sin, y: cy + x * sin + y * cos }));
    xs.push(Math.round(Math.min(...corners.map((c) => c.x))), Math.round(Math.max(...corners.map((c) => c.x))));
    ys.push(Math.round(Math.min(...corners.map((c) => c.y))), Math.round(Math.max(...corners.map((c) => c.y))));
  }
  return { xs, ys };
}

/** Where the crop starts when the tool is chosen (`selectTool`, EditorSession.swift:349-358): the
 * selection's bounds rounded out and cut to the canvas when there is a selection with something in
 * it, as Photoshop's C then Enter crops to it; else, or when that is not a valid crop, the canvas. */
export function cropSeed(state: DocumentState): Rect {
  const canvas = { x: 0, y: 0, width: state.width, height: state.height };
  const b = state.selection && !state.selection.empty ? state.selection.bounds : null;
  if (!b) return canvas;
  const x0 = Math.max(0, Math.floor(b.x)), y0 = Math.max(0, Math.floor(b.y));
  const x1 = Math.min(state.width, Math.ceil(b.x + b.width)), y1 = Math.min(state.height, Math.ceil(b.y + b.height));
  const rect = { x: x0, y: y0, width: x1 - x0, height: y1 - y0 };
  return isValid(rect) ? rect : canvas;
}

export function ratioValue(choice: CropRatio, state: DocumentState): number | null {
  switch (choice) { case "Original": return state.width / state.height; case "1:1": return 1; case "4:3": return 4 / 3; case "16:9": return 16 / 9; default: return null; }
}

/** Keeps the width and the center, like changeCropRatio on macOS. */
export function applyRatio(rect: Rect, ratio: number): Rect {
  const height = rect.width / ratio;
  const next = snapped({ x: rect.x, y: rect.y + rect.height / 2 - height / 2, width: rect.width, height });
  return isValid(next) ? next : rect;
}

export class CropSession {
  private drag: CropDrag;
  private snap: CropSnap;
  constructor(mode: CropDragMode, startDoc: PointLike, original: Rect, state: DocumentState, private readonly ratio: number | null, tolerance: number) {
    this.drag = cropDrag(mode, startDoc, original);
    const t = snapTargets(state);
    this.snap = new CropSnap(t.xs, t.ys, tolerance);
  }
  update(pointDoc: PointLike, symmetric: boolean): { rect: Rect; guides: { xs: number[]; ys: number[] } } {
    const raw = this.drag.updated(pointDoc, this.ratio, symmetric);
    const rect = this.snap.apply(raw, this.drag, pointDoc, this.ratio, symmetric);
    const guides = { xs: [] as number[], ys: [] as number[] };
    if (rect.x !== raw.x) guides.xs.push(rect.x);
    if (rect.x + rect.width !== raw.x + raw.width) guides.xs.push(rect.x + rect.width);
    if (rect.y !== raw.y) guides.ys.push(rect.y);
    if (rect.y + rect.height !== raw.y + raw.height) guides.ys.push(rect.y + rect.height);
    return { rect: isValid(rect) ? rect : raw, guides };
  }
}
