import type { PackageFiles } from "../engine/types";
import { baseName, type ShellBridge } from "./bridge";

/** In-memory shell for browser tests. */
export class MockBridge implements ShellBridge {
  private files = new Map<string, Uint8Array>();
  private packages = new Map<string, PackageFiles>();
  private picks: (string | null)[] = [];
  private dropHandlers: ((paths: string[], position: { x: number; y: number } | null) => void)[] = [];
  private recent: string[] = [];
  private failWriteMessage: string | null = null;

  setNextPick(path: string | null): void { this.picks.push(path); }
  seedFile(path: string, bytes: Uint8Array): void { this.files.set(path, bytes); }
  simulateDrop(paths: string[], position: { x: number; y: number } | null): void { for (const h of this.dropHandlers) h(paths, position); }
  hasPackage(path: string): boolean { return this.packages.has(path); }
  fileBytes(path: string): Uint8Array | undefined { return this.files.get(path); }
  /** The next `writePackage` or `writeFile` call rejects with this message; the flag is
   * then cleared, so subsequent writes succeed normally. */
  failNextWrite(message: string): void { this.failWriteMessage = message; }
  private checkFailWrite(): void {
    if (this.failWriteMessage === null) return;
    const message = this.failWriteMessage;
    this.failWriteMessage = null;
    throw new Error(message);
  }

  private nextPick(): string | null { return this.picks.length ? this.picks.shift()! : null; }
  async pickOpenPackage(): Promise<string | null> { return this.nextPick(); }
  async pickSavePackage(): Promise<string | null> { return this.nextPick(); }
  async pickImportImages(): Promise<string[]> { const p = this.nextPick(); return p ? [p] : []; }
  async pickExportFile(): Promise<string | null> { return this.nextPick(); }
  async readPackage(path: string): Promise<PackageFiles> {
    const p = this.packages.get(path); if (!p) throw new Error(`${path} is not a Compositor project folder.`);
    return { manifest: p.manifest, images: p.images.map((i) => ({ name: i.name, bytes: new Uint8Array(i.bytes) })) };
  }
  async writePackage(path: string, files: PackageFiles): Promise<void> {
    this.checkFailWrite();
    this.packages.set(path, { manifest: files.manifest, images: files.images.map((i) => ({ name: i.name, bytes: new Uint8Array(i.bytes) })) });
  }
  async readFile(path: string): Promise<Uint8Array> { const f = this.files.get(path); if (!f) throw new Error(`Cannot read ${path}`); return new Uint8Array(f); }
  async writeFile(path: string, bytes: Uint8Array): Promise<void> {
    this.checkFailWrite();
    this.files.set(path, new Uint8Array(bytes));
  }
  async recentPackages(): Promise<string[]> { return [...this.recent]; }
  async addRecentPackage(path: string): Promise<void> { this.recent = [path, ...this.recent.filter((p) => p !== path)].slice(0, 10); }
  onFileDrop(handler: (paths: string[], position: { x: number; y: number } | null) => void): () => void {
    this.dropHandlers.push(handler);
    return () => { this.dropHandlers = this.dropHandlers.filter((h) => h !== handler); };
  }
  baseName(path: string): string { return baseName(path); }
}
