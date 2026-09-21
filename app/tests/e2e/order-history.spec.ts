import { test, expect, type Page } from "@playwright/test";
import { clickMenu } from "./helpers";

async function fresh(page: Page) {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  await clickMenu(page, "File", "new");
  await page.getByRole("button", { name: "Create" }).click();
}

const state = (page: Page) => page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId]; });
const names = (page: Page) => page.getByTestId("layer-row").allInnerTexts();
/** The ids the render plan draws, bottom to top: what the canvas and the export actually composite. */
const drawOrder = (page: Page) => page.evaluate(() => {
  const api = (window as any).__compositor;
  const s = api.store.getState();
  const plan = api.engine.renderPlan(s.activeId, null);
  const out: string[] = [];
  for (const n of plan.nodes) {
    if (n.kind === "layer") out.push(n.draw.id);
    else { out.push(n.base.id); for (const c of n.children) out.push(c.id); }
  }
  return out;
});
/** The panel's rows bottom to top, folders dropped: the order the user is being shown. */
const panelOrder = async (page: Page) => {
  const ids = await page.getByTestId("layer-row").evaluateAll((rows) =>
    rows.filter((r) => !r.querySelector(".chip-folder")).map((r) => r.getAttribute("data-layer-id")!));
  return ids.reverse();
};

test("the canvas composites in the order the panel shows, after every gesture that reorders the array", async ({ page }) => {
  await fresh(page);
  // Three root layers: Layer 1, Layer 2, Layer 3 bottom to top.
  await page.getByTestId("layer-add").click();
  await page.getByTestId("layer-add").click();
  expect(await drawOrder(page)).toEqual(await panelOrder(page));

  // Group the bottom two. The wrapper takes their place and the children are appended to the
  // end of the array, so array order and draw order part company here.
  await page.getByTestId("layer-row").nth(2).click();
  await page.getByTestId("layer-row").nth(1).click({ modifiers: ["Shift"] });
  await page.getByTestId("layer-row").nth(1).click({ button: "right" });
  await page.getByTestId("ctx-group").click();
  let d = await state(page);
  const folder = d.layers.find((l: any) => l.isGroup);
  expect(d.layers.map((l: any) => l.id)).not.toEqual(await drawOrder(page));
  expect(await drawOrder(page)).toEqual(await panelOrder(page));
  // Layer 3 was on top before grouping and must still draw last.
  const top = d.layers.find((l: any) => l.name === "Layer 3").id;
  expect((await drawOrder(page)).at(-1)).toBe(top);

  // Drop that top layer onto the folder row: place_layer with no target, the drag path.
  await page.locator('[data-testid="layer-row"]', { hasText: "Layer 3" })
    .dragTo(page.locator(`[data-layer-id="${folder.id}"]`), { targetPosition: { x: 60, y: 14 } });
  d = await state(page);
  expect(d.layers.find((l: any) => l.name === "Layer 3").parentId).toBe(folder.id);
  expect(await drawOrder(page)).toEqual(await panelOrder(page));
});

test("clicking a layer row keeps the redo stack", async ({ page }) => {
  await fresh(page);
  await page.getByTestId("layer-add").click();
  expect((await state(page)).layers.length).toBe(2);
  await page.keyboard.press("Control+z");
  let d = await state(page);
  expect(d.layers.length).toBe(1);
  expect(d.canRedo).toBe(true);
  // The click that used to destroy the redo stack.
  await page.getByTestId("layer-row").nth(0).click();
  d = await state(page);
  expect(d.canRedo).toBe(true);
  expect(d.canUndo).toBe(false);
  await page.keyboard.press("Control+Shift+z");
  expect((await state(page)).layers.length).toBe(2);
});

test("collapsing a folder records no undo entry", async ({ page }) => {
  await fresh(page);
  await page.getByTestId("layer-add").click();
  await page.getByTestId("layer-row").nth(1).click();
  await page.getByTestId("layer-row").nth(1).click({ modifiers: ["Shift"] });
  await page.getByTestId("layer-row").nth(0).click({ button: "right" });
  await page.getByTestId("ctx-group").click();
  const folder = (await state(page)).layers.find((l: any) => l.isGroup);
  // Select a layer inside the folder, then collapse it.
  await page.getByTestId("layer-row").nth(1).click();
  const before = (await state(page)).layers.length;
  await page.getByTestId(`collapse-${folder.id}`).click();
  expect((await state(page)).canRedo).toBe(false);
  // Ctrl+Z must undo the grouping, not the collapse's selection change.
  await page.keyboard.press("Control+z");
  const d = await state(page);
  expect(d.layers.some((l: any) => l.isGroup)).toBe(false);
  expect(d.layers.length).toBe(before - 1);
});

test("a layer dropped into a collapsed folder is revealed", async ({ page }) => {
  await fresh(page);
  await page.getByTestId("layer-add").click();
  await page.getByTestId("layer-add-folder").click();
  const folder = (await state(page)).layers.find((l: any) => l.isGroup);
  await page.getByTestId(`collapse-${folder.id}`).click();
  expect((await names(page)).length).toBe(3);
  await page.locator('[data-testid="layer-row"]', { hasText: "Layer 1" })
    .dragTo(page.locator(`[data-layer-id="${folder.id}"]`), { targetPosition: { x: 60, y: 14 } });
  const d = await state(page);
  expect(d.layers.find((l: any) => l.name === "Layer 1").parentId).toBe(folder.id);
  // The folder reopened, so the moved layer has a row again.
  await expect(page.locator(`[data-layer-id="${d.activeLayerId}"]`)).toBeVisible();
  expect(await page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.collapsed[s.activeId] ?? []; })).toEqual([]);
});

test("typing an opacity is one undo entry and an empty field is not zero", async ({ page }) => {
  await fresh(page);
  const field = page.getByLabel("Opacity");
  await field.fill("35");
  await field.press("Enter");
  expect((await state(page)).layers[0].opacity).toBeCloseTo(0.35, 5);
  // One entry, not one per keystroke: a single undo goes back to 100%. The field owns every
  // key while it has focus (isShortcutBlocked), so move focus off it first.
  await page.getByTestId("layer-row").nth(0).click();
  await page.keyboard.press("Control+z");
  expect((await state(page)).layers[0].opacity).toBeCloseTo(1, 5);
  await page.keyboard.press("Control+Shift+z");
  expect((await state(page)).layers[0].opacity).toBeCloseTo(0.35, 5);
  // Clearing the field commits nothing and puts the committed value back.
  await field.fill("");
  await field.press("Enter");
  expect((await state(page)).layers[0].opacity).toBeCloseTo(0.35, 5);
  await expect(field).toHaveValue("35");
});

test("a drop the engine would refuse is not highlighted", async ({ page }) => {
  await fresh(page);
  await page.getByTestId("layer-add-folder").click();
  const folder = (await state(page)).layers.find((l: any) => l.isGroup);
  await page.locator('[data-testid="layer-row"]', { hasText: "Layer 1" })
    .dragTo(page.locator(`[data-layer-id="${folder.id}"]`), { targetPosition: { x: 60, y: 14 } });
  const inner = (await state(page)).layers.find((l: any) => l.name === "Layer 1");
  // Dragging the folder into its own child is impossible; the row must not light up.
  await page.locator(`[data-layer-id="${folder.id}"]`).hover();
  await page.mouse.down();
  await page.locator(`[data-layer-id="${inner.id}"]`).hover();
  await expect(page.locator(`[data-layer-id="${inner.id}"]`)).not.toHaveAttribute("data-drop-zone", /.*/);
  await page.mouse.up();
  expect((await state(page)).layers.find((l: any) => l.isGroup).parentId).toBeNull();
});

test("Escape closes a context menu without cancelling the crop rectangle", async ({ page }) => {
  await fresh(page);
  await page.getByTestId("tool-crop").click();
  await page.evaluate(() => (window as any).__compositor.store.getState().setCropRect({ x: 1, y: 1, width: 40, height: 40 }));
  await page.getByTestId("layer-row").nth(0).click({ button: "right" });
  await expect(page.getByTestId("ctx-duplicate")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("ctx-duplicate")).toHaveCount(0);
  expect(await page.evaluate(() => (window as any).__compositor.store.getState().cropRect)).not.toBeNull();
});
