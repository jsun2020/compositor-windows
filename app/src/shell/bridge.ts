import type { PackageFiles } from "../engine/types";
export interface RawInfo {decoder:string;width:number;height:number;make:string;model:string;asShotTemperature:number;asShotTint:number;whiteBalanceEstimate:boolean;sensorDecoded:boolean}
export interface RawDevelopSettings {exposure:number;temperature:number;tint:number;tone:number}

export interface ShellBridge {
  rawInspect?(path:string,token:string):Promise<RawInfo>;
  rawDevelop?(path:string,settings:RawDevelopSettings,preview:boolean,token:string):Promise<Uint8Array>;
  rawCancel?(token:string):Promise<void>;
  readClipboardImage(): Promise<{ bytes: Uint8Array; origin: [number, number] | null; layerToken?: string | null }>;
  writeClipboardImage(bytes: Uint8Array, origin: [number, number], layerToken?: string): Promise<void>;
  /** True when `onFileDrop` positions are physical pixels relative to the window (Tauri);
   * false when they are already CSS pixels (the mock bridge used in tests). */
  readonly positionIsPhysical: boolean;
  pickOpenPackage(): Promise<string | null>;
  pickSavePackage(suggested: string): Promise<string | null>;
  pickImportImages(): Promise<string[]>;
  pickExportFile(suggested: string, ext: "png" | "jpg"): Promise<string | null>;
  readPackage(path: string): Promise<PackageFiles>;
  writePackage(path: string, files: PackageFiles): Promise<void>;
  readFile(path: string): Promise<Uint8Array>;
  writeFile(path: string, bytes: Uint8Array): Promise<void>;
  recentPackages(): Promise<string[]>;
  addRecentPackage(path: string): Promise<void>;
  onFileDrop(handler: (paths: string[], position: { x: number; y: number } | null) => void): () => void;
  baseName(path: string): string;
}

export function baseName(path: string): string {
  const last = path.split(/[\\/]/).filter(Boolean).pop() ?? "";
  return last.replace(/\.[^.]+$/, "");
}

let bridge: ShellBridge | null = null;
export async function getBridge(): Promise<ShellBridge> {
  if (bridge) return bridge;
  const isTauri = "__TAURI_INTERNALS__" in window && import.meta.env.VITE_BRIDGE !== "mock";
  bridge = isTauri ? new (await import("./tauri-bridge")).TauriBridge() : new (await import("./mock-bridge")).MockBridge();
  return bridge;
}
