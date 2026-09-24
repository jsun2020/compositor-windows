import { test, expect, type Page } from "@playwright/test";
import { noisePngBase64 } from "./helpers";

// Copied verbatim from adjust-layers.spec.ts, where they are file-local.
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

test("a layer using a blend mode this build does not draw yet shows it, disabled", async ({ page }) => {
  await setup(page);
  await page.evaluate(() => {
    const api = (window as any).__compositor; const s = api.store.getState();
    const id = api.engine.state(s.activeId).layers[0].id;
    api.engine.execute(s.activeId, { type: "SetLayerBlendMode", id, mode: "Soft Light" });
    s.refresh(); s.invalidate();
  });
  expect((await state(page)).layers[0].blendMode).toBe("Soft Light");
  const select = page.getByRole("combobox", { name: "Blend mode", exact: true });
  await expect(select).toHaveValue("Soft Light");
  // Not toBeDisabled(): that assertion's "follow-label" retargeting walks from the <option>,
  // which is not itself a labelable element, up through the enclosing <label>Blend <select>...
  // to the label's control (the <select>), and reports the SELECT's disabled state instead of
  // the option's. Checking the attribute directly targets the option itself.
  await expect(select.locator("option[value='Soft Light']")).toHaveAttribute("disabled", "");
});
