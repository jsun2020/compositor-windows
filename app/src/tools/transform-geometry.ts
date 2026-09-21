import type { Corners, DocumentState, LayerTransform, PointTuple } from "../engine/types";
import type { PointLike, RectLike, SizeLike, Viewport } from "../canvas/viewport";
import { HANDLES } from "./crop-geometry";

export type P = PointLike;
export type Mat3 = number[]; // row-major 3x3

export function center(t: LayerTransform): P { return { x: t.origin[0] + t.size[0] / 2, y: t.origin[1] + t.size[1] / 2 }; }
export function radians(t: LayerTransform): number { return (t.rotation % 360) * Math.PI / 180; }
export function pointOf(t: LayerTransform, unit: P): P {
  const x = (unit.x - 0.5) * t.size[0], y = (unit.y - 0.5) * t.size[1]; const r = radians(t); const c = center(t);
  return { x: c.x + x * Math.cos(r) - y * Math.sin(r), y: c.y + x * Math.sin(r) + y * Math.cos(r) };
}
export function cornersOf(t: LayerTransform): [P, P, P, P] { return [pointOf(t, { x: 0, y: 0 }), pointOf(t, { x: 1, y: 0 }), pointOf(t, { x: 1, y: 1 }), pointOf(t, { x: 0, y: 1 })]; }
export function boundsOf(t: LayerTransform): RectLike { return boundsOfPoints(cornersOf(t)); }
export function boundsOfPoints(pts: P[]): RectLike {
  const xs = pts.map((p) => p.x), ys = pts.map((p) => p.y);
  const x = Math.min(...xs), y = Math.min(...ys);
  return { x, y, width: Math.max(...xs) - x, height: Math.max(...ys) - y };
}
export function containsPoint(t: LayerTransform, p: P): boolean {
  const c = center(t); const r = radians(t); const x = p.x - c.x, y = p.y - c.y;
  return Math.abs(x * Math.cos(r) + y * Math.sin(r)) <= t.size[0] / 2 && Math.abs(-x * Math.sin(r) + y * Math.cos(r)) <= t.size[1] / 2;
}
export function isValidTransform(t: LayerTransform): boolean {
  const vals = [t.origin[0], t.origin[1], t.size[0], t.size[1], t.rotation];
  return vals.every(Number.isFinite) && t.size[0] >= 1 && t.size[0] <= 300_000 && t.size[1] >= 1 && t.size[1] <= 300_000 && Math.abs(t.origin[0]) <= 1_000_000 && Math.abs(t.origin[1]) <= 1_000_000;
}
export function roundedTransform(t: LayerTransform): LayerTransform {
  return { ...t, origin: [Math.round(t.origin[0]), Math.round(t.origin[1])], size: [Math.max(1, Math.round(t.size[0])), Math.max(1, Math.round(t.size[1]))], rotation: Math.round(t.rotation) };
}
export function scalePercent(t: LayerTransform, pixel: SizeLike): number { return t.size[0] / Math.max(1, pixel.width) * 100; }
export function scaledToPercent(t: LayerTransform, percent: number, pixel: SizeLike): LayerTransform {
  const c = center(t); const w = pixel.width * percent / 100, h = pixel.height * percent / 100;
  return { ...t, size: [w, h], origin: [c.x - w / 2, c.y - h / 2] };
}
export function mirrored(t: LayerTransform, horizontal: boolean, axis: number): LayerTransform {
  const c = center(t);
  return horizontal ? { ...t, flipX: !t.flipX, origin: [2 * axis - c.x - t.size[0] / 2, t.origin[1]], rotation: -t.rotation }
    : { ...t, flipY: !t.flipY, origin: [t.origin[0], 2 * axis - c.y - t.size[1] / 2], rotation: -t.rotation };
}

export function mat3Mul(a: Mat3, b: Mat3): Mat3 {
  const o = new Array(9).fill(0);
  for (let r = 0; r < 3; r++) for (let c = 0; c < 3; c++) o[r * 3 + c] = a[r * 3] * b[c] + a[r * 3 + 1] * b[3 + c] + a[r * 3 + 2] * b[6 + c];
  return o;
}
export function mat3Apply(m: Mat3, p: P): P {
  const w = m[6] * p.x + m[7] * p.y + m[8]; const d = Math.abs(w) < 1e-12 ? 1e-12 : w;
  return { x: (m[0] * p.x + m[1] * p.y + m[2]) / d, y: (m[3] * p.x + m[4] * p.y + m[5]) / d };
}
export function mat3Invert(m: Mat3): Mat3 | null {
  const [a, b, c, d, e, f, g, h, i] = m;
  const det = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
  if (!Number.isFinite(det) || Math.abs(det) < 1e-12) return null;
  return [(e * i - f * h) / det, (c * h - b * i) / det, (b * f - c * e) / det, (f * g - d * i) / det, (a * i - c * g) / det, (c * d - a * f) / det, (d * h - e * g) / det, (b * g - a * h) / det, (a * e - b * d) / det];
}
const translation = (x: number, y: number): Mat3 => [1, 0, x, 0, 1, y, 0, 0, 1];
const scaling = (x: number, y: number): Mat3 => [x, 0, 0, 0, y, 0, 0, 0, 1];
const rotation = (r: number): Mat3 => [Math.cos(r), -Math.sin(r), 0, Math.sin(r), Math.cos(r), 0, 0, 0, 1];
/** Layer pixel (x, y) in a w x h grid to document coordinates (flips applied). */
export function pixelToDocument(t: LayerTransform, w: number, h: number): Mat3 {
  const c = center(t);
  return mat3Mul(mat3Mul(mat3Mul(translation(c.x, c.y), rotation(radians(t))), scaling(t.size[0] / Math.max(1, w) * (t.flipX ? -1 : 1), t.size[1] / Math.max(1, h) * (t.flipY ? -1 : 1))), translation(-w / 2, -h / 2));
}
/** Unit square (TL, TR, BR, BL) onto `c`. */
export function homographyUnitTo(c: P[]): Mat3 {
  const sx = c[0].x - c[1].x + c[2].x - c[3].x, sy = c[0].y - c[1].y + c[2].y - c[3].y;
  let g = 0, h = 0;
  if (Math.abs(sx) > 1e-9 || Math.abs(sy) > 1e-9) {
    const dx1 = c[1].x - c[2].x, dx2 = c[3].x - c[2].x, dy1 = c[1].y - c[2].y, dy2 = c[3].y - c[2].y;
    const den = dx1 * dy2 - dx2 * dy1;
    if (Math.abs(den) > 1e-12) { g = (sx * dy2 - dx2 * sy) / den; h = (dx1 * sy - sx * dy1) / den; }
  }
  const a = c[1].x - c[0].x + g * c[1].x, b = c[3].x - c[0].x + h * c[3].x;
  const d = c[1].y - c[0].y + g * c[1].y, e = c[3].y - c[0].y + h * c[3].y;
  return [a, b, c[0].x, d, e, c[0].y, g, h, 1];
}
export function isUsableCorners(c: P[]): boolean {
  if (c.length !== 4 || !c.every((p) => Number.isFinite(p.x) && Number.isFinite(p.y) && Math.abs(p.x) <= 1e6 && Math.abs(p.y) <= 1e6)) return false;
  let sign = 0;
  for (let i = 0; i < 4; i++) {
    const a = c[i], b = c[(i + 1) % 4], d = c[(i + 2) % 4];
    const cross = (b.x - a.x) * (d.y - b.y) - (b.y - a.y) * (d.x - b.x);
    if (Math.abs(cross) <= 0.01) return false;
    if (sign === 0) sign = cross < 0 ? -1 : 1; else if ((cross < 0) !== (sign < 0)) return false;
  }
  return true;
}
export function carriedCorners(placement: LayerTransform, by: LayerTransform, to: P[]): P[] {
  const c = center(by);
  const forward = mat3Mul(mat3Mul(mat3Mul(translation(c.x, c.y), rotation(radians(by))), scaling(by.size[0], by.size[1])), translation(-0.5, -0.5));
  const toUnit = mat3Invert(forward) ?? [1, 0, 0, 0, 1, 0, 0, 0, 1];
  const map = homographyUnitTo(to);
  return cornersOf(placement).map((p) => mat3Apply(map, mat3Apply(toUnit, p)));
}
export const toTuple = (p: P): PointTuple => [p.x, p.y];
export const fromTuple = (t: PointTuple): P => ({ x: t[0], y: t[1] });
export const cornersToTuples = (c: P[]): Corners => [toTuple(c[0]), toTuple(c[1]), toTuple(c[2]), toTuple(c[3])];

export type TransformDragMode = { kind: "move" } | { kind: "resize"; index: number } | { kind: "rotate" } | { kind: "distort"; index: number };
export interface DragOptions { lockRatio: boolean; shift: boolean; alt: boolean; }

/** Port of TransformDrag.updated. */
export function transformDrag(original: LayerTransform, start: P, mode: TransformDragMode) {
  return {
    updated(point: P, opts: DragOptions): LayerTransform {
      let result: LayerTransform = { ...original, origin: [...original.origin] as PointTuple, size: [...original.size] as PointTuple };
      switch (mode.kind) {
        case "distort": return original;
        case "move": {
          let dx = point.x - start.x, dy = point.y - start.y;
          if (opts.shift) { if (Math.abs(dx) >= Math.abs(dy)) dy = 0; else dx = 0; }
          result.origin = [original.origin[0] + dx, original.origin[1] + dy];
          break;
        }
        case "rotate": {
          const c = center(original);
          const delta = Math.atan2(point.y - c.y, point.x - c.x) - Math.atan2(start.y - c.y, start.x - c.x);
          let r = original.rotation + delta * 180 / Math.PI;
          if (opts.shift) r = Math.round(r / 15) * 15;
          result.rotation = r;
          break;
        }
        case "resize": {
          const handle = HANDLES[mode.index];
          const anchorUnit = opts.alt ? { x: 0.5, y: 0.5 } : { x: 1 - handle.x, y: 1 - handle.y };
          const anchor = pointOf(original, anchorUnit);
          const initial = pointOf(original, handle);
          const dx = initial.x + point.x - start.x - anchor.x, dy = initial.y + point.y - start.y - anchor.y;
          const span = opts.alt ? 2 : 1; const r = radians(original);
          const localX = (dx * Math.cos(r) + dy * Math.sin(r)) * span, localY = (-dx * Math.sin(r) + dy * Math.cos(r)) * span;
          const sx = handle.x * 2 - 1, sy = handle.y * 2 - 1;
          const [ow, oh] = original.size;
          let width = sx === 0 ? ow : Math.max(1, localX * sx), height = sy === 0 ? oh : Math.max(1, localY * sy);
          if (opts.lockRatio !== opts.shift) {
            let factor: number;
            if (sx === 0) factor = height / oh; else if (sy === 0) factor = width / ow;
            else factor = Math.max(1 / Math.min(ow, oh), (localX * sx * ow + localY * sy * oh) / (ow * ow + oh * oh));
            width = ow * factor; height = oh * factor;
          }
          const offX = (0.5 - anchorUnit.x) * width, offY = (0.5 - anchorUnit.y) * height;
          const cx = anchor.x + offX * Math.cos(r) - offY * Math.sin(r), cy = anchor.y + offX * Math.sin(r) + offY * Math.cos(r);
          result.size = [width, height]; result.origin = [cx - width / 2, cy - height / 2];
          break;
        }
      }
      return isValidTransform(result) ? result : original;
    },
  };
}

/** Moving distortion corners: a corner handle moves its corner, an edge handle both of that edge's corners, "move" all four. */
export function cornersDrag(original: P[], start: P, mode: "move" | number) {
  return {
    updated(point: P, shift: boolean): P[] {
      let dx = point.x - start.x, dy = point.y - start.y;
      if (shift) { if (Math.abs(dx) >= Math.abs(dy)) dy = 0; else dx = 0; }
      const moved = mode === "move" ? [0, 1, 2, 3] : mode % 2 === 0 ? [mode / 2] : [(mode - 1) / 2, ((mode - 1) / 2 + 1) % 4];
      return original.map((p, i) => (moved.includes(i) ? { x: p.x + dx, y: p.y + dy } : p));
    },
  };
}

export interface OverlayGeometry { handles: P[]; rotationHandle: P; showsRotation: boolean; }
export function overlayGeometry(shape: LayerTransform | P[], viewport: Viewport, docSize: SizeLike): OverlayGeometry {
  if (Array.isArray(shape)) {
    const v = shape.map((p) => viewport.viewPoint(p, docSize));
    const mid = (a: P, b: P) => ({ x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 });
    return { handles: [v[0], mid(v[0], v[1]), v[1], mid(v[1], v[2]), v[2], mid(v[2], v[3]), v[3], mid(v[3], v[0])], rotationHandle: mid(v[0], v[1]), showsRotation: false };
  }
  const handles = HANDLES.map((h) => viewport.viewPoint(pointOf(shape, h), docSize));
  const r = radians(shape);
  return { handles, rotationHandle: { x: handles[1].x + Math.sin(r) * 28, y: handles[1].y - Math.cos(r) * 28 }, showsRotation: true };
}
export function hitOverlay(g: OverlayGeometry, point: P): TransformDragMode | null {
  const near = (o: P) => Math.hypot(point.x - o.x, point.y - o.y) <= 10;
  if (g.showsRotation && near(g.rotationHandle)) return { kind: "rotate" };
  const index = g.handles.findIndex(near);
  if (index >= 0) return { kind: "resize", index };
  for (const [s, e, handle] of [[0, 2, 1], [2, 4, 3], [4, 6, 5], [6, 0, 7]] as const) {
    const a = g.handles[s], b = g.handles[e]; const dx = b.x - a.x, dy = b.y - a.y; const len2 = dx * dx + dy * dy;
    if (len2 <= 0) continue;
    const t = ((point.x - a.x) * dx + (point.y - a.y) * dy) / len2;
    if (t >= 0 && t <= 1 && Math.hypot(point.x - a.x - t * dx, point.y - a.y - t * dy) <= 10) return { kind: "resize", index: handle };
  }
  return null;
}

/** Canvas edges and centre plus every other visible pixel layer's rounded upright bounds and centre. */
export function snapTargets(state: DocumentState, movingIds: string[]): { xs: number[]; ys: number[] } {
  const xs = [0, state.width / 2, state.width], ys = [0, state.height / 2, state.height];
  const byId = new Map(state.layers.map((l) => [l.id, l]));
  for (const layer of state.layers) {
    if (layer.isGroup || !layer.hasPixels || movingIds.includes(layer.id)) continue;
    let node: typeof layer | undefined = layer; let visible = true; let steps = 0;
    while (node && steps++ < 65) { if (!node.visible) { visible = false; break; } node = node.parentId ? byId.get(node.parentId) : undefined; }
    if (!visible) continue;
    const b = boundsOf(layer.transform);
    xs.push(Math.round(b.x), Math.round(b.x + b.width / 2), Math.round(b.x + b.width));
    ys.push(Math.round(b.y), Math.round(b.y + b.height / 2), Math.round(b.y + b.height));
  }
  return { xs, ys };
}
/** Edges are tried in priority order (left/top, centre, right/bottom); the first edge with any
 * target inside tolerance wins, using its smallest move; later edges are not considered. */
function shift(guides: number[], targets: number[], tolerance: number): { move: number; target: number | null } {
  let best: { move: number; target: number } | null = null;
  // Strictly inside the tolerance radius: a move sitting exactly on the boundary is
  // "just barely too far" and does not snap (matches the move tool's edge-of-range e2e case).
  for (const g of guides) for (const t of targets) { const move = t - g; if (Math.abs(move) >= tolerance) continue; if (best && Math.abs(best.move) <= Math.abs(move)) continue; best = { move, target: t }; }
  return { move: best?.move ?? 0, target: best?.target ?? null };
}
export function snapOffset(box: RectLike, xs: number[], ys: number[], tolerance: number): { dx: number; dy: number; x: number | null; y: number | null } {
  const h = shift([box.x, box.x + box.width / 2, box.x + box.width], xs, tolerance);
  const v = shift([box.y, box.y + box.height / 2, box.y + box.height], ys, tolerance);
  return { dx: h.move, dy: v.move, x: h.target, y: v.target };
}
