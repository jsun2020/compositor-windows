import { test, expect, type Page } from "@playwright/test";
import { clickMenu, noisePngBase64 } from "./helpers";

async function setup(page: Page, blank = false) {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const data = blank ? null : await page.evaluate(noisePngBase64);
  await page.evaluate(async data => {
    const api = (window as any).__compositor;
    const doc = api.engine.newDocument(64, 64, false);
    if (data) api.engine.importImage(doc, Uint8Array.from(atob(data), (c: string) => c.charCodeAt(0)), "noise", { x: 32, y: 32 });
    else api.engine.execute(doc, { type: "AddBlankLayer", name: "Blank" });
    api.store.getState().openDocument(doc);
    await api.setZoom(1);
  }, data);
}
const state = (page: Page) => page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId]; });
async function setNumber(page: Page, label: string, value: string) {
  const field = page.getByRole("spinbutton", { name: label, exact: true });
  await field.fill(value); await field.press("Enter");
}

for (const worker of [false, true]) test(`blank Vignette preview, ${worker ? "worker" : "direct"} commit and undo`, async ({ page }) => {
  await setup(page, true);
  const before = await state(page);
  if (worker) await page.evaluate(() => (window as any).__compositor.store.setState({ jobPixels: 1 }));
  await clickMenu(page, "Filter", "filter-vignette");
  await setNumber(page, "Amount", "75");
  const preview = await page.evaluate(() => { const a = (window as any).__compositor, s = a.store.getState(); return Array.from(a.engine.composite(s.activeId, { x: 0, y: 0, width: 64, height: 64 }, 64, 64)); });
  expect(preview.some((v, i) => i % 4 === 3 && (v as number) > 0)).toBe(true);
  await page.getByTestId("adjust-ok").click();
  await expect.poll(() => page.evaluate(() => (window as any).__compositor.store.getState().working)).toBe(false);
  expect((await state(page)).undoDepth).toBe(before.undoDepth + 1);
  const committed = await page.evaluate(() => { const a = (window as any).__compositor, s = a.store.getState(); return Array.from(a.engine.composite(s.activeId, { x: 0, y: 0, width: 64, height: 64 }, 64, 64)); });
  expect(committed).toEqual(preview);
  await clickMenu(page, "Edit", "undo");
  expect((await state(page)).layers[0].hasPixels).toBe(false);
});

for (const [kind, menu, field, value, settings, property] of [
  ["Vignette", "vignette", "Amount", "72", "vignette", "amount"],
  ["Tonal Contrast", "tonal-contrast", "Amount", "67", "tonal", "amount"],
  ["Sharpen", "sharpen", "Amount", "123", "sharpen", "amount"],
  ["Lens Effects", "lens-effects", "Amount", "63", "lens", "flareAmount"],
] as const) test(`${kind} adjustment keeps edits on reopen and in a saved package`, async ({ page }) => {
  await setup(page);
  await clickMenu(page, "Layer", `layer-adjustment-${menu}`);
  const before = await state(page);
  await setNumber(page, field, value);
  await page.getByTestId("adjust-ok").click();
  const after = await state(page);
  expect(after.undoDepth).toBe(before.undoDepth + 1);
  expect(after.layers[1].adjustment.filterLayerSettings[settings][property]).toBe(Number(value));
  expect(after.layers[0].pixelsRevision).toBe(before.layers[0].pixelsRevision);
  await page.getByTestId("layer-row").nth(0).dblclick();
  await expect(page.getByRole("spinbutton", { name: field, exact: true })).toHaveValue(value);
  await page.getByTestId("adjust-cancel").click();
  const result = await page.evaluate(() => {
    const a = (window as any).__compositor, s = a.store.getState(), r = { x: 0, y: 0, width: 64, height: 64 };
    const original = a.engine.composite(s.activeId, r, 64, 64), files = a.engine.savePackage(s.activeId);
    const id = a.engine.openPackage(files, null), reopened = a.engine.composite(id, r, 64, 64);
    const same = original.every((v: number, i: number) => v === reopened[i]); a.engine.closeDocument(id);
    return { same, version: JSON.parse(files.manifest).version };
  });
  expect(result.same).toBe(true);
  expect(result.version).toBe(kind === "Sharpen" ? 19 : kind === "Lens Effects" ? 20 : 18);
});

test("custom crop ratio validates, cancels and applies through the visible controls", async ({ page }) => {
  await setup(page);
  await page.getByTestId("tool-crop").click();
  await page.getByLabel("Ratio", { exact: true }).selectOption("Custom");
  const input = page.getByLabel("Custom ratio width:height");
  await input.fill("0:5"); await expect(page.getByTestId("crop-custom-apply")).toBeDisabled();
  await input.fill("4:5"); await page.getByTestId("crop-custom-cancel").click();
  expect(await page.evaluate(() => (window as any).__compositor.store.getState().cropRatio)).toBe("Free");
  await page.getByLabel("Ratio", { exact: true }).selectOption("Custom");
  await input.fill(" 4 : 5 "); await page.getByTestId("crop-custom-apply").click();
  const ratio = await page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return { value: s.cropRatio, ratio: s.cropRect.width / s.cropRect.height }; });
  expect(ratio.value).toBe("4:5"); expect(ratio.ratio).toBeCloseTo(.8);
  await page.getByTestId("crop-apply").click();
  const d = await state(page); expect(d.width / d.height).toBeCloseTo(.8, 1);
});

test("Prism exposes new controls, applies and records one undo step", async ({ page }) => {
  await setup(page);
  const before = await state(page);
  await clickMenu(page, "Filter", "filter-lens-effects");
  await page.getByLabel("Effect", { exact: true }).selectOption("Prism");
  await expect(page.getByRole("spinbutton", { name: "Amount", exact: true })).toHaveValue("80");
  await setNumber(page, "Blur", "12"); await setNumber(page, "Fringe", "40");
  await page.getByLabel("Direction", { exact: true }).selectOption("Both Ways");
  await page.getByTestId("adjust-ok").click();
  expect((await state(page)).undoDepth).toBe(before.undoDepth + 1);
  await clickMenu(page, "Edit", "undo");
  expect((await state(page)).layers[0].pixelsRevision).toBe(before.layers[0].pixelsRevision);
});

test("Align and Distribute controls move selected layers and undo once", async ({ page }) => {
  await setup(page);
  await page.evaluate(() => {
    const a = (window as any).__compositor, s = a.store.getState(), id = s.activeId;
    const first = s.documents[id].layers[0].id;
    a.engine.execute(id, { type: "DuplicateLayer", id: first });
    a.engine.execute(id, { type: "DuplicateLayer", id: first });
    const d = a.engine.state(id);
    d.layers.forEach((l: any, i: number) => a.engine.execute(id, { type: "SetLayerTransform", id: l.id, transform: { ...l.transform, origin: [[0, 8, 26][i], i * 7], size: [10, 10] } }));
    s.refresh(id); a.store.setState({ selectedLayerIds: d.layers.map((l: any) => l.id), tool: "move" });
  });
  const before = await state(page);
  await page.getByRole("button", { name: "Align top edges", exact: true }).click();
  const aligned = await state(page);
  expect(new Set(aligned.layers.map((l: any) => l.transform.origin[1])).size).toBe(1);
  expect(aligned.undoDepth).toBe(before.undoDepth + 1);
  await clickMenu(page, "Edit", "undo");
  expect((await state(page)).layers.map((l: any) => l.transform)).toEqual(before.layers.map((l: any) => l.transform));
  await page.getByRole("button", { name: "Distribute horizontal spacing", exact: true }).click();
  const positioned = (await state(page)).layers.map((l: any) => l.transform.origin[0]).sort((a: number, b: number) => a - b);
  expect(positioned[1] - positioned[0]).toBe(positioned[2] - positioned[1]);
});

test("oversized lens render reports a recoverable error without losing the document", async ({ page }) => {
  await page.goto("/"); await expect(page.getByTestId("engine-ready")).toBeVisible();
  await page.evaluate(async () => {
    const a = (window as any).__compositor, id = a.engine.newDocument(4000, 2000, false);
    a.engine.execute(id, { type: "AddAdjustmentLayer", kind: "Lens Effects" });
    a.store.getState().openDocument(id); await a.setZoom(.1);
  });
  const before = await state(page);
  await page.evaluate(async () => { await (window as any).__compositor.setZoom(1); });
  await expect(page.getByTestId("error-banner")).toContainText("working memory limit");
  expect((await state(page)).layers).toEqual(before.layers);
  // Fit-to-window can still render the same document. Saving does not bake a fallback.
  await page.evaluate(async () => {
    const a = (window as any).__compositor; a.store.getState().setError(null); await a.setZoom(.1);
  });
  await expect(page.getByTestId("error-banner")).toHaveCount(0);
  const version = await page.evaluate(() => { const a = (window as any).__compositor; return JSON.parse(a.engine.savePackage(a.store.getState().activeId).manifest).version; });
  expect(version).toBe(18); // Default adjustment has no explicit nested Prism settings.
});
