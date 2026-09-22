import type { Page } from "@playwright/test";

/**
 * Opens a top-level dropdown menu ("File", "Edit", "Image", "View") and clicks one of
 * its items. The menu bar mounts each title's items only while that title's dropdown
 * is open, so a bare `getByTestId("menu-<id>").click()` finds nothing until the title
 * has been clicked first.
 */
export async function clickMenu(page: Page, title: "File" | "Edit" | "Layer" | "Image" | "View", id: string): Promise<void> {
  await page.getByRole("button", { name: title, exact: true }).click();
  await page.getByTestId(`menu-${id}`).click();
}

/**
 * Draws a 2x2 fully-opaque red square on a canvas and returns its PNG bytes as base64,
 * decoded in-browser so tests never depend on a hand-verified base64 literal.
 *
 * Must be run via `page.evaluate(redSquarePngBase64)` (as a standalone page function, not
 * called from inside another inline evaluate callback): Playwright serializes a page
 * function by its source text alone, so an imported helper called from within another
 * function's body would not be defined in the browser. Returning base64 (a plain string)
 * also sidesteps any uncertainty about how typed arrays cross the Node/browser boundary.
 */
export async function redSquarePngBase64(): Promise<string> {
  const canvas = document.createElement("canvas");
  canvas.width = 2;
  canvas.height = 2;
  const ctx = canvas.getContext("2d")!;
  ctx.fillStyle = "#ff0000";
  ctx.fillRect(0, 0, 2, 2);
  const blob = await new Promise<Blob>((resolve, reject) => {
    canvas.toBlob((b) => (b ? resolve(b) : reject(new Error("toBlob failed"))), "image/png");
  });
  const bytes = new Uint8Array(await blob.arrayBuffer());
  let binary = "";
  for (const b of bytes) binary += String.fromCharCode(b);
  return btoa(binary);
}

/**
 * A 64x64 PNG of deterministic per-pixel noise (a fixed-seed linear congruential generator,
 * so every run produces the same bytes). Noise is the fixture that tells prefiltering apart
 * from sampling the full-resolution raster: averaging a 4x4 block and interpolating two
 * neighbours of the same block give very different colours. A regular pattern does not - a
 * 2-pixel stripe reduced 4x lands on the same 50/50 mix either way.
 *
 * Same standalone-page-function rule as `redSquarePngBase64`.
 */
export async function noisePngBase64(): Promise<string> {
  const canvas = document.createElement("canvas");
  canvas.width = 64;
  canvas.height = 64;
  const ctx = canvas.getContext("2d")!;
  const image = ctx.createImageData(64, 64);
  let seed = 12345;
  const next = () => { seed = (seed * 1103515245 + 12345) & 0x7fffffff; return (seed >> 16) & 0xff; };
  for (let i = 0; i < 64 * 64; i++) {
    image.data[i * 4] = next();
    image.data[i * 4 + 1] = next();
    image.data[i * 4 + 2] = next();
    image.data[i * 4 + 3] = 255;
  }
  ctx.putImageData(image, 0, 0);
  const blob = await new Promise<Blob>((resolve, reject) => {
    canvas.toBlob((b) => (b ? resolve(b) : reject(new Error("toBlob failed"))), "image/png");
  });
  const bytes = new Uint8Array(await blob.arrayBuffer());
  let binary = "";
  for (const b of bytes) binary += String.fromCharCode(b);
  return btoa(binary);
}

/**
 * The same deterministic noise as `noisePngBase64`, with every pixel nudged (blue channel, up
 * to 255 tries) until its RGB-to-HSL hue is at least 0.1 degrees from an exact half-degree.
 *
 * A pixel whose true hue lands on n + 0.5 exactly can round to an adjacent bucket of the
 * 361-entry hue-response table under the GPU's float32 arithmetic versus the CPU's float64
 * (see the CPU/GPU parity constraint in
 * docs/superpowers/plans/2026-09-22-phase3-adjustments-and-filters.md): both sides are correct
 * for their own precision, they just pick different discrete buckets right at the boundary.
 * That is a property of the fixture landing exactly on a knife edge, not a rendering bug, so
 * the Hue/Saturation parity fixture keeps every pixel clear of it instead.
 *
 * Same standalone-page-function rule as `redSquarePngBase64`.
 */
export async function hueSafeNoisePngBase64(): Promise<string> {
  const canvas = document.createElement("canvas");
  canvas.width = 64;
  canvas.height = 64;
  const ctx = canvas.getContext("2d")!;
  const image = ctx.createImageData(64, 64);
  let seed = 12345;
  const next = () => { seed = (seed * 1103515245 + 12345) & 0x7fffffff; return (seed >> 16) & 0xff; };
  // Mirrors engine/src/adjust/hsv.rs::rgb_to_hsl's hue formula exactly, so this can tell how
  // close a candidate pixel's hue lands to a half-integer degree.
  const hueDegrees = (r: number, g: number, b: number): number => {
    const rf = r / 255, gf = g / 255, bf = b / 255;
    const high = Math.max(rf, gf, bf), low = Math.min(rf, gf, bf);
    const delta = high - low;
    if (delta <= 0) return 0;
    let hue = high === rf ? (gf - bf) / delta : high === gf ? (bf - rf) / delta + 2 : (rf - gf) / delta + 4;
    hue *= 60;
    if (hue < 0) hue += 360;
    return hue;
  };
  const nearHalfDegree = (hue: number): boolean => Math.abs(hue - Math.floor(hue) - 0.5) < 0.1;
  for (let i = 0; i < 64 * 64; i++) {
    const r = next(), g = next();
    let b = next();
    for (let tries = 0; tries < 256 && nearHalfDegree(hueDegrees(r, g, b)); tries++) b = (b + 1) & 0xff;
    image.data[i * 4] = r;
    image.data[i * 4 + 1] = g;
    image.data[i * 4 + 2] = b;
    image.data[i * 4 + 3] = 255;
  }
  ctx.putImageData(image, 0, 0);
  const blob = await new Promise<Blob>((resolve, reject) => {
    canvas.toBlob((b) => (b ? resolve(b) : reject(new Error("toBlob failed"))), "image/png");
  });
  const bytes = new Uint8Array(await blob.arrayBuffer());
  let binary = "";
  for (const b of bytes) binary += String.fromCharCode(b);
  return btoa(binary);
}

/**
 * A 64x64 PNG that is opaque only on every fourth pixel in each axis, transparent elsewhere.
 *
 * The alpha, not the colour, is what a clipping source contributes, so a fully opaque fixture
 * cannot tell a reduced source from an unreduced one. Averaged down 4x this is a uniform
 * sixteenth of coverage; sampled at full resolution the output pixel centres land two pixels
 * away from every opaque column and read zero.
 */
export async function sparseAlphaPngBase64(): Promise<string> {
  const canvas = document.createElement("canvas");
  canvas.width = 64;
  canvas.height = 64;
  const ctx = canvas.getContext("2d")!;
  const image = ctx.createImageData(64, 64);
  for (let y = 0; y < 64; y++) {
    for (let x = 0; x < 64; x++) {
      const i = (y * 64 + x) * 4;
      const on = x % 4 === 0 && y % 4 === 0;
      image.data[i] = 255; image.data[i + 1] = 255; image.data[i + 2] = 255;
      image.data[i + 3] = on ? 255 : 0;
    }
  }
  ctx.putImageData(image, 0, 0);
  const blob = await new Promise<Blob>((resolve, reject) => {
    canvas.toBlob((b) => (b ? resolve(b) : reject(new Error("toBlob failed"))), "image/png");
  });
  const bytes = new Uint8Array(await blob.arrayBuffer());
  let binary = "";
  for (const b of bytes) binary += String.fromCharCode(b);
  return btoa(binary);
}

/**
 * A 32x32 8-bit grayscale PNG with a horizontal ramp from 0 to 255, as base64.
 *
 * Masks in a `.comp` package must be grayscale with no alpha (`decode_package_mask` rejects
 * anything else) and a canvas always encodes RGB or RGBA, so this writes the PNG by hand. The
 * zlib stream uses stored (uncompressed) deflate blocks, which needs no compression library and
 * is still a valid zlib stream. The ramp is the point: a mask whose interior differs from its
 * edges is what makes a wrong mask transform, a wrong background, or an all-zero coverage
 * texture visible in a comparison - a uniform mask hides all three.
 */
export async function grayRampMaskPngBase64(): Promise<string> {
  const W = 32, H = 32;
  const crcTable: number[] = [];
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    crcTable[n] = c >>> 0;
  }
  const crc32 = (bytes: number[]) => {
    let c = 0xffffffff;
    for (const b of bytes) c = crcTable[(c ^ b) & 0xff] ^ (c >>> 8);
    return (c ^ 0xffffffff) >>> 0;
  };
  const be32 = (v: number) => [(v >>> 24) & 0xff, (v >>> 16) & 0xff, (v >>> 8) & 0xff, v & 0xff];
  const chunk = (type: string, data: number[]) => {
    const name = [...type].map((ch) => ch.charCodeAt(0));
    return [...be32(data.length), ...name, ...data, ...be32(crc32([...name, ...data]))];
  };
  // Raw scanlines: one filter byte (0 = none) then W samples.
  const raw: number[] = [];
  for (let y = 0; y < H; y++) {
    raw.push(0);
    for (let x = 0; x < W; x++) raw.push(Math.round((x / (W - 1)) * 255));
  }
  // zlib: 0x78 0x01 header, stored deflate blocks, adler32 of the raw bytes.
  const z: number[] = [0x78, 0x01];
  for (let i = 0; i < raw.length; i += 0xffff) {
    const part = raw.slice(i, i + 0xffff);
    const last = i + 0xffff >= raw.length ? 1 : 0;
    z.push(last, part.length & 0xff, (part.length >> 8) & 0xff, ~part.length & 0xff, (~part.length >> 8) & 0xff, ...part);
  }
  let a = 1, b = 0;
  for (const v of raw) { a = (a + v) % 65521; b = (b + a) % 65521; }
  z.push(...be32(((b << 16) | a) >>> 0));
  // Colour type 0 (grayscale), bit depth 8.
  const png = [
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a,
    ...chunk("IHDR", [...be32(W), ...be32(H), 8, 0, 0, 0, 0]),
    ...chunk("IDAT", z),
    ...chunk("IEND", []),
  ];
  let binary = "";
  for (const v of png) binary += String.fromCharCode(v);
  return btoa(binary);
}

/**
 * A 16x16 PNG that is opaque green except for a fully transparent 8x8 hole in the middle: a
 * clipping source whose interior is transparent, which is the fragment the layer shader used
 * to read an out-of-bounds backdrop texel for.
 */
export async function ringPngBase64(): Promise<string> {
  const canvas = document.createElement("canvas");
  canvas.width = 16;
  canvas.height = 16;
  const ctx = canvas.getContext("2d")!;
  ctx.fillStyle = "#00c000";
  ctx.fillRect(0, 0, 16, 16);
  ctx.clearRect(4, 4, 8, 8);
  const blob = await new Promise<Blob>((resolve, reject) => {
    canvas.toBlob((b) => (b ? resolve(b) : reject(new Error("toBlob failed"))), "image/png");
  });
  const bytes = new Uint8Array(await blob.arrayBuffer());
  let binary = "";
  for (const b of bytes) binary += String.fromCharCode(b);
  return btoa(binary);
}
