export interface SizeLike { width: number; height: number; }
export interface PointLike { x: number; y: number; }
export interface RectLike { x: number; y: number; width: number; height: number; }

export const ZOOM_RANGE: [number, number] = [0.001, 32];
const FIT_MARGIN = 96;

/** Document: pixels, top-left origin. View: CSS pixels. Zoom 1 draws one document pixel per device pixel. */
export class Viewport {
  viewSize: SizeLike = { width: 0, height: 0 };
  backingScale = 1;
  private _zoom = 1;
  pan: SizeLike = { width: 0, height: 0 };
  private _followsFit = true;

  get zoom(): number { return this._zoom; }
  get followsFit(): boolean { return this._followsFit; }
  get pointsPerPixel(): number { return this._zoom / this.backingScale; }
  get center(): PointLike { return { x: this.viewSize.width / 2, y: this.viewSize.height / 2 }; }

  documentRect(size: SizeLike): RectLike {
    const w = size.width * this.pointsPerPixel, h = size.height * this.pointsPerPixel;
    const c = this.center;
    return { x: c.x - w / 2 + this.pan.width, y: c.y - h / 2 + this.pan.height, width: w, height: h };
  }
  documentPoint(point: PointLike, size: SizeLike): PointLike {
    const o = this.documentRect(size);
    return { x: (point.x - o.x) / this.pointsPerPixel, y: (point.y - o.y) / this.pointsPerPixel };
  }
  viewPoint(point: PointLike, size: SizeLike): PointLike {
    const o = this.documentRect(size);
    return { x: o.x + point.x * this.pointsPerPixel, y: o.y + point.y * this.pointsPerPixel };
  }
  fit(size: SizeLike): void {
    if (this.viewSize.width <= 0 || this.viewSize.height <= 0) { this._followsFit = true; return; }
    const zoom = Math.min(Math.max(1, this.viewSize.width - FIT_MARGIN) / size.width,
      Math.max(1, this.viewSize.height - FIT_MARGIN) / size.height) * this.backingScale;
    this._zoom = clamp(zoom);
    this.pan = { width: 0, height: 0 };
    this._followsFit = true;
  }
  resize(size: SizeLike, backingScale: number, docSize?: SizeLike): void {
    const oldScale = this.pointsPerPixel;
    this.viewSize = { ...size };
    this.backingScale = Math.max(1, backingScale);
    if (this._followsFit && docSize) { this.fit(docSize); }
    else {
      const ratio = this.pointsPerPixel / oldScale;
      this.pan = { width: this.pan.width * ratio, height: this.pan.height * ratio };
    }
  }
  setZoom(value: number, anchor: PointLike, size: SizeLike): void {
    if (!Number.isFinite(value)) return;
    const pixel = this.documentPoint(anchor, size);
    this._zoom = clamp(value);
    const moved = this.viewPoint(pixel, size);
    this.pan = { width: this.pan.width + anchor.x - moved.x, height: this.pan.height + anchor.y - moved.y };
    this._followsFit = false;
  }
  translate(delta: SizeLike): void {
    this.pan = { width: this.pan.width + delta.width, height: this.pan.height + delta.height };
    this._followsFit = false;
  }
}

function clamp(value: number): number { return Math.min(ZOOM_RANGE[1], Math.max(ZOOM_RANGE[0], value)); }
