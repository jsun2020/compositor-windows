/** Exact premultiplied bytes, with a bounded palette; photographs fall back to RGBA. */
export const MAX_PIXEL_PALETTE = 1024;
const LITTLE_ENDIAN = new Uint8Array(new Uint32Array([1]).buffer)[0] === 1;

export function encodePalettePixels(source: Uint8Array): { palette: number[]; indices: ArrayBuffer } | null {
  if (!source.length || source.length % 4) return null;
  const count = source.length / 4;
  const words = LITTLE_ENDIAN && source.byteOffset % 4 === 0 ? new Uint32Array(source.buffer, source.byteOffset, count) : null;
  const view = new DataView(source.buffer, source.byteOffset, source.byteLength);
  const word = (i: number) => words ? words[i] : view.getUint32(i * 4, true);
  const indices = new Map<number, number>(), palette: number[] = [];
  let previous = -1;
  // Establish the complete palette before allocating a full index plane. A
  // result with too many colours exits early and keeps the existing raw path.
  for (let i = 0; i < count; i++) {
    const value = word(i);
    if (value !== previous && !indices.has(value)) {
      if (indices.size === MAX_PIXEL_PALETTE) return null;
      indices.set(value, indices.size);
      palette.push(...source.subarray(i * 4, i * 4 + 4));
    }
    previous = value;
  }
  const depth = indices.size <= 256 ? 1 : 2;
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
  if (!Array.isArray(palette) || !palette.length || palette.length % 4 || palette.length > MAX_PIXEL_PALETTE * 4
      || !palette.every(c => Number.isInteger(c) && c >= 0 && c <= 255) || !pixels) throw Error("Invalid palette job pixels");
  const size = palette.length / 4, depth = size <= 256 ? 1 : 2;
  if (pixels.byteLength !== count * depth) throw Error("Invalid palette job pixels");
  return { bytes: Uint8Array.from(palette), depth };
}
