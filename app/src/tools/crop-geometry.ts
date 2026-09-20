import type { PointLike, RectLike } from "../canvas/viewport";
export type Rect = RectLike;
export type Point = PointLike;

export const HANDLES: Point[] = [
  { x: 0, y: 0 }, { x: 0.5, y: 0 }, { x: 1, y: 0 }, { x: 1, y: 0.5 }, { x: 1, y: 1 }, { x: 0.5, y: 1 }, { x: 0, y: 1 }, { x: 0, y: 0.5 },
];

function standardized(r: Rect): Rect {
  const x = Math.min(r.x, r.x + r.width), y = Math.min(r.y, r.y + r.height);
  return { x, y, width: Math.abs(r.width), height: Math.abs(r.height) };
}
/** Whole document pixels, at least 1 wide and high (matches CropGeometry.snapped). */
export function snapped(rect: Rect): Rect {
  const r = standardized(rect);
  const x = Math.round(r.x), y = Math.round(r.y);
  return { x, y, width: Math.max(1, Math.round(r.x + r.width) - x), height: Math.max(1, Math.round(r.y + r.height) - y) };
}
export function isValid(r: Rect): boolean {
  return [r.x, r.y, r.width, r.height].every(Number.isFinite) && r.width >= 1 && r.width <= 30_000 && r.height >= 1 && r.height <= 30_000
    && Math.abs(r.x) <= 1_000_000 && Math.abs(r.y) <= 1_000_000;
}
/** A frame dragged from start to end; with a ratio the longer axis wins; symmetric grows out from start. */
export function create(start: Point, end: Point, ratio: number | null, symmetric: boolean): Rect {
  let dx = end.x - start.x, dy = end.y - start.y;
  if (ratio !== null) {
    if (Math.abs(dx) > Math.abs(dy) * ratio) dy = (dy < 0 ? -1 : 1) * Math.abs(dx) / ratio;
    else dx = (dx < 0 ? -1 : 1) * Math.abs(dy) * ratio;
  }
  if (symmetric) return snapped({ x: start.x - Math.abs(dx), y: start.y - Math.abs(dy), width: Math.abs(dx) * 2, height: Math.abs(dy) * 2 });
  return snapped({ x: Math.min(start.x, start.x + dx), y: Math.min(start.y, start.y + dy), width: Math.abs(dx), height: Math.abs(dy) });
}

/** Port of TransformDrag.updated for the resize case, rotation 0. */
export function resizeRect(original: Rect, index: number, start: Point, point: Point, lockRatio: boolean, symmetric: boolean): Rect {
  const handle = HANDLES[index];
  const anchorUnit = symmetric ? { x: 0.5, y: 0.5 } : { x: 1 - handle.x, y: 1 - handle.y };
  const unitPoint = (u: Point) => ({ x: original.x + u.x * original.width, y: original.y + u.y * original.height });
  const anchor = unitPoint(anchorUnit);
  const initial = unitPoint(handle);
  const dx = initial.x + point.x - start.x - anchor.x;
  const dy = initial.y + point.y - start.y - anchor.y;
  const span = symmetric ? 2 : 1;
  const localX = dx * span, localY = dy * span;
  const sx = handle.x * 2 - 1, sy = handle.y * 2 - 1;
  let width = sx === 0 ? original.width : Math.max(1, localX * sx);
  let height = sy === 0 ? original.height : Math.max(1, localY * sy);
  if (lockRatio) {
    let factor: number;
    if (sx === 0) factor = height / original.height;
    else if (sy === 0) factor = width / original.width;
    else factor = Math.max(1 / Math.min(original.width, original.height),
      (localX * sx * original.width + localY * sy * original.height) / (original.width * original.width + original.height * original.height));
    width = original.width * factor; height = original.height * factor;
  }
  const cx = anchor.x + (0.5 - anchorUnit.x) * width, cy = anchor.y + (0.5 - anchorUnit.y) * height;
  const result = { x: cx - width / 2, y: cy - height / 2, width, height };
  return isValid(result) ? result : original;
}

export type CropDragMode = { kind: "create" } | { kind: "move" } | { kind: "resize"; index: number };
export interface CropDrag {
  mode: CropDragMode; start: Point; original: Rect;
  updated(point: Point, ratio: number | null, symmetric: boolean): Rect;
}
export function cropDrag(mode: CropDragMode, start: Point, original: Rect): CropDrag {
  return {
    mode, start, original,
    updated(point, ratio, symmetric) {
      switch (mode.kind) {
        case "create": return create(start, point, ratio, symmetric);
        case "move": return snapped({ ...original, x: original.x + point.x - start.x, y: original.y + point.y - start.y });
        case "resize": return snapped(resizeRect(original, mode.index, start, point, ratio !== null, symmetric));
      }
    },
  };
}

/** Crop edges snap to nearby layer and canvas edges while dragging. */
export class CropSnap {
  constructor(readonly xs: number[], readonly ys: number[], readonly tolerance: number) {}
  private nearest(value: number, targets: number[]): number | null {
    let best: number | null = null;
    for (const t of targets) {
      if (Math.abs(t - value) > this.tolerance) continue;
      if (best !== null && Math.abs(best - value) <= Math.abs(t - value)) continue;
      best = t;
    }
    return best;
  }
  apply(rect: Rect, drag: CropDrag, point: Point, ratio: number | null, symmetric: boolean): Rect {
    if (this.tolerance <= 0) return rect;
    let horizontal: boolean, vertical: boolean;
    switch (drag.mode.kind) {
      case "move": {
        const shift = (edges: number[], targets: number[]) => {
          const moves = edges.map((e) => { const n = this.nearest(e, targets); return n === null ? null : n - e; }).filter((m): m is number => m !== null);
          return moves.length ? moves.reduce((a, b) => (Math.abs(a) < Math.abs(b) ? a : b)) : 0;
        };
        const dx = shift([rect.x, rect.x + rect.width], this.xs), dy = shift([rect.y, rect.y + rect.height], this.ys);
        return { ...rect, x: rect.x + dx, y: rect.y + dy };
      }
      case "create": if (ratio !== null) return rect; horizontal = true; vertical = true; break;
      case "resize": { if (ratio !== null) return rect; const h = HANDLES[drag.mode.index]; horizontal = h.x !== 0.5; vertical = h.y !== 0.5; }
    }
    let r = { ...rect };
    if (horizontal) {
      if (Math.abs(point.x - r.x) <= Math.abs(point.x - (r.x + r.width))) {
        const x = this.nearest(r.x, this.xs);
        if (x !== null && x < r.x + r.width) r = { x, y: r.y, width: r.x + r.width - x, height: r.height };
      } else {
        const x = this.nearest(r.x + r.width, this.xs);
        if (x !== null && x > r.x) r.width = x - r.x;
      }
    }
    if (vertical) {
      if (Math.abs(point.y - r.y) <= Math.abs(point.y - (r.y + r.height))) {
        const y = this.nearest(r.y, this.ys);
        if (y !== null && y < r.y + r.height) r = { x: r.x, y, width: r.width, height: r.y + r.height - y };
      } else {
        const y = this.nearest(r.y + r.height, this.ys);
        if (y !== null && y > r.y) r.height = y - r.y;
      }
    }
    if (symmetric) {
      let center = { x: drag.original.x + drag.original.width / 2, y: drag.original.y + drag.original.height / 2 };
      if (drag.mode.kind === "create") center = drag.start;
      if (horizontal) {
        const half = point.x >= center.x ? r.x + r.width - center.x : center.x - r.x;
        if (half >= 0.5) { r.x = center.x - half; r.width = half * 2; }
      }
      if (vertical) {
        const half = point.y >= center.y ? r.y + r.height - center.y : center.y - r.y;
        if (half >= 0.5) { r.y = center.y - half; r.height = half * 2; }
      }
    }
    return r;
  }
}
