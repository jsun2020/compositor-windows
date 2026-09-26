import init, { WasmEngine } from "./pkg/compositor_engine.js";
import type { Command, Dirty, DocumentState, LayerAdjustment, LayerTransform, LevelsAuto, LevelsSample, LevelsSettings, PackageFiles, PreviewEdit, PreviewRequest, RenderPlan, SpatialBlur, SpatialGrid } from "./types";

export class EngineClient {
  private constructor(private readonly wasm: WasmEngine, private readonly memory: WebAssembly.Memory) {}

  static async load(): Promise<EngineClient> {
    const exports = await init();
    return new EngineClient(new WasmEngine(), exports.memory);
  }

  version(): string { return this.wasm.version(); }
  newDocument(width: number, height: number, emptyLayer: boolean): string { return this.wasm.new_document(width, height, emptyLayer); }
  openPackage(files: PackageFiles, path: string | null): string {
    return this.wasm.open_package(files.manifest, files.images.map((i) => i.name), files.images.map((i) => i.bytes), path ?? undefined);
  }
  savePackage(doc: string): PackageFiles {
    this.wasm.prepare_save(doc);
    try {
      const manifest = this.wasm.save_package_manifest(doc);
      const images = this.wasm.save_package_image_names(doc).map((name) => ({ name, bytes: this.wasm.save_package_image(doc, name) }));
      return { manifest, images };
    } finally { this.wasm.finish_save(doc); }
  }
  markSaved(doc: string, path: string | null): void { this.wasm.mark_saved(doc, path ?? undefined); }
  closeDocument(doc: string): void { this.wasm.close_document(doc); }
  documentIds(): string[] { return this.wasm.document_ids(); }
  state(doc: string): DocumentState { return JSON.parse(this.wasm.state(doc)) as DocumentState; }
  execute(doc: string, command: Command): Dirty { return JSON.parse(this.wasm.execute(doc, JSON.stringify(command))) as Dirty; }
  undo(doc: string): Dirty { return JSON.parse(this.wasm.undo(doc)) as Dirty; }
  redo(doc: string): Dirty { return JSON.parse(this.wasm.redo(doc)) as Dirty; }
  /** Drops the last history entry and returns to the state before it, with no redo: for a
   * gesture the user cancelled, such as Escape during an Alt-drag duplicate. */
  revert(doc: string): Dirty { return JSON.parse(this.wasm.revert(doc)) as Dirty; }
  importImage(doc: string | null, bytes: Uint8Array, name: string, at: { x: number; y: number } | null): string {
    return this.wasm.import_image(doc ?? undefined, bytes, name, at?.x, at?.y);
  }
  exportPng(doc: string): Uint8Array { return this.wasm.export_png(doc); }
  exportJpeg(doc: string, quality: number, matte: [number, number, number]): Uint8Array { return this.wasm.export_jpeg(doc, quality, ...matte); }
  /** Composites and encodes the document scaled to fit `maxSide` (aspect preserved), for a
   * fast quality/matte preview image that does not need a full-resolution encode. */
  exportJpegPreview(doc: string, quality: number, matte: [number, number, number], maxSide: number): Uint8Array {
    return this.wasm.export_jpeg_preview(doc, quality, ...matte, maxSide);
  }
  composite(doc: string, region: { x: number; y: number; width: number; height: number }, outWidth: number, outHeight: number): Uint8Array {
    return this.wasm.composite(doc, region.x, region.y, region.width, region.height, outWidth, outHeight);
  }
  /** A view on wasm memory; valid only until the next engine call. `level` is how many sharp
   * halvings to apply first (see `prefilterLevel`), matching the CPU compositor's prefilter.
   *
   * The pointer call runs before `this.memory.buffer` is read on purpose: marshalling the two
   * string arguments into wasm can grow linear memory, which detaches any buffer captured
   * beforehand. */
  layerPixels(doc: string, layer: string, level = 0): Uint8Array | null {
    const len = this.wasm.layer_pixels_len(doc, layer, level);
    if (len === 0) return null;
    const ptr = this.wasm.layer_pixels_ptr(doc, layer, level);
    return new Uint8Array(this.memory.buffer, ptr, len);
  }

  /** Hands `use` what the plan's draw of `layer` samples at `level` (engine `Engine::draw_raster`):
   * the padded effects image when the plan draws the layer's effects, else its pixels. The GL
   * renderer uploads this, so it draws the bytes the CPU compositor samples. The engine makes the
   * raster once and keeps it while `use` runs (`prepare_draw_pixels`), so the pointer is read from
   * that one raster, not from a second computation; then it drops it (`release_draw_pixels`), as
   * one near the limit is about 0.8 GB. The view is valid only inside `use`, which must copy what
   * it keeps (texImage2D does). Same view rules as `layerPixels`. */
  drawPixels<T>(doc: string, layer: string, level: number, edit: PreviewEdit | null, use: (pixels: Uint8Array | null) => T): T {
    const len = this.wasm.prepare_draw_pixels(doc, layer, level, edit ? JSON.stringify(edit) : undefined);
    try {
      if (len === 0) return use(null);
      const ptr = this.wasm.draw_pixels_ptr();
      return use(new Uint8Array(this.memory.buffer, ptr, len));
    } finally {
      this.wasm.release_draw_pixels();
    }
  }

  renderPlan(doc: string, edit: PreviewEdit | null): RenderPlan { return JSON.parse(this.wasm.render_plan(doc, edit ? JSON.stringify(edit) : undefined)) as RenderPlan; }
  compositeEdit(doc: string, edit: PreviewEdit | null, region: { x: number; y: number; width: number; height: number }, outWidth: number, outHeight: number): Uint8Array {
    return this.wasm.composite_edit(doc, edit ? JSON.stringify(edit) : undefined, region.x, region.y, region.width, region.height, outWidth, outHeight);
  }
  /** A view on wasm memory; valid only until the next engine call. The pointer is sequenced
   * into a local before `this.memory.buffer` is read: argument evaluation is left to right, so
   * reading the buffer first would capture it before `mask_pixels_ptr` marshals its two string
   * arguments through `__wbindgen_malloc`, which can grow memory and detach that buffer. */
  maskPixels(doc: string, layer: string): Uint8Array | null {
    const len = this.wasm.mask_pixels_len(doc, layer);
    if (len === 0) return null;
    const ptr = this.wasm.mask_pixels_ptr(doc, layer);
    return new Uint8Array(this.memory.buffer, ptr, len);
  }
  clipDependents(doc: string, ids: string[]): string[] { return JSON.parse(this.wasm.clip_dependents(doc, JSON.stringify(ids))) as string[]; }
  mergeAction(doc: string, ids: string[]): string | null { return this.wasm.merge_action(doc, JSON.stringify(ids)) ?? null; }
  groupBox(doc: string, ids: string[]): LayerTransform | null { const t = this.wasm.group_box(doc, JSON.stringify(ids)); return t ? (JSON.parse(t) as LayerTransform) : null; }
  canToggleClipping(doc: string, id: string): boolean { return this.wasm.can_toggle_clipping(doc, id); }
  canPlace(doc: string, id: string, parent: string | null): boolean { return this.wasm.can_place(doc, id, parent ?? undefined); }

  /** Substitutes one layer's pixels with the open panel's result. Cleared by any command, undo
   * or redo, and by passing null. */
  setPreview(doc: string, request: PreviewRequest | null): Dirty {
    return JSON.parse(this.wasm.set_preview(doc, request ? JSON.stringify(request) : undefined)) as Dirty;
  }
  /** Four arrays of 256 bins: the mean of the channels, then red, green and blue. */
  histogram(doc: string, layer: string): number[][] { return JSON.parse(this.wasm.histogram(doc, layer)) as number[][]; }
  /** Auto Levels from `histogram`'s bins, which the panel already holds: nothing is recomposited. */
  autoLevels(histogram: number[][], mode: LevelsAuto): LevelsSettings { return JSON.parse(this.wasm.auto_levels(JSON.stringify(histogram), mode)) as LevelsSettings; }
  levelsSampling(doc: string, layer: string, settings: LevelsSettings, at: { x: number; y: number }, mode: LevelsSample): LevelsSettings {
    return JSON.parse(this.wasm.levels_sampling(doc, layer, JSON.stringify(settings), at.x, at.y, mode)) as LevelsSettings;
  }
  sampleLayerColor(doc: string, layer: string, at: { x: number; y: number }): [number, number, number] | null {
    const json = this.wasm.sample_layer_color(doc, layer, at.x, at.y);
    return json ? (JSON.parse(json) as [number, number, number]) : null;
  }
  /** Reads the STORED document, never an open preview -- for the Hue/Saturation eyedroppers, the
   * same reason `sampleLayerColor` bypasses the preview for Levels'. A caller that wants what the
   * canvas is showing right now, preview included, should read the rendered canvas directly
   * (`readDocumentPixels` in the test API) rather than ask this to special-case it. */
  sampleColor(doc: string, at: { x: number; y: number }): [number, number, number] | null {
    const json = this.wasm.sample_color(doc, at.x, at.y);
    return json ? (JSON.parse(json) as [number, number, number]) : null;
  }
  /** The engine's `LayerAdjustment::is_identity`: Gradient Map never is, Grain only at amount 0,
   * and the channel and range selectors never count. */
  adjustmentIsIdentity(adjustment: LayerAdjustment): boolean { return this.wasm.adjustment_is_identity(JSON.stringify(adjustment)); }
  adjustmentLut(adjustment: LayerAdjustment): Uint8Array { return this.wasm.adjustment_lut(JSON.stringify(adjustment)); }
  hueResponse(adjustment: LayerAdjustment): Float32Array { return this.wasm.hue_response_table(JSON.stringify(adjustment)); }

  /** A blur adjustment's kernel in output pixels and its halving level (engine `spatial_blur`). */
  spatialBlur(adjustment: LayerAdjustment, outPerDoc: number): SpatialBlur { return JSON.parse(this.wasm.spatial_blur(JSON.stringify(adjustment), outPerDoc)) as SpatialBlur; }
  /** The halving lattice and pad for the document's plan at `outPerDoc` output px per document px (engine `spatial_grid`). */
  spatialGrid(doc: string, edit: PreviewEdit | null, outPerDoc: number): SpatialGrid {
    return JSON.parse(this.wasm.spatial_grid(doc, edit ? JSON.stringify(edit) : undefined, outPerDoc)) as SpatialGrid;
  }
  /** One axis of a render's working span, from the canvas's leading edge (engine `spatial_span`). */
  spatialSpan(near: number, far: number, canvas: number, grid: SpatialGrid): [number, number] {
    const s = this.wasm.spatial_span(near, far, canvas, grid.cell, grid.pad); return [s[0], s[1]];
  }
}
