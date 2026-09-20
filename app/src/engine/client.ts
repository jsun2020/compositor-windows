import init, { WasmEngine } from "./pkg/compositor_engine.js";
import type { Command, Dirty, DocumentState, PackageFiles } from "./types";

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
  importImage(doc: string | null, bytes: Uint8Array, name: string, at: { x: number; y: number } | null): string {
    return this.wasm.import_image(doc ?? undefined, bytes, name, at?.x, at?.y);
  }
  exportPng(doc: string): Uint8Array { return this.wasm.export_png(doc); }
  exportJpeg(doc: string, quality: number, matte: [number, number, number]): Uint8Array { return this.wasm.export_jpeg(doc, quality, ...matte); }
  composite(doc: string, region: { x: number; y: number; width: number; height: number }, outWidth: number, outHeight: number): Uint8Array {
    return this.wasm.composite(doc, region.x, region.y, region.width, region.height, outWidth, outHeight);
  }
  /** A view on wasm memory; valid only until the next engine call. */
  layerPixels(doc: string, layer: string): Uint8Array | null {
    const len = this.wasm.layer_pixels_len(doc, layer);
    if (len === 0) return null;
    const ptr = this.wasm.layer_pixels_ptr(doc, layer);
    return new Uint8Array(this.memory.buffer, ptr, len);
  }
}
