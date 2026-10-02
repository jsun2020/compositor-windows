import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open, save } from "@tauri-apps/plugin-dialog";
import type { PackageFiles } from "../engine/types";
import { baseName, type ShellBridge } from "./bridge";

export class TauriBridge implements ShellBridge {
  readonly positionIsPhysical = true;
  async readClipboardImage(): Promise<{ bytes: Uint8Array; origin: [number, number] | null; layerToken: string | null }> {
    const buffer = await invoke<ArrayBuffer>("read_clipboard_image");
    if (buffer.byteLength <= 53) throw new Error("The clipboard does not contain an image");
    const view = new DataView(buffer);
    const origin: [number, number] | null = view.getUint8(0) ? [view.getFloat64(1, true), view.getFloat64(9, true)] : null;
    const token=new TextDecoder().decode(new Uint8Array(buffer,17,36));
    return { bytes: new Uint8Array(buffer, 53), origin, layerToken:token.includes("\0")?null:token };
  }
  async writeClipboardImage(bytes: Uint8Array, origin: [number, number], layerToken?:string): Promise<void> {
    await invoke("write_clipboard_image", bytes, { headers: { origin: JSON.stringify(origin),...(layerToken?{"layer-token":layerToken}:{}) } });
  }
  async pickOpenPackage(): Promise<string | null> {
    const picked = await open({ directory: true, multiple: false, title: "Open Compositor Project (.comp folder)" });
    return typeof picked === "string" ? picked : null;
  }
  async pickSavePackage(suggested: string): Promise<string | null> {
    const picked = await save({ defaultPath: suggested.endsWith(".comp") ? suggested : `${suggested}.comp`, filters: [{ name: "Compositor Project", extensions: ["comp"] }] });
    return picked ? (picked.endsWith(".comp") ? picked : `${picked}.comp`) : null;
  }
  async pickImportImages(): Promise<string[]> {
    const picked = await open({ multiple: true, filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "tif", "tiff", "webp", "bmp"] }] });
    return Array.isArray(picked) ? picked : picked ? [picked] : [];
  }
  async pickExportFile(suggested: string, ext: "png" | "jpg"): Promise<string | null> {
    return (await save({ defaultPath: `${suggested}.${ext}`, filters: [{ name: ext.toUpperCase(), extensions: ext === "jpg" ? ["jpg", "jpeg"] : ["png"] }] })) ?? null;
  }
  async readPackage(path: string): Promise<PackageFiles> {
    const header = await invoke<{ manifest: string; image_names: string[] }>("read_package_manifest", { path });
    const images = [];
    for (const name of header.image_names) {
      const bytes = await invoke<ArrayBuffer>("read_package_image", { path, name });
      images.push({ name, bytes: new Uint8Array(bytes) });
    }
    return { manifest: header.manifest, images };
  }
  async writePackage(path: string, files: PackageFiles): Promise<void> {
    const token = await invoke<string>("write_package_begin", { path });
    try {
      await invoke("write_package_manifest", { token, manifest: files.manifest });
      for (const image of files.images) {
        await invoke("write_package_image", image.bytes, { headers: { token, name: encodeURIComponent(image.name) } });
      }
      await invoke("write_package_commit", { token });
    } catch (e) { await invoke("write_package_abort", { token }); throw e; }
  }
  async readFile(path: string): Promise<Uint8Array> { return new Uint8Array(await invoke<ArrayBuffer>("read_file", { path })); }
  async writeFile(path: string, bytes: Uint8Array): Promise<void> { await invoke("write_file", bytes, { headers: { path: encodeURIComponent(path) } }); }
  recentPackages(): Promise<string[]> { return invoke<string[]>("recent_packages"); }
  async addRecentPackage(path: string): Promise<void> { await invoke("add_recent_package", { path }); }
  onFileDrop(handler: (paths: string[], position: { x: number; y: number } | null) => void): () => void {
    let unlisten: (() => void) | null = null;
    let cancelled = false;
    getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type === "drop") handler(event.payload.paths, event.payload.position ? { x: event.payload.position.x, y: event.payload.position.y } : null);
    }).then((u) => {
      // The caller may have unsubscribed before this promise settled; if so, don't
      // leak the listener, unlisten immediately instead of stashing it for later.
      if (cancelled) { u(); } else { unlisten = u; }
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }
  baseName(path: string): string { return baseName(path); }
}
