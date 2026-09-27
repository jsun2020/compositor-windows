/** How often the ants march, in ms (`updateAntsTimer`, EditorCanvas.swift:2004-2020). */
export const ANTS_INTERVAL_MS = 120;
/** One march step; the phase wraps at the dash period (4 on, 4 off). */
export const nextPhase = (phase: number): number => (phase + 1) % 8;
/** At most this share of the time goes on the ants: a step whose frame took longer than a third of
 * `ANTS_INTERVAL_MS` stretches the period to three times what it took. */
export const ANTS_BUSY_SHARE = 1 / 3;
/** How long to wait before the next march step, `elapsed` ms after the last one began and its
 * frame was drawn: the rest of `ANTS_INTERVAL_MS`, or of `elapsed / ANTS_BUSY_SHARE` if longer
 * (final review F3: a four-million-point outline took hundreds of ms a frame to stroke). */
export const antsDelay = (elapsed: number): number => Math.max(ANTS_INTERVAL_MS, elapsed / ANTS_BUSY_SHARE) - elapsed;

/** The outline's detail for a zoom (device px per document px): the power of two at or above the
 * zoom, at most 1 and at least 1/4096, `min(1, 2^ceil(log2(max(scale, 1/4096))))`
 * (TransformOverlay.swift:124), so the engine traces a very detailed outline no coarser than the
 * screen shows it (`selection_lod`; :92-177, :284-298). */
export function outlineStep(zoom: number): number {
  if (!(zoom > 0)) return 1;
  return Math.min(1, Math.pow(2, Math.ceil(Math.log2(Math.max(zoom, 1 / 4096)))));
}

/** Outlines with more points than this come from the engine as they are only at 1:1 and closer
 * (engine `OUTLINE_DETAIL_LIMIT`); drawn, such an outline keeps only the edges near the view. */
export const OUTLINE_DETAIL_LIMIT = 20_000;

/** How many points a flat outline (`EngineClient.selectionOutline`) holds, all contours together. */
export function outlinePoints(flat: Float64Array): number {
  let points = 0, i = 1;
  for (let c = 0; c < (flat[0] ?? 0); c++) { const n = flat[i]; points += n; i += 1 + 2 * n; }
  return points;
}

/** The last outline fetched, kept until the document, the selection's revision or the step
 * changes: the ants redraw every tick without asking the engine again. */
export class OutlineCache {
  private key: string | null = null;
  private outline: Float64Array = new Float64Array(0);
  get(doc: string, revision: number, step: number, fetch: () => Float64Array): Float64Array {
    const key = `${doc}:${revision}:${step}`;
    if (key !== this.key) { this.outline = fetch(); this.key = key; }
    return this.outline;
  }
}

/** How far past the view, as a fraction of its size on each side, a detailed outline's path keeps
 * its edges: a pan within it reuses the path, and every tick strokes only about 1.5 x 1.5 views.
 */
export const CULL_MARGIN = 0.25;

/** What a path is built into: a `Path2D`, or a recorder in the unit tests. */
export interface PathSink { moveTo(x: number, y: number): void; lineTo(x: number, y: number): void; closePath(): void; }
/** A box in view px relative to the document's scaled origin. */
export interface Box { x0: number; y0: number; x1: number; y1: number; }

/** The flat outline into `sink`, each point scaled by `scale` view px per document px. With `cull`,
 * only the edges whose bounds meet it, each run of them one open subpath; without, every contour
 * closed. */
export function traceOutline(flat: Float64Array, scale: number, cull: Box | null, sink: PathSink): void {
  let i = 1;
  for (let c = 0; c < (flat[0] ?? 0); c++) {
    const n = flat[i++], start = i;
    i += 2 * n;
    if (n === 0) continue;
    if (!cull) {
      sink.moveTo(flat[start] * scale, flat[start + 1] * scale);
      for (let k = 1; k < n; k++) sink.lineTo(flat[start + 2 * k] * scale, flat[start + 2 * k + 1] * scale);
      sink.closePath();
      continue;
    }
    let open = false;
    for (let k = 0; k < n; k++) {
      const a = start + 2 * k, b = start + 2 * ((k + 1) % n);
      const ax = flat[a] * scale, ay = flat[a + 1] * scale, bx = flat[b] * scale, by = flat[b + 1] * scale;
      if (Math.max(ax, bx) < cull.x0 || Math.min(ax, bx) > cull.x1 || Math.max(ay, by) < cull.y0 || Math.min(ay, by) > cull.y1) { open = false; continue; }
      if (!open) { sink.moveTo(ax, ay); open = true; }
      sink.lineTo(bx, by);
    }
  }
}

/** The ants' path, kept while the outline, the zoom and (for a detailed outline) the neighbourhood
 * of the view stay the same: a march tick only moves the dash, and a pan or an outline drag only
 * moves where the path is stroked (final review F3). The path is in view px about the document's
 * scaled origin; the caller translates to it. A detailed outline keeps the edges within
 * `CULL_MARGIN` of the view on every side, so small pans reuse it. */
export class AntsPathCache<P extends PathSink> {
  private outline: Float64Array | null = null;
  private points = 0;
  private scale = 0;
  private cull: Box | null = null;
  private path: P | null = null;
  /** Paths built, for the tests. */
  builds = 0;
  constructor(private readonly make: () => P) {}
  get(outline: Float64Array, scale: number, view: Box): P {
    if (outline !== this.outline) { this.outline = outline; this.points = outlinePoints(outline); this.path = null; }
    const culls = this.points > OUTLINE_DETAIL_LIMIT;
    const inside = (b: Box | null) => !!b && view.x0 >= b.x0 && view.y0 >= b.y0 && view.x1 <= b.x1 && view.y1 <= b.y1;
    if (this.path && this.scale === scale && (!culls || inside(this.cull))) return this.path;
    const mx = CULL_MARGIN * (view.x1 - view.x0), my = CULL_MARGIN * (view.y1 - view.y0);
    this.cull = culls ? { x0: view.x0 - mx, y0: view.y0 - my, x1: view.x1 + mx, y1: view.y1 + my } : null;
    this.scale = scale;
    this.path = this.make();
    traceOutline(outline, scale, this.cull, this.path);
    this.builds++;
    return this.path;
  }
}
