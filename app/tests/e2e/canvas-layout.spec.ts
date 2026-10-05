import { test, expect, type Page } from "@playwright/test";

// Deliver ResizeObserver later than React's toolbar commit. This forces the
// geometry race found by the original shape-tool test on the Windows runner.
async function deferResizeNotifications(page: Page) {
  await page.addInitScript(() => {
    const NativeObserver = window.ResizeObserver;
    const control = { hold: false, pending: [] as (() => void)[] };
    (window as any).__resizeControl = control;
    window.ResizeObserver = class implements ResizeObserver {
      private native: ResizeObserver;
      constructor(callback: ResizeObserverCallback) {
        this.native = new NativeObserver(entries => {
          const deliver = () => callback(entries, this);
          if (control.hold) control.pending.push(deliver); else deliver();
        });
      }
      observe(target: Element, options?: ResizeObserverOptions) { this.native.observe(target, options); }
      unobserve(target: Element) { this.native.unobserve(target); }
      disconnect() { this.native.disconnect(); }
    };
  });
}

test("toolbar layout is mapped before deferred resize callbacks and the first shape drag", async ({ page }) => {
  await deferResizeNotifications(page);
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  await page.evaluate(async () => {
    const api = (window as any).__compositor;
    const s = api.store.getState();
    s.openDocument(api.engine.newDocument(100, 80, true));
    await api.setZoom(4);
    s.setPaletteColor({ red: 1, green: 0, blue: 0 }, false);
    s.run({ type: "SelectAll" });
  });
  // A known toolbar height change, independent of the runner's native fonts.
  await page.addStyleTag({ content: '[data-testid="shape-options"] { height: 88px; flex-shrink: 0; }' });
  const before = await page.evaluate(() => {
    const s = (window as any).__compositor.store.getState();
    (window as any).__resizeControl.hold = true;
    return s.viewports[s.activeId].viewSize.height;
  });
  await page.keyboard.press("u");
  await expect(page.getByTestId("shape-options")).toBeVisible();
  const coordinates = await page.evaluate(() => {
    const api = (window as any).__compositor, s = api.store.getState();
    const vp = s.viewports[s.activeId], doc = s.documents[s.activeId];
    const el = document.querySelector('[data-testid="canvas-view"]') as HTMLElement;
    const r = el.getBoundingClientRect();
    const client = (x: number, y: number) => {
      const p = vp.viewPoint({ x, y }, doc);
      return { x: r.left + p.x, y: r.top + p.y };
    };
    return { mapped: { ...vp.viewSize }, actual: { width: el.clientWidth, height: el.clientHeight },
      a: client(10, 10), b: client(40, 30), depth: doc.undoDepth };
  });
  expect(coordinates.actual.height).not.toBe(before);
  expect(coordinates.mapped).toEqual(coordinates.actual);
  await page.evaluate(() => {
    const control = (window as any).__resizeControl;
    control.hold = false;
    for (const deliver of control.pending.splice(0)) deliver();
  });
  await page.mouse.move(coordinates.a.x, coordinates.a.y);
  await page.mouse.down();
  await page.mouse.move(coordinates.b.x, coordinates.b.y, { steps: 3 });
  await page.mouse.up();
  const result = await page.evaluate(() => {
    const s = (window as any).__compositor.store.getState(), doc = s.documents[s.activeId];
    const layer = doc.layers.find((l: any) => l.id === doc.activeLayerId);
    return { origin: layer.transform.origin, size: layer.transform.size, depth: doc.undoDepth, selection: doc.selection };
  });
  expect(result.origin).toEqual([10, 10]);
  expect(result.size).toEqual([30, 20]);
  expect(result.depth).toBe(coordinates.depth + 1);
  expect(result.selection).not.toBeNull();
});
