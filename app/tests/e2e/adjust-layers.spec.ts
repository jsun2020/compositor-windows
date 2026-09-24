import { test, expect, type Page } from "@playwright/test";
import { clickMenu, noisePngBase64 } from "./helpers";

async function setup(page: Page) {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const b64 = await page.evaluate(noisePngBase64);
  await page.evaluate(async (data) => {
    const api = (window as any).__compositor;
    const png = Uint8Array.from(atob(data), (c: string) => c.charCodeAt(0));
    const doc = api.engine.newDocument(64, 64, false);
    api.engine.importImage(doc, png, "noise", { x: 32, y: 32 });
    api.store.getState().openDocument(doc);
    await api.setZoom(1);
  }, b64);
}
const state = (page: Page) => page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId]; });

test("a new adjustment layer appears above the active layer and never rewrites its pixels", async ({ page }) => {
  await setup(page);
  const before = (await state(page)).layers[0];
  await clickMenu(page, "Layer", "layer-adjustment-levels");
  const d = await state(page);
  expect(d.layers.length).toBe(2);
  expect(d.layers[1].adjustment.kind).toBe("Levels");
  expect(d.layers[1].hasPixels).toBe(false);
  expect(d.layers[0].pixelsRevision).toBe(before.pixelsRevision);
  await expect(page.getByTestId(`adjustment-chip-${d.layers[1].id}`)).toBeVisible();
  expect(await page.getByTestId("layer-row").nth(0).innerText()).toContain("Levels");
});

test("double-clicking an ordinary layer row still renames it", async ({ page }) => {
  await setup(page);
  // Pairs with the next test: that one pins the adjustment-row branch of the row's
  // onDoubleClick conditional, this one pins the other branch. Without both, a regression
  // that always took one branch (e.g. an adjustment layer opening a rename editor, or an
  // ordinary layer opening a panel instead) could pass the rest of the suite unnoticed.
  const row = page.getByTestId("layer-row").nth(0);
  await row.dblclick();
  const input = row.getByRole("textbox");
  await expect(input).toBeVisible();
  await expect(page.getByTestId("adjust-panel")).toHaveCount(0);
  await input.fill("Renamed");
  await input.press("Enter");
  expect((await state(page)).layers[0].name).toBe("Renamed");
});

test("double-clicking an adjustment row edits it live and OK records one step", async ({ page }) => {
  await setup(page);
  await clickMenu(page, "Layer", "layer-adjustment-curves");
  const id = (await state(page)).layers[1].id;
  // Ruling (pre-flight, Task 17): close the auto-opened panel FIRST. addAdjustmentLayer ends by
  // calling beginAdjust, so the menu action leaves a Curves panel already open, and beginAdjust
  // no-ops while adjustEdit is set. Without this cancel the dblclick below is dead: the title
  // already reads "Curves" and the assertion passes even if double-click is wired to nothing at
  // all, which is the exact capability this test claims to cover. Cancelling records nothing, so
  // the undo depth captured later is unaffected.
  await page.getByTestId("adjust-cancel").click();
  await expect(page.getByTestId("adjust-panel")).toHaveCount(0, { timeout: 2000 });
  await page.getByTestId("layer-row").nth(0).dblclick();
  await expect(page.getByTestId("adjust-title")).toHaveText("Curves");
  const editor = page.getByTestId("curves-editor");
  const box = (await editor.boundingBox())!;
  const depthBefore = (await state(page)).undoDepth;
  await page.mouse.click(box.x + box.width * 0.5, box.y + box.height * 0.2);
  // The live preview goes through the plan: the document has no new undo step yet.
  expect((await state(page)).undoDepth).toBe(depthBefore); // only the New Curves Adjustment step so far
  await page.getByTestId("adjust-ok").click();
  expect((await state(page)).undoDepth).toBe(depthBefore + 1);
  const points = (await state(page)).layers.find((l: any) => l.id === id).adjustment.curves.channels[0];
  expect(points.length).toBe(3);
  await page.keyboard.press("Control+z");
  expect((await state(page)).layers.find((l: any) => l.id === id).adjustment.curves.channels[0].length).toBe(2);
});

test("cancelling an edit restores the settings and records nothing", async ({ page }) => {
  await setup(page);
  await clickMenu(page, "Layer", "layer-adjustment-exposure");
  const depth = (await state(page)).undoDepth;
  // No dblclick here: addAdjustmentLayer already opened the Exposure panel, so a double-click on
  // the row would have nothing to do -- and the floating adjust-panel (position: fixed, right of
  // the layers list) covers the row's centre at this viewport size, so Playwright would retry an
  // intercepted click until it timed out. Do not reintroduce it.
  // Ruling (Task 16): a role-based locator, not getByLabel. The open panel's dialog carries
  // aria-label="Exposure", identical to this field's label, so getByLabel("Exposure") matches the
  // dialog as well as the field even with exact matching. A number input's role is spinbutton,
  // which the dialog can never match.
  await page.getByRole("spinbutton", { name: "Exposure", exact: true }).fill("2");
  await page.getByRole("spinbutton", { name: "Exposure", exact: true }).press("Enter");
  await page.getByTestId("adjust-cancel").click();
  const d = await state(page);
  expect(d.undoDepth).toBe(depth);
  // exposureSettings is Option<ExposureSettings>: absent until a commit, exactly as on the Mac
  // (LayerAdjustment.swift), and absent means identity. Cancel committed nothing, so it stays absent.
  expect(d.layers[1].adjustment.exposureSettings?.exposure ?? 0).toBe(0);
});

test("an adjustment layer saves, reopens and still renders the same", async ({ page }) => {
  await setup(page);
  await clickMenu(page, "Layer", "layer-adjustment-gradient-map");
  const saved = await page.evaluate(() => {
    const api = (window as any).__compositor; const s = api.store.getState();
    const before = Array.from(api.engine.composite(s.activeId, { x: 0, y: 0, width: 64, height: 64 }, 64, 64)) as number[];
    const files = api.engine.savePackage(s.activeId);
    const reopened = api.engine.openPackage(files, null);
    const after = Array.from(api.engine.composite(reopened, { x: 0, y: 0, width: 64, height: 64 }, 64, 64)) as number[];
    const manifest = JSON.parse(files.manifest);
    api.engine.closeDocument(reopened);
    return { same: before.every((v, i) => v === after[i]), version: manifest.version, adjustment: manifest.layers[1].adjustment };
  });
  expect(saved.same).toBe(true);
  expect(saved.version).toBe(9);
  expect(saved.adjustment.kind).toBe("Gradient Map");
  expect(saved.adjustment.gradientMapSettings).toBeTruthy();
  expect(saved.adjustment.exposureSettings).toBeUndefined();
});
