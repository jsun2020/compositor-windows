import { test, expect, type Page } from "@playwright/test";
import { clickMenu, solidPngBase64 } from "./helpers";

// Phase 4b-1: the palette at the foot of the tool rail and the colour picker, through the real
// swatches, keys, panel and canvas (the store's rules are pinned in palette-store and picker-store).

type Pt = [number, number];

/** A 64 x 48 document, red on the left 32 columns and blue on the right, at 4 CSS px a pixel. */
async function setup(page: Page) {
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const red = await page.evaluate(solidPngBase64, { width: 32, height: 48, color: "#ff0000" });
  const blue = await page.evaluate(solidPngBase64, { width: 32, height: 48, color: "#0000ff" });
  await page.evaluate(async ([r, b]) => {
    const api = (window as any).__compositor;
    const bytes = (data: string) => Uint8Array.from(atob(data), (c: string) => c.charCodeAt(0));
    const doc = api.engine.newDocument(64, 48, false);
    api.engine.importImage(doc, bytes(r), "Red", { x: 16, y: 24 });
    api.engine.importImage(doc, bytes(b), "Blue", { x: 48, y: 24 });
    api.engine.execute(doc, { type: "MergeLayers", ids: api.engine.state(doc).layers.map((l: any) => l.id) });
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
const palette = (page: Page) => page.evaluate(() => (window as any).__compositor.store.getState().palette);
const swatch = (page: Page, which: "foreground" | "background") => page.getByTestId(`palette-${which}`).evaluate((el) => getComputedStyle(el).backgroundColor);
const picker = (page: Page) => page.getByTestId("color-picker");
const undoDepth = (page: Page) => page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId].undoDepth; });

test("the swatches show the palette; X swaps and D resets; a picker opened on one commits on OK only", async ({ page }) => {
  await setup(page);
  const depth = await undoDepth(page);
  await expect(page.getByTestId("palette")).toBeVisible();
  expect([await swatch(page, "foreground"), await swatch(page, "background")]).toEqual(["rgb(0, 0, 0)", "rgb(255, 255, 255)"]);
  await page.keyboard.press("x");
  expect([await swatch(page, "foreground"), await swatch(page, "background")]).toEqual(["rgb(255, 255, 255)", "rgb(0, 0, 0)"]);
  await page.keyboard.press("d");
  expect(await swatch(page, "foreground")).toBe("rgb(0, 0, 0)");
  // The foreground swatch opens the picker on black; a hex typed and Enter in the field sets its
  // working colour, the swatch stays black until OK.
  await page.getByTestId("palette-foreground").click();
  await expect(picker(page)).toBeVisible();
  await expect(page.getByTestId("picker-title")).toHaveText("Color Picker (Foreground Color)");
  await page.getByTestId("picker-hex").fill("#ff8000");
  await page.getByTestId("picker-hex").press("Enter");
  await expect(page.getByTestId("picker-red")).toHaveValue("255");
  await expect(page.getByTestId("picker-green")).toHaveValue("128");
  expect(await swatch(page, "foreground")).toBe("rgb(0, 0, 0)");
  // Enter outside a field is OK.
  await page.getByTestId("picker-title").click();
  await page.keyboard.press("Enter");
  await expect(picker(page)).toBeHidden();
  expect(await swatch(page, "foreground")).toBe("rgb(255, 128, 0)");
  // Escape cancels: the background stays white.
  await page.getByTestId("palette-background").click();
  await page.getByTestId("picker-blue").fill("0");
  await page.keyboard.press("Escape");
  await expect(picker(page)).toBeHidden();
  expect(await swatch(page, "background")).toBe("rgb(255, 255, 255)");
  expect(await undoDepth(page), "the palette records no history").toBe(depth);
});

test("the picker opens where it was left, and dragging its field and hue strip sets the colour", async ({ page }) => {
  await setup(page);
  await page.getByTestId("palette-foreground").click();
  const first = await picker(page).boundingBox();
  const canvas = await page.getByTestId("canvas-view").boundingBox();
  // First opened centred on the canvas.
  expect(Math.abs(first!.x + first!.width / 2 - (canvas!.x + canvas!.width / 2))).toBeLessThanOrEqual(1);
  const title = await page.getByTestId("picker-title").boundingBox();
  await page.mouse.move(title!.x + 20, title!.y + 8); await page.mouse.down();
  await page.mouse.move(title!.x - 80, title!.y - 40, { steps: 4 }); await page.mouse.up();
  const moved = await picker(page).boundingBox();
  expect([moved!.x - first!.x, moved!.y - first!.y]).toEqual([-100, -48]);
  // Ruling M5: the field is a fixed 256 x 256 CSS px, whatever the document's size or zoom.
  const field0 = await page.getByTestId("picker-field").boundingBox();
  expect([field0!.width, field0!.height]).toEqual([256, 256]);
  // A quarter of the way down the hue strip is 270 degrees (within the pointer's pixel); the field
  // dragged past its top-right corner holds full saturation and brightness; past its bottom, none.
  const hue = await page.getByTestId("picker-hue").boundingBox();
  await page.mouse.click(hue!.x + hue!.width / 2, hue!.y + hue!.height / 4);
  const field = await page.getByTestId("picker-field").boundingBox();
  await page.mouse.move(field!.x + 100, field!.y + 100); await page.mouse.down();
  await page.mouse.move(field!.x + field!.width + 30, field!.y - 30, { steps: 3 }); await page.mouse.up();
  const hsb = await page.evaluate(() => (window as any).__compositor.store.getState().colorPicker.hsb);
  expect(Math.abs(hsb.hue - 270)).toBeLessThanOrEqual(360 / 256);
  expect([hsb.saturation, hsb.brightness]).toEqual([1, 1]);
  // Red is half of full at 270 degrees: 7F or 80, blue full, green none.
  await expect(page.getByTestId("picker-hex")).toHaveValue(/^(7F|80)00FF$/);
  await page.mouse.move(field!.x + 100, field!.y + 100); await page.mouse.down();
  await page.mouse.move(field!.x + 100, field!.y + field!.height + 40, { steps: 3 }); await page.mouse.up();
  await expect(page.getByTestId("picker-hex")).toHaveValue("000000");
  await page.getByTestId("picker-cancel").click();
  await page.getByTestId("palette-background").click();
  const again = await picker(page).boundingBox();
  expect([again!.x, again!.y]).toEqual([moved!.x, moved!.y]);
});

test("while the picker is open a press on the canvas samples it, showing the ring, and no tool acts", async ({ page }) => {
  await setup(page);
  await page.keyboard.press("m"); // the Marquee: a press would otherwise start a selection
  await page.getByTestId("palette-foreground").click();
  // Moved off the document, which it opens centred over.
  await page.evaluate(() => (window as any).__compositor.store.getState().setPickerAt({ x: 700, y: 360 }));
  const left = await client(page, [10, 20]), right = await client(page, [50, 20]);
  await page.mouse.move(left.x, left.y); await page.mouse.down();
  await expect(page.getByTestId("picker-hex")).toHaveValue("FF0000");
  // The ring is drawn about the pointer: 43 px from it, the sampled colour above and black below.
  const ring = await page.evaluate(([x, y]) => {
    const overlay = document.querySelector('[data-testid="overlay"]') as HTMLCanvasElement;
    const r = overlay.getBoundingClientRect(); const dpr = window.devicePixelRatio || 1;
    const at = (dx: number, dy: number) => Array.from(overlay.getContext("2d")!.getImageData(Math.round((x - r.left + dx) * dpr), Math.round((y - r.top + dy) * dpr), 1, 1).data);
    return { above: at(0, -43), below: at(0, 43) };
  }, [left.x, left.y]);
  expect(ring.above).toEqual([255, 0, 0, 255]);
  expect(ring.below).toEqual([0, 0, 0, 255]);
  // Dragging keeps sampling.
  await page.mouse.move(right.x, right.y, { steps: 5 });
  await expect(page.getByTestId("picker-hex")).toHaveValue("0000FF");
  await page.mouse.up();
  const state = await page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return { selection: s.documents[s.activeId].selection, draft: s.selectionDraft, ring: s.sampleRing }; });
  expect(state).toEqual({ selection: null, draft: null, ring: null });
  expect((await palette(page)).foreground).toEqual({ red: 0, green: 0, blue: 0 });
  await page.getByTestId("picker-ok").click();
  expect(await swatch(page, "foreground")).toBe("rgb(0, 0, 255)");
});

test("with a mask targeted a swatch offers black or white, and the picker does not open", async ({ page }) => {
  await setup(page);
  await page.evaluate(() => {
    const api = (window as any).__compositor; const s = api.store.getState();
    s.run({ type: "AddMask", id: s.documents[s.activeId].activeLayerId, revealing: true });
    api.store.getState().setMaskSelected(true);
  });
  await page.getByTestId("palette-background").click();
  await expect(page.getByTestId("palette-mask-popover")).toBeVisible();
  await expect(picker(page)).toBeHidden();
  await page.getByTestId("mask-black").click();
  expect([await swatch(page, "foreground"), await swatch(page, "background")]).toEqual(["rgb(255, 255, 255)", "rgb(0, 0, 0)"]);
  // The image's own colours are untouched and come back with the pixels targeted.
  await page.evaluate(() => (window as any).__compositor.store.getState().setMaskSelected(false));
  expect([await swatch(page, "foreground"), await swatch(page, "background")]).toEqual(["rgb(0, 0, 0)", "rgb(255, 255, 255)"]);
});

test("Gradient Map starts at the palette, its ends open the picker, which previews them, and Escape closes only the picker", async ({ page }) => {
  await setup(page);
  await page.evaluate(() => { const s = (window as any).__compositor.store.getState(); s.setPaletteColor({ red: 1, green: 1, blue: 0 }, false); s.setPaletteColor({ red: 0, green: 0, blue: 0 }, true); });
  await clickMenu(page, "Image", "image-gradient-map");
  await expect(page.getByTestId("adjust-panel")).toBeVisible();
  // Red and blue map by their luminance between yellow (shadows) and black (highlights).
  const shown = () => page.evaluate(() => {
    const api = (window as any).__compositor; const s = api.store.getState();
    return Array.from(api.engine.composite(s.activeId, { x: 10, y: 20, width: 1, height: 1 }, 1, 1) as Uint8Array);
  });
  const before = await shown();
  await page.getByTestId("gradient-map-shadows").click();
  await expect(page.getByTestId("picker-title")).toHaveText("Color Picker (Gradient Map Shadows)");
  await expect(page.getByTestId("picker-hex")).toHaveValue("FFFF00");
  await page.getByTestId("picker-hex").fill("00FFFF");
  await page.getByTestId("picker-hex").press("Enter");
  const previewed = await shown();
  expect(previewed, "the canvas previews the working colour").not.toEqual(before);
  await page.getByTestId("picker-title").click();
  await page.keyboard.press("Escape");
  await expect(picker(page)).toBeHidden();
  await expect(page.getByTestId("adjust-panel"), "Escape went to the picker alone").toBeVisible();
  expect(await shown(), "Cancel put the end back").toEqual(before);
  await page.getByTestId("adjust-cancel").click();
  // A new Gradient Map layer starts at the palette too.
  await clickMenu(page, "Layer", "layer-adjustment-gradient-map");
  const settings = await page.evaluate(() => { const s = (window as any).__compositor.store.getState(); const d = s.documents[s.activeId]; return d.layers.find((l: any) => l.id === d.activeLayerId).adjustment.gradientMapSettings; });
  expect([settings.shadows, settings.highlights]).toEqual([{ red: 1, green: 1, blue: 0 }, { red: 0, green: 0, blue: 0 }]);
});
