import { test, expect, type Page } from "@playwright/test";
import { solidPngBase64 } from "./helpers";

// Phase 4b-1: the Eyedropper (I) samples the canvas into the image's foreground colour.

type Pt = [number, number];

/** A 64 x 48 document: red on the left 32 columns, blue on the right, transparent below row 40,
 * at 4 CSS px a pixel. */
async function setup(page: Page) {
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const red = await page.evaluate(solidPngBase64, { width: 32, height: 40, color: "#ff0000" });
  const blue = await page.evaluate(solidPngBase64, { width: 32, height: 40, color: "#0000ff" });
  await page.evaluate(async ([r, b]) => {
    const api = (window as any).__compositor;
    const bytes = (data: string) => Uint8Array.from(atob(data), (c: string) => c.charCodeAt(0));
    const doc = api.engine.newDocument(64, 48, false);
    api.engine.importImage(doc, bytes(r), "Red", { x: 16, y: 20 });
    api.engine.importImage(doc, bytes(b), "Blue", { x: 48, y: 20 });
    api.store.getState().openDocument(doc);
    await api.setZoom(4);
  }, [red, blue]);
}
const client = (page: Page, p: Pt) => page.evaluate(([x, y]) => {
  const s = (window as any).__compositor.store.getState(); const vp = s.viewports[s.activeId]; const d = s.documents[s.activeId];
  const v = vp.viewPoint({ x, y }, { width: d.width, height: d.height });
  const r = document.querySelector('[data-testid="canvas-view"]')!.getBoundingClientRect();
  return { x: r.left + v.x, y: r.top + v.y };
}, p);
const foreground = (page: Page) => page.evaluate(() => (window as any).__compositor.store.getState().palette.foreground);

test("I picks the Eyedropper; a press sets the foreground to the colour under it, a drag follows, a transparent pixel keeps it", async ({ page }) => {
  await setup(page);
  const depth = await page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId].undoDepth; });
  await page.keyboard.press("i");
  await expect(page.getByTestId("tool-eyedropper")).toHaveClass(/active/);
  const red = await client(page, [10, 10]), blue = await client(page, [50, 10]), clear = await client(page, [50, 44]);
  await page.mouse.move(red.x, red.y); await page.mouse.down();
  expect(await foreground(page)).toEqual({ red: 1, green: 0, blue: 0 });
  const ring = () => page.evaluate(() => (window as any).__compositor.store.getState().sampleRing);
  expect(await ring()).toMatchObject({ sampled: { red: 1, green: 0, blue: 0 }, original: { red: 0, green: 0, blue: 0 } });
  await page.mouse.move(blue.x, blue.y, { steps: 4 });
  await expect.poll(() => foreground(page)).toEqual({ red: 0, green: 0, blue: 1 });
  expect((await ring()).original, "the ring keeps the colour the press began with").toEqual({ red: 0, green: 0, blue: 0 });
  await page.mouse.move(clear.x, clear.y, { steps: 2 });
  await page.mouse.up();
  expect(await foreground(page), "a transparent pixel has no colour to take").toEqual({ red: 0, green: 0, blue: 1 });
  expect(await ring()).toBeNull();
  expect(await page.getByTestId("palette-foreground").evaluate((el) => getComputedStyle(el).backgroundColor)).toBe("rgb(0, 0, 255)");
  const after = await page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId].undoDepth; });
  expect(after, "sampling records nothing").toBe(depth);
});
