import type { PackageFiles } from "../engine/types";

export interface ShellBridge {
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
