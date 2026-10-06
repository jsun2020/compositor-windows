import { test, expect } from "@playwright/test";
import { clickMenu } from "./helpers";

for (const kind of ["gl", "cpu"] as const) {
  test(`${kind}: closing the last tab clears its picture and overlay without creating a document`, async ({ page }) => {
    if (kind === "cpu") await page.addInitScript(() => {
      const original = HTMLCanvasElement.prototype.getContext;
      HTMLCanvasElement.prototype.getContext = function (type: string, ...rest: unknown[]) {
        if (type === "webgl2") return null;
        return (original as any).call(this, type, ...rest);
      };
    });
    await page.goto("/");
    await expect(page.getByTestId("engine-ready")).toBeVisible();
    await page.evaluate(() => {
      const api = (window as any).__compositor;
      for (const [name, color] of [["Red", "#ff0000"], ["Blue", "#0000ff"]]) {
        const canvas = document.createElement("canvas"); canvas.width = 96; canvas.height = 64;
        const ctx = canvas.getContext("2d")!; ctx.fillStyle = color; ctx.fillRect(0, 0, 96, 64);
        const bytes = Uint8Array.from(atob(canvas.toDataURL().split(",")[1]), c => c.charCodeAt(0));
        const id = api.engine.importImage(null, bytes, name, null);
        api.engine.markSaved(id, `C:/test/${name}.comp`);
        api.store.getState().openDocument(id);
      }
    });
    await expect(page.getByTestId("project-tab")).toHaveCount(2);
    expect(await page.evaluate(() => (window as any).__compositor.store.getState().rendererKind)).toBe(kind);
    await page.keyboard.press("b");
    const box = await page.getByTestId("canvas-view").boundingBox();
    await page.mouse.move(box!.x + box!.width / 2, box!.y + box!.height / 2);
    await expect.poll(() => page.getByTestId("overlay").evaluate((canvas: HTMLCanvasElement) =>
      canvas.getContext("2d")!.getImageData(0, 0, canvas.width, canvas.height).data.some(v => v !== 0))).toBe(true);
    const picture = () => page.getByTestId("canvas-view").locator("canvas").first().evaluate((c: HTMLCanvasElement) => c.toDataURL());
    const blue = await picture();
    await page.getByRole("button", { name: "Close Red", exact: true }).click();
    await expect(page.getByTestId("project-tab")).toHaveCount(1);
    await expect.poll(picture).toBe(blue);
    await page.getByRole("button", { name: "Close Blue", exact: true }).click();
    await expect(page.getByTestId("project-tab")).toHaveCount(0);
    await expect(page.getByTestId("layer-row")).toHaveCount(0);
    await expect(page.getByText("Open an image or create a new canvas", { exact: true })).toBeVisible();
    await expect.poll(() => page.evaluate(() => {
      const api = (window as any).__compositor, pixels = api.renderer.readPixels();
      let worst = 0;
      for (let i = 0; i < pixels.length; i++) worst = Math.max(worst, Math.abs(pixels[i] - (i % 4 === 3 ? 255 : 41)));
      const canvas = document.querySelector('[data-testid="overlay"]') as HTMLCanvasElement;
      const overlay = canvas.getContext("2d")!.getImageData(0, 0, canvas.width, canvas.height).data;
      return { pictureCleared: worst <= 1, overlayCleared: overlay.every(v => v === 0), active: api.store.getState().activeId };
    })).toEqual({ pictureCleared: true, overlayCleared: true, active: null });
    // Clearing must keep the renderer usable. A new transparent document is an explicit user action.
    await clickMenu(page, "File", "new");
    await page.getByRole("button", { name: "Create", exact: true }).click();
    await expect(page.getByTestId("project-tab")).toHaveCount(1);
    await expect(page.getByTestId("layer-row")).toHaveCount(1);
    await expect(page.getByText("Open an image or create a new canvas", { exact: true })).toHaveCount(0);
    await expect.poll(picture).not.toBe(blue);
  });
}
