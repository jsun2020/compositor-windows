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
