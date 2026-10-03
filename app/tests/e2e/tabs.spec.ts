import { test, expect, type Page } from "@playwright/test";

// Dragging a project tab along the strip reorders the tabs (Compositor 1.4.5, ProjectTabs.swift:147-205):
// a press that moves 3 px or more becomes a drag and selects its tab; let go, the tab lands in the gap
// nearest it; a job's result to come (or a project operation) leaves the tabs where they were.

async function threeTabs(page: Page): Promise<string[]> {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  return page.evaluate(() => {
    const api = (window as any).__compositor;
    return [40, 50, 60].map((size) => { const doc = api.engine.newDocument(size, size, false); api.store.getState().openDocument(doc); return doc as string; });
  });
}
const tab = (page: Page, id: string) => page.locator(`[data-testid="project-tab"][data-doc-id="${id}"]`);
const order = (page: Page) => page.evaluate(() => (window as any).__compositor.store.getState().order as string[]);
const active = (page: Page) => page.evaluate(() => (window as any).__compositor.store.getState().activeId as string);
/** Presses the tab's title, moves by `dx` in `steps` moves, and lets go. */
async function drag(page: Page, id: string, dx: number, steps = 8) {
  const box = (await tab(page, id).locator("span").boundingBox())!;
  const x = box.x + box.width / 2, y = box.y + box.height / 2;
  await page.mouse.move(x, y);
  await page.mouse.down();
  for (let i = 1; i <= steps; i++) await page.mouse.move(x + dx * i / steps, y);
  await page.mouse.up();
}

test("dragging a tab past its neighbours moves it there and selects it; a 2 px wobble is a click", async ({ page }) => {
  const [a, b, c] = await threeTabs(page);
  expect(await order(page)).toEqual([a, b, c]);
  expect(await active(page)).toBe(c);
  // Under the 3 px threshold nothing moves; the press is a click on `a`.
  await drag(page, a, 2, 2);
  expect(await order(page)).toEqual([a, b, c]);
  // `c` dragged to the far left lands first and is selected.
  await tab(page, a).click();
  const toStart = (await tab(page, a).boundingBox())!.x - (await tab(page, c).boundingBox())!.x - 20;
  await drag(page, c, toStart);
  expect(await order(page)).toEqual([c, a, b]);
  expect(await active(page)).toBe(c);
  // `c` dragged past the last tab lands last.
  const toEnd = (await tab(page, b).boundingBox())!.x + (await tab(page, b).boundingBox())!.width - (await tab(page, c).boundingBox())!.x + 20;
  await drag(page, c, toEnd);
  expect(await order(page)).toEqual([a, b, c]);
});

test("while a job's result is to come a tab drag moves nothing", async ({ page }) => {
  const [a, b, c] = await threeTabs(page);
  await page.evaluate(() => (window as any).__compositor.store.setState({ working: true }));
  const toStart = (await tab(page, a).boundingBox())!.x - (await tab(page, c).boundingBox())!.x - 20;
  await drag(page, c, toStart);
  expect(await order(page)).toEqual([a, b, c]);
  await page.evaluate(() => (window as any).__compositor.store.setState({ working: false }));
  await drag(page, c, toStart);
  expect(await order(page)).toEqual([c, a, b]);
});
