/** Exact premultiplied bytes, with a bounded palette; photographs fall back to RGBA. */
export const MAX_PIXEL_PALETTE = 1024;
const LITTLE_ENDIAN = new Uint8Array(new Uint32Array([1]).buffer)[0] === 1;

export function encodePalettePixels(source: Uint8Array, minimumRunPixels = 1): { palette: number[]; indices: ArrayBuffer; runs?: true } | null {
  if (!source.length || source.length % 4) return null;
  const count = source.length / 4;
  const words = LITTLE_ENDIAN && source.byteOffset % 4 === 0 ? new Uint32Array(source.buffer, source.byteOffset, count) : null;
  const view = new DataView(source.buffer, source.byteOffset, source.byteLength);
  const word = (i: number) => words ? words[i] : view.getUint32(i * 4, true);
  const indices = new Map<number, number>(), palette: number[] = [];
  let previous = -1, runs = 0;
  const runLimit = count / minimumRunPixels;
  // Establish the complete palette before allocating a full index plane. A
  // result with too many colours exits early and keeps the existing raw path.
  for (let i = 0; i < count; i++) {
    const value = word(i);
    if (value !== previous) {
      // Large results with short colour bands are cheaper to transfer as raw
      // RGBA into the existing buffer than to expand pixel indices on the UI.
      if (++runs > runLimit) return null;
      if (!indices.has(value)) {
        if (indices.size === MAX_PIXEL_PALETTE) return null;
        indices.set(value, indices.size);
        palette.push(...source.subarray(i * 4, i * 4 + 4));
      }
    }
    previous = value;
  }
  const depth = indices.size <= 256 ? 1 : 2;
  // A large result with very long flat bands need not transfer one index for
  // every pixel. Each little-endian record is [palette index, pixel count].
  // Keep ordinary indices for shorter bands and existing small callers.
  if (minimumRunPixels >= 64 && runs <= count / 4096) {
    const output = new ArrayBuffer(runs * 8), records = new DataView(output);
    let start = 0, record = 0;
    previous = word(0);
    for (let i = 1; i <= count; i++) {
      const value = i < count ? word(i) : -1;
      if (value === previous) continue;
      records.setUint32(record, indices.get(previous)!, true);
      records.setUint32(record + 4, i - start, true);
      record += 8; start = i; previous = value;
    }
    return { palette, indices: output, runs: true };
  }
  const output = new Uint8Array(count * depth), outputView = new DataView(output.buffer);
  const wide = depth === 2 && LITTLE_ENDIAN ? new Uint16Array(output.buffer) : null;
  previous = -1; let index = 0;
  for (let i = 0; i < count; i++) {
    const value = word(i);
    if (value !== previous) { index = indices.get(value)!; previous = value; }
    if (depth === 1) output[i] = index;
    else if (wide) wide[i] = index;
    else outputView.setUint16(i * 2, index, true);
  }
  return { palette, indices: output.buffer };
}

export function validatePalettePixels(palette: unknown, pixels: ArrayBuffer | null, count: number): { bytes: Uint8Array; depth: 1 | 2 } {
  const bytes = validatePalette(palette);
  if (!pixels) throw Error("Invalid palette job pixels");
  const size = bytes.length / 4, depth = size <= 256 ? 1 : 2;
  if (pixels.byteLength !== count * depth) throw Error("Invalid palette job pixels");
  return { bytes, depth };
}

function validatePalette(palette: unknown): Uint8Array {
  if (!Array.isArray(palette) || !palette.length || palette.length % 4 || palette.length > MAX_PIXEL_PALETTE * 4
      || !palette.every(c => Number.isInteger(c) && c >= 0 && c <= 255)) throw Error("Invalid palette job pixels");
  return Uint8Array.from(palette);
}

/** Validate every record and the complete output size before reserving WASM. */
export function validatePaletteRuns(palette: unknown, pixels: ArrayBuffer | null, count: number): { bytes: Uint8Array; records: DataView } {
  const bytes = validatePalette(palette);
  if (!Number.isInteger(count) || count <= 0 || count > 100_000_000
      || !pixels || !pixels.byteLength || pixels.byteLength % 8) throw Error("Invalid palette job runs");
  const records = new DataView(pixels), colors = bytes.length / 4;
  let total = 0;
  for (let at = 0; at < records.byteLength; at += 8) {
    const index = records.getUint32(at, true), length = records.getUint32(at + 4, true);
    if (index >= colors || length === 0 || length > count - total) throw Error("Invalid palette job runs");
    total += length;
  }
  if (total !== count) throw Error("Invalid palette job runs");
  return { bytes, records };
}
