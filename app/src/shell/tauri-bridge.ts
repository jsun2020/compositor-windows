import {parseNativeCmyk} from "./native-cmyk";
import {parseNativeImages} from "./native-images";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open, save } from "@tauri-apps/plugin-dialog";
import type { PackageFiles } from "../engine/types";
import { baseName, type ShellBridge, type RawInfo,type RawDevelopSettings } from "./bridge";

export class TauriBridge implements ShellBridge {
  async importCmykFile(path:string,profile:string|null){return parseNativeCmyk(await invoke<ArrayBuffer>("import_cmyk_file",{path,profile}));}
  listColorProfiles():Promise<import("../canvas/color-proof").ColorProfile[]> {return invoke("list_color_profiles");}
  async pickColorProfile():Promise<string|null> {const path=await open({multiple:false,filters:[{name:"CMYK ICC profile",extensions:["icc","icm"]}]});return typeof path==="string"?path:null;}
  proofColorProfile(path:string,importProfile:boolean):Promise<{profile:import("../canvas/color-proof").ColorProfile;lut:number[]}> {return invoke("proof_color_profile",{path,import:importProfile});}
  async exportCmyk(png:Uint8Array,profile:string,format:"JPEG"|"TIFF"|"PDF",quality:number,dpi:number):Promise<Uint8Array> {return new Uint8Array(await invoke<ArrayBuffer>("export_cmyk",png,{headers:{"color-options":encodeURIComponent(JSON.stringify({profile,format,quality,dpi}))}}));}

  async decodeNativeImage(path:string,fitting:[number,number]|null,remaining:number){return parseNativeImages(await invoke<ArrayBuffer>("decode_native_image",{path,fitting,remaining}));}
  private fonts:Promise<string[]>|null=null;
  listFonts():Promise<string[]> {return this.fonts??=invoke<string[]>("list_fonts").catch(error=>{this.fonts=null;throw error;});}
  async setFullscreen(value:boolean):Promise<void> { const {getCurrentWindow}=await import("@tauri-apps/api/window");await getCurrentWindow().setFullscreen(value); }
  rawInspect(path:string,token:string):Promise<RawInfo> {return invoke("raw_inspect",{path,token});}
  async rawDevelop(path:string,settings:RawDevelopSettings,preview:boolean,token:string):Promise<Uint8Array> {return new Uint8Array(await invoke<ArrayBuffer>("raw_develop",{path,settings,preview,token}));}
  rawCancel(token:string):Promise<void> {return invoke("raw_cancel",{token});}
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
    // Origin is a browser-controlled HTTP header on Tauri's fetch IPC path.
    await invoke("write_clipboard_image", bytes, { headers: { "compositor-pixel-origin": JSON.stringify(origin),...(layerToken?{"layer-token":layerToken}:{}) } });
  }
  async pickOpenPackage(): Promise<string | null> {
    const picked = await open({ directory: true, multiple: false, title: "Open Compositor Project (.comp folder)" });
    return typeof picked === "string" ? picked : null;
  }
  async pickSavePackage(suggested: string): Promise<string | null> {
    const picked = await save({ defaultPath: suggested.endsWith(".comp") ? suggested : `${suggested}.comp`, filters: [{ name: "Compositor Project", extensions: ["comp"] }] });
    return picked ? (picked.endsWith(".comp") ? picked : `${picked}.comp`) : null;
  }
  async pickImportFolder():Promise<string|null> {const path=await open({directory:true,multiple:false});return typeof path==="string"?path:null;}
  expandImportPaths(paths:string[]):Promise<{paths:string[];errors:string[]}>{return invoke("expand_import_paths",{paths});}
  async pickImportImages(): Promise<string[]> {
    const picked = await open({ multiple: true, filters: [{ name: "Images, Photoshop and Camera RAW", extensions: ["png", "gif", "pdf", "heic", "heif", "hif", "jpg", "jpeg", "tif", "tiff", "exr", "hdr", "webp", "bmp", "psd", "psb", "dng", "cr2", "cr3", "crw", "nef", "nrw", "arw", "srf", "sr2", "raf", "rw2", "orf", "pef", "ptx", "raw", "rwl", "srw", "3fr", "fff", "iiq", "kdc", "dcr", "mos", "mef", "erf", "x3f"] }] });
    return Array.isArray(picked) ? picked : picked ? [picked] : [];
  }
  async pickExportFile(suggested: string, ext: "png" | "jpg" | "pdf" | "exr" | "tif" | "gif"): Promise<string | null> {
    return (await save({ defaultPath: `${suggested}.${ext}`, filters: [{ name: ext.toUpperCase(), extensions: ext === "jpg" ? ["jpg", "jpeg"] : [ext] }] })) ?? null;
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
