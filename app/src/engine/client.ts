import init, { WasmEngine } from "./pkg/compositor_engine.js";
import {releaseJobResult} from "./jobs";
import type { Command, Dirty, DocumentState, LayerAdjustment, LayerTransform, LevelsAuto, LevelsSample, LevelsSettings, PackageFiles, PixelRect, PreviewEdit, PreviewRequest, RenderPlan, SpatialBlur, SpatialGrid } from "./types";

/** `[x, y, width, height]` from the engine as a rectangle; an empty array as null (take it whole). */
function rectOf(v: ArrayLike<number>): PixelRect | null { return v.length === 4 ? { x: v[0], y: v[1], width: v[2], height: v[3] } : null; }

/** A job's input copied out of wasm memory (engine `job_input`): the JSON header, the layer's pixel
 * and mask bytes, and its selection's points (if any), each its own buffer, ready to transfer to the
 * job worker. `points` is null exactly when the input has no selection; a selection with nothing in
 * it is still a (zero-length) buffer, never null (jobs.ts explains why the distinction matters). */
export interface JobInputCopy { input: string; pixels: ArrayBuffer | null; mask: ArrayBuffer | null; points: ArrayBuffer | null; }

export class EngineClient {
  lastJobInputCpuMs=0;
  lastInstallCpuMs=0;
  private installQueue:Promise<void>=Promise.resolve();
  private constructor(private readonly wasm: WasmEngine, private readonly memory: WebAssembly.Memory, readonly module: WebAssembly.Module) {}

  /** Compiles the engine's wasm once and instantiates it; the compiled module is kept for the job
   * worker, which instantiates a second engine from it (`JobClient`). */
  static async load(): Promise<EngineClient> {
    const response = await fetch(new URL("./pkg/compositor_engine_bg.wasm", import.meta.url));
    // A missing or refused file is said as such, not as the compile error its error page would raise
    // (final review minor 7).
    if (!response.ok) {
      const status = response.statusText ? `${response.status} ${response.statusText}` : String(response.status);
      throw new Error(`The engine could not be loaded (${status}).`);
    }
    const module = await WebAssembly.compile(await response.arrayBuffer());
    const exports = await init({ module_or_path: module });
    return new EngineClient(new WasmEngine(), exports.memory, module);
  }

  version(): string { return this.wasm.version(); }
  /** The engine's wasm memory in bytes. It only grows; the perf harness reads its high-water mark. */
  wasmBytes(): number { return this.memory.buffer.byteLength; }
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
  beginFloating(doc: string, layer: string, duplicate: boolean): string { return this.wasm.begin_floating(doc, layer, duplicate); }
  cancelFloating(doc: string): void { this.wasm.cancel_floating(doc); }
  commitFloating(doc: string, transform: LayerTransform, corners: import("./types").Corners | null): Dirty {
    return JSON.parse(this.wasm.commit_floating(doc, JSON.stringify(transform), corners ? JSON.stringify(corners) : undefined)) as Dirty;
  }
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
  /** The layer's stored pixel count (`Engine::stored_pixels`): `state()` shows a previewed layer at the
   * preview's size, which may be a reduced copy. */
  storedPixels(doc: string, layer: string): number { return this.wasm.stored_pixels(doc, layer); }
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
  /** Copies the kept job buffers out of wasm memory (pixels, mask, then the selection's points, in
   * that order: each pointer call can grow memory, which detaches any buffer already read, so every
   * buffer is sliced out immediately after its own pointer call, before the next one), then lets the
   * engine drop them (`release_job`). `input` is `prepare_job` / `prepare_display_job`'s JSON, whose
   * `selection` field decides whether `points` is a (possibly empty) buffer or null: `job_points_len`
   * alone cannot tell an empty selection from no selection at all. */
  private takeJob(input: string): JobInputCopy {
    try {
      const copy = (mask: boolean) => {
        const len = this.wasm.job_buffer_len(mask);
        if (len === 0) return null;
        const ptr = this.wasm.job_buffer_ptr(mask);
        return new Uint8Array(this.memory.buffer, ptr, len).slice().buffer;
      };
      const pixels = copy(false);
      const mask = copy(true);
      const hasSelection = (JSON.parse(input) as { selection?: unknown }).selection != null;
      let points: ArrayBuffer | null = null;
      if (hasSelection) {
        const len = this.wasm.job_points_len();
        const ptr = this.wasm.job_points_ptr();
        points = new Uint8Array(this.memory.buffer, ptr, len).slice().buffer;
      }
      return { input, pixels, mask, points };
    } finally { this.wasm.release_job(); }
  }
  /** A job's input for `layer` (engine `job_input`): the stored layer, never a preview. */
  jobInput(doc: string, layer: string): JobInputCopy { return this.takeJob(this.wasm.prepare_job(doc, layer)); }
  jobHeader(doc: string, layer: string): string {
    try { return this.wasm.prepare_job(doc, layer); } finally { this.wasm.release_job(); }
  }
  private async copyStored(doc:string,layer:string,mask:boolean,size:number,cpu?:{value:number}):Promise<ArrayBuffer|null>{
    if(!size)return null;
    const allocated=performance.now(),out=new Uint8Array(size);let start=performance.now();if(cpu)cpu.value+=start-allocated;
    for(let at=0;at<size;at+=4*1024*1024){
      const t=performance.now(),length=Math.min(4*1024*1024,size-at),ptr=this.wasm.stored_buffer_ptr(doc,layer,mask);
      out.set(new Uint8Array(this.memory.buffer,ptr+at,length),at);
      if(cpu)cpu.value+=performance.now()-t;
      if(performance.now()-start>=8&&at+length<size){await new Promise<void>(r=>requestAnimationFrame(()=>r()));start=performance.now();}
    }
    return out.buffer;
  }
  jobInputAsync(doc:string,layer:string):JobInputCopy|Promise<JobInputCopy>{
    const started=performance.now();
    const input=this.wasm.prepare_job(doc,layer);let points:ArrayBuffer|null=null;
    const h=JSON.parse(input) as {pixels:[number,number]|null;mask:[number,number]|null;selection:unknown};
    const pixelSize=h.pixels?h.pixels[0]*h.pixels[1]*4:0,maskSize=h.mask?h.mask[0]*h.mask[1]:0;
    if(pixelSize+maskSize<=4*1024*1024){const result=this.takeJob(input);this.lastJobInputCpuMs=performance.now()-started;return result;}
    try{if(h.selection!=null){const len=this.wasm.job_points_len(),ptr=this.wasm.job_points_ptr();points=new Uint8Array(this.memory.buffer,ptr,len).slice().buffer;}}
    finally{this.wasm.release_job();}
    const cpu={value:performance.now()-started};
    return(async()=>{const pixels=await this.copyStored(doc,layer,false,pixelSize,cpu);
    const mask=await this.copyStored(doc,layer,true,maskSize,cpu);this.lastJobInputCpuMs=cpu.value;
    return{input,pixels,mask,points};})();
  }
  clipboardInput(doc: string, layer: string | null, mask: boolean, merged: boolean): { input: string; layers: { pixels: ArrayBuffer | null; mask: ArrayBuffer | null }[]; points: ArrayBuffer | null } {
    const snapshot = this.takeJob(this.wasm.prepare_clipboard(doc, layer ?? undefined, mask, merged));
    const header = JSON.parse(snapshot.input) as { layers: { record: { id: string } }[] };
    const layers = header.layers.map(({ record }) => {
      const pixels = this.layerPixels(doc, record.id)?.slice().buffer ?? null;
      const mask = this.maskPixels(doc, record.id)?.slice().buffer ?? null;
      return { pixels, mask };
    });
    return { input: snapshot.input, layers, points: snapshot.points };
  }
  async clipboardInputAsync(doc:string,layer:string|null,mask:boolean,merged:boolean):Promise<ReturnType<EngineClient["clipboardInput"]>>{
    const snapshot=this.takeJob(this.wasm.prepare_clipboard(doc,layer??undefined,mask,merged));
    const header=JSON.parse(snapshot.input) as {layers:{record:{id:string};pixels:[number,number]|null;mask:[number,number]|null}[]};
    const layers:ReturnType<EngineClient["clipboardInput"]>["layers"]=[];
    for(const l of header.layers)layers.push({pixels:await this.copyStored(doc,l.record.id,false,l.pixels?l.pixels[0]*l.pixels[1]*4:0),mask:await this.copyStored(doc,l.record.id,true,l.mask?l.mask[0]*l.mask[1]:0)});
    return{input:snapshot.input,layers,points:snapshot.points};
  }
  pastePixels(doc: string, width: number, height: number, pixels: ArrayBuffer, origin: [number, number] | null, keepSelection = false): Dirty {
    return JSON.parse(this.wasm.paste_pixels(doc, width, height, new Uint8Array(pixels), origin?.[0], origin?.[1], keepSelection)) as Dirty;
  }
  pasteCopiedLayers(doc:string,snapshot:ReturnType<EngineClient["clipboardInput"]>):Dirty{
    return JSON.parse(this.wasm.paste_copied_layers(doc,snapshot.input,snapshot.layers.map(l=>l.pixels?new Uint8Array(l.pixels):null),snapshot.layers.map(l=>l.mask?new Uint8Array(l.mask):null))) as Dirty;
  }
  /** An effects job's input (engine `display_job_input`): the layer as the canvas shows it, its pixels
   * after `level` halvings. */
  displayJobInput(doc: string, layer: string, level: number): JobInputCopy { return this.takeJob(this.wasm.prepare_display_job(doc, layer, level)); }
  /** Puts an edit job's result back (engine `install_job`), only onto the layer exactly as the job took
   * it (its stamp, read from `input`); a changed layer refuses with the engine's message. `display` is
   * the result halved to the canvas's level, which the new pixels adopt (F1). */
  installJob(doc: string, layer: string, input: string, output: string, pixels: ArrayBuffer | null, mask: ArrayBuffer | null, display: ArrayBuffer | null = null): Dirty {
    const stamp = JSON.stringify((JSON.parse(input) as { stamp: unknown }).stamp);
    const view = (b: ArrayBuffer | null) => (b ? new Uint8Array(b) : undefined);
    return JSON.parse(this.wasm.install_job(doc, layer, stamp, output, view(pixels), view(mask), view(display))) as Dirty;
  }
  async installJobAsync(doc:string,layer:string,input:string,output:string,pixels:ArrayBuffer|null,mask:ArrayBuffer|null,display:ArrayBuffer|null=null,preview=false,valid:()=>boolean=()=>true):Promise<Dirty>{
    // One small FFI copy is cheaper than staging; keep the established install path.
    if(!preview&&(pixels?.byteLength??0)+(mask?.byteLength??0)+(display?.byteLength??0)<=4*1024*1024){if(!valid())throw Error("The preview was cancelled");const t=performance.now(),dirty=this.installJob(doc,layer,input,output,pixels,mask,display);this.lastInstallCpuMs=performance.now()-t;return dirty;}
    const before=this.installQueue;let release!:()=>void;this.installQueue=new Promise<void>(r=>{release=r;});await before;
    try{
      const dirty=await this.stageJob(doc,layer,input,output,pixels,mask,display,preview,valid);
      // A committed large result has moved into WASM. Retire its transferred
      // buffers before callers draw the first frame; previews still need them.
      if(!preview){const t=performance.now();releaseJobResult({header:output,pixels,mask,display});this.lastInstallCpuMs+=performance.now()-t;}
      return dirty;
    }finally{release();}
  }
  private async stageJob(doc:string,layer:string,input:string,output:string,pixels:ArrayBuffer|null,mask:ArrayBuffer|null,display:ArrayBuffer|null,preview:boolean,valid:()=>boolean):Promise<Dirty>{
    const began=performance.now();this.lastInstallCpuMs=0;
    if(!valid())throw Error("The preview was cancelled");
    const stamp=JSON.stringify((JSON.parse(input) as {stamp:unknown}).stamp),buffers=[pixels,mask,display];
    this.wasm.begin_staged_install(pixels?.byteLength??0,mask?.byteLength??0,display?.byteLength??0);
    this.lastInstallCpuMs=performance.now()-began;
    try{
      let start=performance.now();
      for(let plane=0;plane<buffers.length;plane++){const b=buffers[plane];if(!b)continue;
        for(let at=0;at<b.byteLength;at+=4*1024*1024){const t=performance.now(),length=Math.min(4*1024*1024,b.byteLength-at);this.wasm.append_staged_install(plane,new Uint8Array(b,at,length));this.lastInstallCpuMs+=performance.now()-t;
          if(!valid())throw Error("The preview was cancelled");
          if(performance.now()-start>=8){await new Promise<void>(r=>requestAnimationFrame(()=>r()));start=performance.now();}
        }
      }
      if(!valid())throw Error("The preview was cancelled");const t=performance.now(),dirty=JSON.parse(this.wasm.finish_staged_install(doc,layer,stamp,output,preview)) as Dirty;this.lastInstallCpuMs+=performance.now()-t;return dirty;
    }finally{this.wasm.cancel_staged_install();}
  }
  keepJobPreview(doc:string,layer:string,input:string,output:string,pixels:ArrayBuffer,mask:ArrayBuffer|null,display:ArrayBuffer|null=null):Dirty{
    const stamp=JSON.stringify((JSON.parse(input) as {stamp:unknown}).stamp);
    return JSON.parse(this.wasm.keep_job_preview(doc,layer,stamp,output,new Uint8Array(pixels),mask?new Uint8Array(mask):undefined,display?new Uint8Array(display):undefined)) as Dirty;
  }
  textStamp(doc:string,layer:string):string {try{return JSON.stringify((JSON.parse(this.wasm.prepare_job(doc,layer)) as {stamp:unknown}).stamp);}finally{this.wasm.release_job();}}
  installText(doc:string,layer:string|null,style:import("../tools/text-style").TextStyle,width:number,height:number,pixels:ArrayBuffer,origin:[number,number],stamp:string|null):Dirty {
    return JSON.parse(this.wasm.install_text(doc,layer??undefined,JSON.stringify(style),width,height,new Uint8Array(pixels),origin[0],origin[1],stamp??undefined)) as Dirty;
  }
  keepTextPreview(doc:string,layer:string,style:import("../tools/text-style").TextStyle,width:number,height:number,pixels:ArrayBuffer,stamp:string):void {
    this.wasm.keep_text_preview(doc,layer,JSON.stringify(style),width,height,new Uint8Array(pixels),stamp);
  }
  /** Whether the canvas's effects image for `layer` is made already (engine `has_effects_image`):
   * `drawPixels` then hands it over without making it. */
  hasEffectsImage(doc: string, layer: string, edit: PreviewEdit | null): boolean {
    return this.wasm.has_effects_image(doc, layer, edit ? JSON.stringify(edit) : undefined);
  }
  /** Keeps a full-size effects image the job worker made (engine `keep_effects_image`), when the
   * layer's pixels and mask are still what the job took (the stamp in `input`) and its current
   * `EffectsDraw` still equals `key` (the engine's own `EffectsDraw.key` the job was asked to draw,
   * not `EffectsImages`'s own compound `bytesKey`; fix round 1, issue 2); false when it is not kept. */
  keepEffectsImage(doc: string, layer: string, input: string, key: string, edit: PreviewEdit | null, width: number, height: number, bytes: ArrayBuffer): boolean {
    const stamp = JSON.stringify((JSON.parse(input) as { stamp: unknown }).stamp);
    return this.wasm.keep_effects_image(doc, layer, stamp, key, edit ? JSON.stringify(edit) : undefined, width, height, new Uint8Array(bytes));
  }
  /** What changed in the layer's pixels since revision `from` (engine `pixels_delta`): a rectangle of
   * its pixel grid, empty when nothing did, or null when the whole raster must be uploaded again. */
  pixelsDelta(doc: string, layer: string, from: number): PixelRect | null { return rectOf(this.wasm.pixels_delta(doc, layer, from)); }
  /** The bytes of `rect` of the layer's pixels after `level` halvings, as the canvas shows them
   * (engine `layer_region`): a copy, for a partial upload. */
  layerRegion(doc: string, layer: string, level: number, rect: PixelRect): Uint8Array {
    return this.wasm.layer_region(doc, layer, level, rect.x, rect.y, rect.width, rect.height);
  }
  /** `pixelsDelta` for the layer's mask, in the mask's own grid (engine `mask_delta`). */
  maskDelta(doc: string, layer: string, from: number): PixelRect | null { return rectOf(this.wasm.mask_delta(doc, layer, from)); }
  /** The pixels a Fill or a Gradient on the layer, or its mask, paints (engine `edit_pixels`, ruling C1):
   * what decides the job worker. Throws where the edit is refused for its size. */
  editPixels(doc: string, layer: string, mask: boolean): number { return this.wasm.edit_pixels(doc, layer, mask); }
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
  /** The selection's outline for the marching ants, in document pixels, flat as the engine lays it
   * out: the number of contours, then for each its number of points and their x, y (read it with
   * `traceOutline`, canvas/ants.ts); empty with no selection. `step` is screen pixels per document
   * pixel, a power of two: below 1 a very detailed outline comes back traced at that resolution
   * (engine `selection_lod`). Kept flat: a four-million-point outline as tuple arrays cost more than
   * the engine's work (final review F3). */
  selectionOutline(doc: string, step: number): Float64Array {
    return this.wasm.selection_outline(doc, step);
  }
  /** Whether `at` lies inside a selection with something in it (engine `selection_contains`). */
  selectionContains(doc: string, at: { x: number; y: number }): boolean { return this.wasm.selection_contains(doc, at.x, at.y); }
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
