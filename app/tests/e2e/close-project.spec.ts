import { test, expect, type Page } from "@playwright/test";
import { clickMenu, redSquarePngBase64 } from "./helpers";

const closeDialog = (page: Page) => page.getByRole("dialog", { name: "Close project", exact: true });
async function dirtyProject(page: Page): Promise<string> {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const b64 = await page.evaluate(redSquarePngBase64);
  const id = await page.evaluate(b64 => {
    const api = (window as any).__compositor;
    const id = api.engine.importImage(null, Uint8Array.from(atob(b64), (c: string) => c.charCodeAt(0)), "red", null);
    api.store.getState().openDocument(id);
    return id as string;
  }, b64);
  await expect(page.getByTestId("project-tab")).toContainText("\u2022");
  return id;
}
const snapshot = (page: Page) => page.evaluate(() => {
  const s = (window as any).__compositor.store.getState();
  return { activeId: s.activeId, order: s.order, modified: s.activeId ? s.documents[s.activeId].isModified : null, busy: s.busy };
});
async function requestClose(page: Page) {
  await page.getByTestId("project-tab").last().getByRole("button").click();
  await expect(closeDialog(page)).toBeVisible();
}

test("Cancel Close and Escape retain the modified project; keyboard focus stays in the choice", async ({ page }) => {
  const id = await dirtyProject(page);
  await requestClose(page);
  const cancel = closeDialog(page).getByRole("button", { name: "Cancel Close", exact: true });
  await expect(cancel).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(closeDialog(page).getByRole("button", { name: "Save and Close", exact: true })).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(closeDialog(page).getByRole("button", { name: "Don't Save and Close", exact: true })).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(cancel).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(closeDialog(page)).toHaveCount(0);
  expect(await snapshot(page)).toEqual({ activeId: id, order: [id], modified: true, busy: false });
  await requestClose(page);
  await page.keyboard.press("Escape");
  await expect(closeDialog(page)).toHaveCount(0);
  expect(await snapshot(page)).toEqual({ activeId: id, order: [id], modified: true, busy: false });
});

test("cancelling Save As keeps changes, then Don't Save and Close clears the last document and canvas", async ({ page }) => {
  const id = await dirtyProject(page);
  await expect.poll(() => page.evaluate(() => {
    const pixels = (window as any).__compositor.renderer.readPixels();
    return pixels.some((v: number, i: number) => i % 4 === 0 && v === 255 && pixels[i + 1] === 0 && pixels[i + 2] === 0);
  })).toBe(true);
  await page.evaluate(() => (window as any).__compositor.bridge.setNextPick(null));
  await requestClose(page);
  await closeDialog(page).getByRole("button", { name: "Save and Close", exact: true }).click();
  await expect(closeDialog(page)).toHaveCount(0);
  await expect(page.getByTestId("working")).toHaveCount(0);
  expect(await snapshot(page)).toEqual({ activeId: id, order: [id], modified: true, busy: false });
  await requestClose(page);
  await closeDialog(page).getByRole("button", { name: "Don't Save and Close", exact: true }).click();
  await expect(page.getByTestId("project-tab")).toHaveCount(0);
  await expect(page.getByTestId("layer-row")).toHaveCount(0);
  await expect(page.getByText("Open an image or create a new canvas", { exact: true })).toBeVisible();
  expect(await snapshot(page)).toEqual({ activeId: null, order: [], modified: null, busy: false });
  await expect.poll(() => page.evaluate(() => {
    const pixels = (window as any).__compositor.renderer.readPixels();
    return pixels.every((v: number, i: number) => Math.abs(v - (i % 4 === 3 ? 255 : 41)) <= 1);
  })).toBe(true);
  await clickMenu(page, "File", "new");
  await page.getByRole("button", { name: "Create", exact: true }).click();
  await expect(page.getByTestId("layer-row")).toHaveCount(1);
  expect(await page.evaluate(() => {
    const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId].layers.some((l: any) => l.hasPixels);
  })).toBe(false);
});

test("Save and Close finishes the package write before closing; reopening preserves imported pixels", async ({ page }) => {
  await dirtyProject(page);
  const original = await page.evaluate(() => Array.from((window as any).__compositor.readDocumentPixels()));
  await page.evaluate(() => (window as any).__compositor.bridge.setNextPick("C:/projects/CloseSaved.comp"));
  await requestClose(page);
  await closeDialog(page).getByRole("button", { name: "Save and Close", exact: true }).click();
  await expect(page.getByTestId("project-tab")).toHaveCount(0);
  expect(await page.evaluate(() => (window as any).__compositor.bridge.hasPackage("C:/projects/CloseSaved.comp"))).toBe(true);
  await page.evaluate(() => (window as any).__compositor.bridge.setNextPick("C:/projects/CloseSaved.comp"));
  await clickMenu(page, "File", "open");
  await expect(page.getByTestId("project-tab")).toContainText("CloseSaved");
  expect((await snapshot(page)).modified).toBe(false);
  expect(await page.evaluate(() => Array.from((window as any).__compositor.readDocumentPixels()))).toEqual(original);
});

test("failed saving during close leaves the project modified and reports the error", async ({ page }) => {
  const id = await dirtyProject(page);
  await page.evaluate(() => {
    const bridge = (window as any).__compositor.bridge;
    bridge.setNextPick("C:/projects/CloseFailed.comp"); bridge.failNextWrite("disk is full");
  });
  await requestClose(page);
  await closeDialog(page).getByRole("button", { name: "Save and Close", exact: true }).click();
  await expect(page.getByTestId("error-banner")).toContainText("disk is full");
  expect(await snapshot(page)).toEqual({ activeId: id, order: [id], modified: true, busy: false });
  expect(await page.evaluate(() => (window as any).__compositor.bridge.hasPackage("C:/projects/CloseFailed.comp"))).toBe(false);
});

test("closing a saved project with a pending transform offers a choice and saves the visible placement", async ({ page }) => {
  await dirtyProject(page);
  await page.evaluate(() => (window as any).__compositor.bridge.setNextPick("C:/projects/TransformClose.comp"));
  await clickMenu(page, "File", "save");
  await expect(page.getByTestId("project-tab")).not.toContainText("\u2022");
  await page.evaluate(() => {
    const s = (window as any).__compositor.store.getState();
    s.beginTransform({ persistent: true });
    const edit = (window as any).__compositor.store.getState().transformEdit;
    s.previewTransform({ ...edit.draft, origin: [5, 7] }, null);
  });
  await requestClose(page);
  await closeDialog(page).getByRole("button", { name: "Cancel Close", exact: true }).click();
  expect(await page.evaluate(() => (window as any).__compositor.store.getState().transformEdit.draft.origin)).toEqual([5, 7]);
  await requestClose(page);
  await closeDialog(page).getByRole("button", { name: "Save and Close", exact: true }).click();
  await expect(page.getByTestId("project-tab")).toHaveCount(0);
  await page.evaluate(() => (window as any).__compositor.bridge.setNextPick("C:/projects/TransformClose.comp"));
  await clickMenu(page, "File", "open");
  await expect(page.getByTestId("layer-row")).toHaveCount(1);
  expect(await page.evaluate(() => {
    const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId].layers[0].transform.origin;
  })).toEqual([5, 7]);
});

test("closing a modified background tab saves that project and preserves the active project", async ({ page }) => {
  const background = await dirtyProject(page);
  const b64 = await page.evaluate(redSquarePngBase64);
  const active = await page.evaluate(b64 => {
    const api = (window as any).__compositor;
    const id = api.engine.newDocument(32, 24, true);
    api.engine.importImage(id, Uint8Array.from(atob(b64), (c: string) => c.charCodeAt(0)), "active", null);
    api.store.getState().openDocument(id);
    api.store.getState().beginAdjust({ kind: "Levels" });
    api.bridge.setNextPick("C:/projects/Background.comp"); return id as string;
  }, b64);
  await expect(page.getByTestId("adjust-panel")).toBeVisible();
  await page.getByTestId("project-tab").first().getByRole("button").click();
  await expect(closeDialog(page)).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(closeDialog(page)).toHaveCount(0);
  await expect(page.getByTestId("adjust-panel")).toBeVisible();
  await page.getByTestId("project-tab").first().getByRole("button").click();
  await expect(closeDialog(page)).toBeVisible();
  // A second close request cannot stack a confirmation or close another document.
  await page.evaluate(() => (document.querySelectorAll('[data-testid="project-tab"] button')[1] as HTMLButtonElement).click());
  await expect(closeDialog(page)).toHaveCount(1);
  expect((await snapshot(page)).activeId).toBe(active);
  await expect(page.getByTestId("adjust-panel")).toBeVisible();
  await closeDialog(page).getByRole("button", { name: "Save and Close", exact: true }).click();
  await expect(page.getByTestId("project-tab")).toHaveCount(1);
  expect((await snapshot(page)).order).toEqual([active]);
  expect((await snapshot(page)).activeId).toBe(active);
  const saved = await page.evaluate(async () => {
    const api = (window as any).__compositor;
    const files = await api.bridge.readPackage("C:/projects/Background.comp");
    const id = api.engine.openPackage(files, "C:/projects/Background.comp");
    return { width: api.engine.state(id).width, height: api.engine.state(id).height };
  });
  expect(saved).toEqual({ width: 2, height: 2 });
  expect(background).not.toBe(active);
});
