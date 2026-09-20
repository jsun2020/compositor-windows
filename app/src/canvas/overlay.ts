import type { Viewport } from "./viewport";
import type { Rect } from "../tools/crop-geometry";
import { HANDLES } from "../tools/crop-geometry";

export interface OverlayState { docWidth: number; docHeight: number; cropRect: Rect | null; guides: { xs: number[]; ys: number[] }; }

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
  ctx.strokeStyle = "#ff40ff"; ctx.lineWidth = 1;
  for (const x of state.guides.xs) { const v = viewport.viewPoint({ x, y: 0 }, size).x; ctx.beginPath(); ctx.moveTo(v + 0.5, 0); ctx.lineTo(v + 0.5, viewport.viewSize.height); ctx.stroke(); }
  for (const y of state.guides.ys) { const v = viewport.viewPoint({ x: 0, y }, size).y; ctx.beginPath(); ctx.moveTo(0, v + 0.5); ctx.lineTo(viewport.viewSize.width, v + 0.5); ctx.stroke(); }
}
