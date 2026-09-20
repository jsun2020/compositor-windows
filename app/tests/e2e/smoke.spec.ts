import { test, expect } from "@playwright/test";

test("engine loads in the browser", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toContainText("Compositor engine 0.1.0");
  const ids = await page.evaluate(() => {
    const api = (window as unknown as { __compositor: { engine: { newDocument(w: number, h: number, e: boolean): string; documentIds(): string[] } } }).__compositor;
    api.engine.newDocument(10, 10, true);
    return api.engine.documentIds();
  });
  expect(ids).toHaveLength(1);
});
