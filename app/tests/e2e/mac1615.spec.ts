import { expect, test } from "@playwright/test";
import { clickMenu, noisePngBase64 } from "./helpers";

test("Extract Depth uses the bundled model and creates one undoable grayscale layer", async ({
  page,
}) => {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const png = await page.evaluate(noisePngBase64);
  const before = await page.evaluate((data) => {
    const api = (window as any).__compositor;
    const doc = api.engine.newDocument(100, 90, false);
    api.engine.importImage(
      doc,
      Uint8Array.from(atob(data), (c) => c.charCodeAt(0)),
      "Subject",
      { x: 45, y: 35 },
    );
    api.store.getState().openDocument(doc);
    return api.store.getState().documents[doc];
  }, png);
  const source = before.layers[before.layers.length - 1];
  await clickMenu(page, "Filter", "extract-depth");
  await expect
    .poll(
      () =>
        page.evaluate(() => {
          const s = (window as any).__compositor.store.getState();
          return s.error || s.documents[s.activeId].layers.at(-1)?.name;
        }),
      { timeout: 20_000 },
    )
    .toBe("Depth");
  const after = await page.evaluate(() => {
    const api = (window as any).__compositor,
      s = api.store.getState(),
      doc = s.documents[s.activeId];
    const pixels: Uint8Array = api.engine.layerPixels(
      s.activeId,
      doc.activeLayerId,
    );
    const gray = new Set<number>();
    for (let i = 0; i < pixels.length; i += 4) {
      if (
        pixels[i] !== pixels[i + 1] ||
        pixels[i] !== pixels[i + 2] ||
        pixels[i + 3] !== 255
      )
        throw Error("Invalid depth pixel");
      gray.add(pixels[i]);
    }
    return { doc, tones: gray.size };
  });
  const layer = after.doc.layers.at(-1);
  expect([layer.pixelsWidth, layer.pixelsHeight]).toEqual([
    source.pixelsWidth,
    source.pixelsHeight,
  ]);
  expect(layer.transform).toEqual(source.transform);
  expect(
    after.doc.layers.find((l: { id: string }) => l.id === source.id)
      .pixelsRevision,
  ).toBe(source.pixelsRevision);
  expect(after.doc.undoDepth).toBe(before.undoDepth + 1);
  expect(after.tones).toBeGreaterThan(8);
  await clickMenu(page, "Edit", "undo");
  await expect(page.getByText("Depth", { exact: true })).toHaveCount(0);
});

for (const [kind, title] of [
  ["rim-light", "Rim Light"],
  ["volumetric-light", "Volumetric Light"],
] as const) {
  test(`${title} previews locally, applies inside the raster and can be undone`, async ({
    page,
  }) => {
    await page.goto("/");
    await expect(page.getByTestId("engine-ready")).toBeVisible();
    const before = await page.evaluate(() => {
      const api = (window as any).__compositor,
        doc = api.engine.newDocument(96, 72, false);
      const el = document.createElement("canvas");
      el.width = 96;
      el.height = 72;
      const ctx = el.getContext("2d")!;
      ctx.fillStyle = "#747d88";
      ctx.fillRect(20, 12, 56, 48);
      const data = Uint8Array.from(atob(el.toDataURL().split(",")[1]), (c) =>
        c.charCodeAt(0),
      );
      api.engine.importImage(doc, data, "Cutout", { x: 48, y: 36 });
      api.store.getState().openDocument(doc);
      return api.store.getState().documents[doc];
    });
    await clickMenu(page, "Filter", kind);
    const panel = page.getByRole("dialog", { name: title });
    const apply = panel.getByRole("button", { name: "Apply", exact: true });
    await expect(apply).toBeEnabled({ timeout: 20_000 });
    if (kind === "rim-light") {
      await panel.getByRole("button", { name: "Add light" }).click();
      await expect(panel.getByLabel("Light 2 color")).toHaveValue("#b3d6ff");
    } else {
      await panel
        .getByLabel("Lighting preview")
        .click({ position: { x: 8, y: 8 } });
      await panel.getByLabel("Keep subject clear").uncheck();
    }
    await expect(apply).toBeEnabled();
    await apply.click();
    await expect(panel).toHaveCount(0);
    const result = await page.evaluate(() => {
      const api = (window as any).__compositor,
        s = api.store.getState(),
        doc = s.documents[s.activeId];
      const pixels: Uint8Array = api.engine.layerPixels(
        s.activeId,
        doc.activeLayerId,
      );
      let changed = 0;
      for (let i = 0; i < pixels.length; i += 4)
        if (
          pixels[i + 3] === 255 &&
          (pixels[i] !== 116 || pixels[i + 1] !== 125 || pixels[i + 2] !== 136)
        )
          changed++;
      return { depth: doc.undoDepth, changed, corner: pixels[3] };
    });
    expect(result.depth).toBe(before.undoDepth + 1);
    expect(result.changed).toBeGreaterThan(10);
    expect(result.corner).toBe(0);
    await clickMenu(page, "Edit", "undo");
    expect(
      await page.evaluate(() => {
        const a = (window as any).__compositor,
          s = a.store.getState(),
          d = s.documents[s.activeId];
        return Array.from(
          a.engine
            .layerPixels(s.activeId, d.activeLayerId)
            .slice((36 * 96 + 48) * 4, (36 * 96 + 48) * 4 + 4),
        );
      }),
    ).toEqual([116, 125, 136, 255]);
  });
}

test("exporting multiple layers writes trimmed PNGs and asks before replacing them", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  await page.evaluate(() => {
    const {engine,store,bridge} = (window as any).__compositor;
    const doc = engine.newDocument(40, 24, false);
    const ids = [8, 12].map(width => {
      engine.execute(doc, {type:"AddShape", shape:{kind:"Rectangle",rect:{x:4,y:3,width,height:6},cornerRadius:0},color:[1,0,0]});
      const id = engine.state(doc).activeLayerId;
      engine.execute(doc,{type:"RenameLayer",id,name:"A/B"});
      return id;
    });
    store.getState().openDocument(doc);
    store.getState().selectLayers(ids,ids[1]);
    bridge.setNextPick("C:/out");
  });
  await clickMenu(page,"File","export-layers");
  const sizes = await page.evaluate(async () => {
    const bridge = (window as any).__compositor.bridge;
    return Promise.all(["A-B.png","A-B 2.png"].map(async name => {
      const bytes = await bridge.readFile("C:/out/"+name);
      const bmp = await createImageBitmap(new Blob([bytes], {type:"image/png"}));
      const size = [bmp.width,bmp.height];
      bmp.close();
      return size;
    }));
  });
  expect(sizes).toEqual([[12,6],[8,6]]);
  await page.evaluate(() => {
    const {bridge} = (window as any).__compositor;
    bridge.seedFile("C:/out/A-B.png",new Uint8Array([17]));
    bridge.setNextPick("C:/out");
  });
  const dialog = page.waitForEvent("dialog").then(async prompt => {
    const message = prompt.message();
    await prompt.dismiss();
    return message;
  });
  await clickMenu(page,"File","export-layers");
  expect(await dialog).toContain("2 existing PNG");
  expect(await page.evaluate(() => Array.from((window as any).__compositor.bridge.fileBytes("C:/out/A-B.png")))).toEqual([17]);
});
