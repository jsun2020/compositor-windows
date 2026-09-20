export type CanvasUnit = "Pixels" | "Percent" | "Inches" | "Centimeters";
export const CANVAS_UNITS: CanvasUnit[] = ["Pixels", "Percent", "Inches", "Centimeters"];

export class CanvasSizeDraft {
  width: number; height: number;
  relative = false; locked = false; unit: CanvasUnit = "Pixels";
  constructor(readonly originalWidth: number, readonly originalHeight: number, readonly resolution: number) {
    this.width = originalWidth; this.height = originalHeight;
  }
  get valid(): boolean {
    const w = Math.round(this.width), h = Math.round(this.height);
    return Number.isFinite(this.width) && Number.isFinite(this.height) && w >= 1 && w <= 30_000 && h >= 1 && h <= 30_000;
  }
  displayed(widthAxis: boolean): number {
    const original = widthAxis ? this.originalWidth : this.originalHeight;
    const pixels = (widthAxis ? this.width : this.height) - (this.relative ? original : 0);
    switch (this.unit) {
      case "Pixels": return pixels;
      case "Percent": return pixels / original * 100;
      case "Inches": return pixels / this.resolution;
      case "Centimeters": return pixels / this.resolution * 2.54;
    }
  }
  set(value: number, widthAxis: boolean): void {
    const original = widthAxis ? this.originalWidth : this.originalHeight;
    let pixels: number;
    switch (this.unit) {
      case "Pixels": pixels = value; break;
      case "Percent": pixels = value / 100 * original; break;
      case "Inches": pixels = value * this.resolution; break;
      case "Centimeters": pixels = value / 2.54 * this.resolution; break;
    }
    const final = pixels + (this.relative ? original : 0);
    if (widthAxis) { this.width = final; if (this.locked) this.height = final * this.originalHeight / this.originalWidth; }
    else { this.height = final; if (this.locked) this.width = final * this.originalWidth / this.originalHeight; }
  }
}
