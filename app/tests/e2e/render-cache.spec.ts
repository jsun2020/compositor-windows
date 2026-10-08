import { test, expect } from "@playwright/test";

test("retained GPU compositions remain byte exact across edits, previews, masks, views and document sessions", async ({ page }) => {
  await page.goto("/"); await expect(page.getByTestId("engine-ready")).toBeVisible();
  const result = await page.evaluate(async () => {
    const api = (window as any).__compositor, e = api.engine, r = api.renderer;
    const doc = e.newDocument(32, 24, true), layer = e.state(doc).layers[0].id;
    api.store.getState().openDocument(doc); await api.setZoom(4);
    await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
    e.execute(doc, { type: "Fill", id: layer, mask: false, color: [.7, .2, .4] });
    let active = doc, options = { checkerboard: false }, preview: any = null;
    const vp = api.store.getState().viewports[doc], gl = r.gl as WebGL2RenderingContext;
    const draw = gl.drawArrays.bind(gl); let offscreen = 0;
    gl.drawArrays = (...args) => { if (gl.getParameter(gl.DRAW_FRAMEBUFFER_BINDING)) offscreen++; draw(...args); };
    const render = () => r.render(e, e.state(active), vp, window.devicePixelRatio || 1, options, preview);
    const rows: { label: string; exact: boolean; reused: boolean }[] = [];
    const check = (label: string) => {
      render(); const cached = r.readPixels() as Uint8Array;
      r.clear(); render(); const fresh = r.readPixels() as Uint8Array;
      offscreen = 0; render(); const stable = r.readPixels() as Uint8Array;
      rows.push({ label, exact: cached.length === fresh.length && cached.every((v, i) => v === fresh[i]) && stable.every((v, i) => v === fresh[i]), reused: offscreen === 0 });
    };
    try {
      check("initial");
      e.execute(doc, { type: "SetLayerOpacity", id: layer, opacity: .63 }); check("opacity");
      e.execute(doc, { type: "Fill", id: layer, mask: false, color: [.1, .6, .9] }); check("pixels");
      e.undo(doc); check("undo"); e.redo(doc); check("redo");
      e.execute(doc, { type: "AddMask", id: layer, revealing: true }); check("mask added");
      e.execute(doc, { type: "InvertMask", id: layer }); check("mask pixels");
      e.execute(doc, { type: "SetMaskEnabled", id: layer, enabled: false }); check("mask disabled");
      e.undo(doc); check("mask enabled by undo");
      options = { checkerboard: true }; check("presentation checkerboard");
      e.execute(doc, { type: "DeleteMask", id: layer }); check("mask deleted");
      const transform = e.state(doc).layers[0].transform;
      preview = { kind: "layer", id: layer, draft: { ...transform, origin: [3.25, 2.5], rotation: 17 } }; check("transform preview");
      preview = null; check("preview cancelled");
      e.execute(doc, { type: "SetLayerTransform", id: layer, transform: { ...transform, sampling: "Nearest", origin: [.25, .5] } }); check("sampling and transform");
      vp.translate({ width: 11, height: -7 }); check("pan");
      vp.setZoom(2, vp.center, e.state(doc)); check("zoom");
      const oldSize = { ...vp.viewSize }; vp.resize({ width: oldSize.width - 13, height: oldSize.height - 9 }, vp.backingScale); check("resize");
      vp.resize(oldSize, vp.backingScale); check("resize restored");
      const other = e.openPackage(e.savePackage(doc), null); active = other;
      check("new document handle with identical layer ids");
      e.execute(other, { type: "Fill", id: layer, mask: false, color: [.9, .8, .1] }); check("second document edited");
      active = doc; check("original document restored");
      e.closeDocument(other);
      r.clear(); offscreen = 0; render(); const redrawAfterClear = offscreen > 0;
      return { kind: api.store.getState().rendererKind, rows, redrawAfterClear, glError: gl.getError() };
    } finally { gl.drawArrays = draw; api.store.getState().closeDocument(doc); }
  });
  expect(result.kind).toBe("gl"); expect(result.glError).toBe(0); expect(result.redrawAfterClear).toBe(true);
  for (const row of result.rows) { expect(row.exact, row.label).toBe(true); expect(row.reused, row.label).toBe(true); }
});
