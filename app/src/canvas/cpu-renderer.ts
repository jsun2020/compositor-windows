import type { DocumentState } from "../engine/types";
import type { EngineClient } from "../engine/client";
import type { Viewport } from "./viewport";
import type { RenderOptions, Renderer } from "./renderer";

/** Asks the engine for the visible document region at the current zoom and blits it. */
export class CpuRenderer implements Renderer {
  readonly kind = "cpu" as const;
  private ctx: CanvasRenderingContext2D;
  constructor(private readonly canvas: HTMLCanvasElement, private readonly engine: EngineClient) {
    this.ctx = canvas.getContext("2d")!;
  }
  sync(): void {}
  render(state: DocumentState, viewport: Viewport, dpr: number, options: RenderOptions): void {
    const W = Math.max(1, Math.round(viewport.viewSize.width * dpr)), H = Math.max(1, Math.round(viewport.viewSize.height * dpr));
    if (this.canvas.width !== W || this.canvas.height !== H) { this.canvas.width = W; this.canvas.height = H; }
    const ctx = this.ctx;
    ctx.fillStyle = "#292929"; ctx.fillRect(0, 0, W, H);
    const rect = viewport.documentRect({ width: state.width, height: state.height });
    const x = Math.round(rect.x * dpr), y = Math.round(rect.y * dpr);
    const w = Math.max(1, Math.round(rect.width * dpr)), h = Math.max(1, Math.round(rect.height * dpr));
    // Only the part of the document inside the canvas element is composited.
    const vx0 = Math.max(0, x), vy0 = Math.max(0, y), vx1 = Math.min(W, x + w), vy1 = Math.min(H, y + h);
    if (vx1 <= vx0 || vy1 <= vy0) return;
    const docPerPx = state.width / w;
    const region = { x: (vx0 - x) * docPerPx, y: (vy0 - y) * docPerPx, width: (vx1 - vx0) * docPerPx, height: (vy1 - vy0) * docPerPx };
    const outW = vx1 - vx0, outH = vy1 - vy0;
    if (options.checkerboard) {
      for (let cy = 0; cy < outH; cy += 8 * dpr) for (let cx = 0; cx < outW; cx += 8 * dpr) {
        ctx.fillStyle = ((Math.floor(cx / (8 * dpr)) + Math.floor(cy / (8 * dpr))) % 2 === 0) ? "#cccccc" : "#f2f2f2";
        ctx.fillRect(vx0 + cx, vy0 + cy, 8 * dpr, 8 * dpr);
      }
    } else { ctx.clearRect(vx0, vy0, outW, outH); }
    const premultiplied = this.engine.composite(state.id, region, outW, outH);
    const straight = new Uint8ClampedArray(premultiplied.length);
    for (let i = 0; i < premultiplied.length; i += 4) {
      const a = premultiplied[i + 3];
      straight[i + 3] = a;
      for (let c = 0; c < 3; c++) straight[i + c] = a === 0 ? 0 : Math.min(255, Math.round(premultiplied[i + c] * 255 / a));
    }
    const image = new ImageData(straight, outW, outH);
    const scratch = document.createElement("canvas"); scratch.width = outW; scratch.height = outH;
    scratch.getContext("2d")!.putImageData(image, 0, 0);
    ctx.drawImage(scratch, vx0, vy0);
  }
  readPixels(): Uint8Array {
    const img = this.ctx.getImageData(0, 0, this.canvas.width, this.canvas.height).data;
    const out = new Uint8Array(img.length);
    for (let i = 0; i < img.length; i += 4) { const a = img[i + 3]; out[i + 3] = a; for (let c = 0; c < 3; c++) out[i + c] = Math.round(img[i + c] * a / 255); }
    return out;
  }
  dispose(): void {}
}
