import type { LayerTransform } from "../engine/types";
import { boundsOf, cornersDrag, snapOffset, transformDrag, type P, type TransformDragMode } from "./transform-geometry";

export interface SnapTargetsWithTolerance { xs: number[]; ys: number[]; tolerance: number; }
export interface SessionInit { mode: TransformDragMode; startDoc: P; original: LayerTransform; originalCorners: P[] | null; snap: SnapTargetsWithTolerance | null; }
export interface SessionResult { draft: LayerTransform; corners: P[] | null; guides: { xs: number[]; ys: number[] }; }

export class TransformSession {
  constructor(private readonly init: SessionInit) {}
  update(point: P, mods: { shift: boolean; alt: boolean; ctrl: boolean }): SessionResult {
    const { mode, startDoc, original, originalCorners, snap } = this.init;
    const guides = { xs: [] as number[], ys: [] as number[] };
    if (mode.kind === "distort" && originalCorners) {
      const corners = cornersDrag(originalCorners, startDoc, mode.index < 0 ? "move" : mode.index).updated(point, mods.shift);
      return { draft: original, corners, guides };
    }
    let draft = transformDrag(original, startDoc, mode).updated(point, { lockRatio: mode.kind === "resize", shift: mods.shift, alt: mods.alt });
    if (mode.kind === "move" && snap) {
      const s = snapOffset(boundsOf(draft), snap.xs, snap.ys, snap.tolerance);
      if (s.dx !== 0 || s.dy !== 0) draft = { ...draft, origin: [draft.origin[0] + s.dx, draft.origin[1] + s.dy] };
      if (s.x !== null) guides.xs.push(s.x); if (s.y !== null) guides.ys.push(s.y);
    }
    return { draft, corners: originalCorners, guides };
  }
}

/** What a press starts. Ctrl on a corner handle begins a distortion; with corners pending every handle moves corners. */
export function startMode(hit: TransformDragMode | null, inside: boolean, ctrl: boolean, hasCorners: boolean): TransformDragMode {
  if (hasCorners) { if (hit && hit.kind === "resize") return { kind: "distort", index: hit.index }; return { kind: "distort", index: -1 }; }
  if (hit && hit.kind === "resize" && ctrl && hit.index % 2 === 0) return { kind: "distort", index: hit.index };
  if (hit) return hit;
  void inside; // a press anywhere moves the active layer, as macOS does
  return { kind: "move" };
}

export function nudgeDelta(key: string, shift: boolean): { dx: number; dy: number } | null {
  const step = shift ? 10 : 1;
  switch (key) { case "ArrowLeft": return { dx: -step, dy: 0 }; case "ArrowRight": return { dx: step, dy: 0 }; case "ArrowUp": return { dx: 0, dy: -step }; case "ArrowDown": return { dx: 0, dy: step }; default: return null; }
}
