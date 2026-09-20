import type { Page } from "@playwright/test";

/**
 * Opens a top-level dropdown menu ("File", "Edit", "Image", "View") and clicks one of
 * its items. The menu bar mounts each title's items only while that title's dropdown
 * is open, so a bare `getByTestId("menu-<id>").click()` finds nothing until the title
 * has been clicked first.
 */
export async function clickMenu(page: Page, title: "File" | "Edit" | "Image" | "View", id: string): Promise<void> {
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
