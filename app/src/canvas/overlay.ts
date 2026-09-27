import type { Viewport } from "./viewport";
import type { Rect } from "../tools/crop-geometry";
import { HANDLES } from "../tools/crop-geometry";
import type { OverlayGeometry } from "../tools/transform-geometry";
import type { Guide, SelectionShape } from "../engine/types";

/** The marching ants: the selection's outline as a path in view px about `at`, the document's
 * scaled origin (moved by the outline's offset while it is dragged), built once and kept by
 * `AntsPathCache` (canvas/ants.ts). */
export interface AntsState { path: Path2D; at: { x: number; y: number }; phase: number; }
/** An outline being drawn, in document pixels (`LassoDraft`). */
export interface DraftState { kind: SelectionShape; points: { x: number; y: number }[]; cursor: { x: number; y: number } | null; }

export interface OverlayState {
  docWidth: number; docHeight: number; cropRect: Rect | null;
  /** Temporary smart-snap lines shown while dragging a crop or transform; not the saved guides. */
  guides: { xs: number[]; ys: number[] };
  transform: OverlayGeometry | null;
  /** The document's saved guides, or null while View > Hide Guides is in effect. */
  canvasGuides: Guide[] | null;
  /** Null with no selection, or an empty one. */
  ants?: AntsState | null;
  draft?: DraftState | null;
}

/** The dash the ants march along, in view px (`drawSelection`, TransformOverlay.swift:283-298). */
export const ANTS_DASH = 4;

/** A white line under a black dash shifted by `phase` (TransformOverlay.swift:283-298): the kept
 * path stroked twice where it lies, nothing rebuilt. */
function drawAnts(ctx: CanvasRenderingContext2D, ants: AntsState): void {
  ctx.save();
  ctx.translate(ants.at.x, ants.at.y);
  ctx.lineWidth = 1;
  ctx.strokeStyle = "white";
  ctx.stroke(ants.path);
  ctx.setLineDash([ANTS_DASH, ANTS_DASH]);
  ctx.lineDashOffset = ants.phase;
  ctx.strokeStyle = "black";
  ctx.stroke(ants.path);
  ctx.restore();
}

/** The outline being drawn: black 0.8 alpha 2 px under white 1 px; the Polygonal Lasso's first
 * corner as an 8 px handle to click (`drawLassoDraft`, TransformOverlay.swift:301-333). */
function drawDraft(ctx: CanvasRenderingContext2D, viewport: Viewport, size: { width: number; height: number }, draft: DraftState): void {
  const points = draft.points.map((p) => viewport.viewPoint(p, size));
  if (draft.kind === "Polygonal" && draft.cursor) points.push(viewport.viewPoint(draft.cursor, size));
  if (points.length === 0) return;
  ctx.save();
  ctx.beginPath();
  if (draft.kind === "Ellipse" && points.length === 4) {
    const xs = points.map((p) => p.x), ys = points.map((p) => p.y);
    const x0 = Math.min(...xs), x1 = Math.max(...xs), y0 = Math.min(...ys), y1 = Math.max(...ys);
    ctx.ellipse((x0 + x1) / 2, (y0 + y1) / 2, (x1 - x0) / 2, (y1 - y0) / 2, 0, 0, Math.PI * 2);
  } else {
    points.forEach((p, i) => { if (i === 0) ctx.moveTo(p.x, p.y); else ctx.lineTo(p.x, p.y); });
    if (draft.kind === "Rectangle") ctx.closePath();
  }
  ctx.strokeStyle = "rgba(0,0,0,0.8)"; ctx.lineWidth = 2; ctx.stroke();
  ctx.strokeStyle = "white"; ctx.lineWidth = 1; ctx.stroke();
  if (draft.kind === "Polygonal") {
    ctx.fillStyle = "white"; ctx.fillRect(points[0].x - 4, points[0].y - 4, 8, 8);
    ctx.strokeStyle = "black"; ctx.strokeRect(points[0].x - 4, points[0].y - 4, 8, 8);
  }
  ctx.restore();
}

export function drawOverlay(ctx: CanvasRenderingContext2D, viewport: Viewport, dpr: number, state: OverlayState): void {
  const W = ctx.canvas.width, H = ctx.canvas.height;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, W, H);
  const size = { width: state.docWidth, height: state.docHeight };
  const rect = viewport.documentRect(size);
  // Pixel grid once a document pixel is 8 device pixels or larger.
  if (viewport.zoom >= 8) {
    ctx.strokeStyle = "rgba(0,0,0,0.25)"; ctx.lineWidth = 1 / dpr;
    ctx.beginPath();
    const step = viewport.pointsPerPixel;
    const x0 = Math.max(0, Math.floor(-rect.x / step)), x1 = Math.min(state.docWidth, Math.ceil((viewport.viewSize.width - rect.x) / step));
    const y0 = Math.max(0, Math.floor(-rect.y / step)), y1 = Math.min(state.docHeight, Math.ceil((viewport.viewSize.height - rect.y) / step));
    for (let x = x0; x <= x1; x++) { const vx = rect.x + x * step; ctx.moveTo(vx, rect.y); ctx.lineTo(vx, rect.y + rect.height); }
    for (let y = y0; y <= y1; y++) { const vy = rect.y + y * step; ctx.moveTo(rect.x, vy); ctx.lineTo(rect.x + rect.width, vy); }
    ctx.stroke();
  }
  // Saved guides go under the crop dimming and transform handles, as the Mac draws them first
  // (TransformOverlay.swift:92-100).
  if (state.canvasGuides) {
    ctx.strokeStyle = "rgba(0,255,255,0.9)"; ctx.lineWidth = 1 / dpr;
    for (const g of state.canvasGuides) {
      ctx.beginPath();
      if (g.axis === "vertical") { const v = viewport.viewPoint({ x: g.position, y: 0 }, size).x; ctx.moveTo(v, 0); ctx.lineTo(v, viewport.viewSize.height); }
      else { const v = viewport.viewPoint({ x: 0, y: g.position }, size).y; ctx.moveTo(0, v); ctx.lineTo(viewport.viewSize.width, v); }
      ctx.stroke();
    }
  }
  if (state.cropRect) {
    const c = state.cropRect;
    const tl = viewport.viewPoint({ x: c.x, y: c.y }, size), br = viewport.viewPoint({ x: c.x + c.width, y: c.y + c.height }, size);
    ctx.fillStyle = "rgba(0,0,0,0.5)";
    ctx.beginPath();
    ctx.rect(0, 0, viewport.viewSize.width, viewport.viewSize.height);
    ctx.rect(tl.x, tl.y, br.x - tl.x, br.y - tl.y);
    ctx.fill("evenodd");
    ctx.strokeStyle = "white"; ctx.lineWidth = 1;
    ctx.strokeRect(tl.x + 0.5, tl.y + 0.5, br.x - tl.x, br.y - tl.y);
    ctx.strokeStyle = "rgba(255,255,255,0.4)";
    for (const f of [1 / 3, 2 / 3]) {
      ctx.beginPath(); ctx.moveTo(tl.x + (br.x - tl.x) * f, tl.y); ctx.lineTo(tl.x + (br.x - tl.x) * f, br.y); ctx.stroke();
      ctx.beginPath(); ctx.moveTo(tl.x, tl.y + (br.y - tl.y) * f); ctx.lineTo(br.x, tl.y + (br.y - tl.y) * f); ctx.stroke();
    }
    ctx.fillStyle = "white";
    for (const h of HANDLES) { const hx = tl.x + (br.x - tl.x) * h.x, hy = tl.y + (br.y - tl.y) * h.y; ctx.fillRect(hx - 4, hy - 4, 8, 8); }
  }
  if (state.transform) {
    const g = state.transform;
    const corners = [g.handles[0], g.handles[2], g.handles[4], g.handles[6]];
    ctx.strokeStyle = "#3a6ea5"; ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(corners[0].x + 0.5, corners[0].y + 0.5);
    for (const c of corners.slice(1)) ctx.lineTo(c.x + 0.5, c.y + 0.5);
    ctx.closePath();
    ctx.stroke();
    if (g.showsRotation) {
      ctx.beginPath();
      ctx.moveTo(g.handles[1].x + 0.5, g.handles[1].y + 0.5);
      ctx.lineTo(g.rotationHandle.x + 0.5, g.rotationHandle.y + 0.5);
      ctx.stroke();
      ctx.beginPath();
      ctx.arc(g.rotationHandle.x, g.rotationHandle.y, 4, 0, Math.PI * 2);
      ctx.fillStyle = "white";
      ctx.fill();
      ctx.stroke();
    }
    for (const h of g.handles) {
      ctx.fillStyle = "white";
      ctx.fillRect(h.x - 3.5, h.y - 3.5, 7, 7);
      ctx.strokeStyle = "#3a6ea5"; ctx.lineWidth = 1;
      ctx.strokeRect(h.x - 3.5, h.y - 3.5, 7, 7);
    }
  }
  if (state.ants) drawAnts(ctx, state.ants);
  if (state.draft) drawDraft(ctx, viewport, size, state.draft);
  ctx.strokeStyle = "#ff40ff"; ctx.lineWidth = 1;
  for (const x of state.guides.xs) { const v = viewport.viewPoint({ x, y: 0 }, size).x; ctx.beginPath(); ctx.moveTo(v + 0.5, 0); ctx.lineTo(v + 0.5, viewport.viewSize.height); ctx.stroke(); }
  for (const y of state.guides.ys) { const v = viewport.viewPoint({ x: 0, y }, size).y; ctx.beginPath(); ctx.moveTo(0, v + 0.5); ctx.lineTo(viewport.viewSize.width, v + 0.5); ctx.stroke(); }
}
