import { test, expect, type Page } from "@playwright/test";
import { solidPngBase64 } from "./helpers";

// Phase 4b-1: the history cap (engine/tests/history_cap.rs pins the engine) seen from the app.

type Pt = [number, number];
const state = (page: Page) => page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId]; });
/** Where document point `p` is on screen. */
const client = (page: Page, p: Pt) => page.evaluate(([x, y]) => {
  const s = (window as any).__compositor.store.getState(); const vp = s.viewports[s.activeId]; const d = s.documents[s.activeId];
  const v = vp.viewPoint({ x, y }, { width: d.width, height: d.height });
  const r = document.querySelector('[data-testid="canvas-view"]')!.getBoundingClientRect();
  return { x: r.left + v.x, y: r.top + v.y };
}, p);

test("at the history cap, Escape during an Alt-drag duplicate still takes the copy away", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const png = await page.evaluate(solidPngBase64, { width: 40, height: 30, color: "#20a060" });
  await page.evaluate(async (data) => {
    const api = (window as any).__compositor;
    const doc = api.engine.newDocument(64, 48, false);
    api.engine.importImage(doc, Uint8Array.from(atob(data), (c: string) => c.charCodeAt(0)), "Green", { x: 32, y: 24 });
    const layer = api.engine.state(doc).layers[0].id;
    // 100 entries: the import and 99 renames. The next push trims the oldest.
    for (let n = 0; n < 99; n++) api.engine.execute(doc, { type: "RenameLayer", id: layer, name: `n${n}` });
    api.store.getState().openDocument(doc);
    await api.setZoom(4);
  }, png);
  expect((await state(page)).undoDepth).toBe(100);
  await page.keyboard.press("v");
  const a = await client(page, [30, 24]), b = await client(page, [40, 30]);
  await page.keyboard.down("Alt");
  await page.mouse.move(a.x, a.y); await page.mouse.down();
  await page.mouse.move(b.x, b.y, { steps: 4 });
  const during = await state(page);
  expect(during.layers.length, "the copy exists while dragging").toBe(2);
  expect(during.undoDepth, "the copy's push trimmed the oldest entry").toBe(100);
  // Alt is let go first: the keymap matches Escape only without modifiers.
  await page.keyboard.up("Alt");
  await page.keyboard.press("Escape");
  await page.mouse.up();
  const after = await state(page);
  expect(after.layers.map((l: any) => l.name)).toEqual(["n98"]);
  expect(after.undoDepth).toBe(99);
  expect(after.canRedo, "nothing to redo: the copy was reverted, not undone").toBe(false);
});
