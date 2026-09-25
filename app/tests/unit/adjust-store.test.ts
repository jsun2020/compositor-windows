import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { SETTLE_MS, useEditor } from "../../src/state/store";
import { defaultAdjustment, defaultFilterParams, isAdjustIdentity, previewRequestFor, resetAdjustment } from "../../src/state/adjust-edit";
import type { Command, DocumentState, LayerAdjustment, LayerState, PreviewRequest } from "../../src/engine/types";
import { DEFAULT_BLACK_WHITE, DEFAULT_COLOR_BALANCE } from "../../src/engine/types";
import type { EngineClient } from "../../src/engine/client";
import type { ShellBridge } from "../../src/shell/bridge";
import { closeProject, importImages } from "../../src/actions/files";
import { runAction } from "../../src/shortcuts/useShortcuts";

function layer(id: string, o: Partial<LayerState> = {}): LayerState {
  return { id, name: id, visible: true, isGroup: false, parentId: null, opacity: 1, blendMode: "Normal",
    transform: { origin: [0, 0], size: [10, 10], rotation: 0, flipX: false, flipY: false, sampling: "High quality" },
    pixelsWidth: 10, pixelsHeight: 10, pixelsRevision: 1, hasPixels: true, hasMask: false, maskWidth: 0, maskHeight: 0,
    maskRevision: 0, maskEnabled: true, maskLinked: true, maskSourceId: null, maskPlacement: null, maskBackground: 255, ...o };
}
function document(layers: LayerState[], active: string): DocumentState {
  return { id: "D", documentId: "D", width: 10, height: 10, resolution: 72, activeLayerId: active, canUndo: false,
    canRedo: false, isModified: false, undoDepth: 0, path: null, guides: [], undrawn: [], layers };
}

function install(layers: LayerState[], active: string) {
  const calls: Command[] = [];
  const previews: (PreviewRequest | null)[] = [];
  const previewDocs: string[] = [];
  const history: string[] = [];
  const closed: string[] = [];
  const imports: string[] = [];
  const histograms: string[] = [];
  const autos: { bins: number[][]; mode: string }[] = [];
  const state = document(layers, active);
  const engine = {
    state: () => state,
    execute: (_id: string, cmd: Command) => { calls.push(cmd); return { structure: true, canvas: false, layers: [] }; },
    setPreview: (id: string, request: PreviewRequest | null) => { previewDocs.push(id); previews.push(request); return { structure: true, canvas: false, layers: [] }; },
    histogram: () => { histograms.push("histogram"); return [new Array(256).fill(1), new Array(256).fill(1), new Array(256).fill(1), new Array(256).fill(1)]; },
    autoLevels: (bins: number[][], mode: string) => { autos.push({ bins, mode }); const l = defaultAdjustment("Levels").levels; l.ranges[0].black = 10; return l; },
    undo: () => { history.push("undo"); return { structure: true, canvas: false, layers: [] }; },
    redo: () => { history.push("redo"); return { structure: true, canvas: false, layers: [] }; },
    closeDocument: (id: string) => { closed.push(id); },
    importImage: (doc: string | null) => { imports.push(doc ?? "new"); return doc ?? "N"; },
    // Stands in for LayerAdjustment::is_identity, which e2e exercises for real: these store tests
    // only need Levels at its defaults to count as "nothing to do".
    adjustmentIsIdentity: (a: LayerAdjustment) => JSON.stringify(a) === JSON.stringify(defaultAdjustment(a.kind)),
  } as unknown as EngineClient;
  const bridge = { readFile: async () => new Uint8Array([1]), baseName: (p: string) => p, pickImportImages: async () => ["C:/b.png"] } as unknown as ShellBridge;
  useEditor.setState({ engine, bridge, activeId: "D", documents: { D: state }, order: ["D"], selectedLayerIds: [active], maskSelected: false,
    transformEdit: null, adjustEdit: null, error: null, tool: "move", cropRect: null, sheet: null, busy: false, collapsed: {} });
  return { calls, previews, previewDocs, history, closed, imports, histograms, autos };
}

// Every test runs on fake timers, so a settled-preview timer one test schedules can never fire
// into the next test's store.
beforeEach(() => { vi.useFakeTimers(); });
afterEach(() => { vi.clearAllTimers(); vi.useRealTimers(); });

describe("adjustment panels", () => {
  beforeEach(() => useEditor.setState({ adjustEdit: null, error: null }));

  it("opening a panel on a pixel layer previews without recording anything", () => {
    const { calls, previews } = install([layer("A")], "A");
    expect(useEditor.getState().beginAdjust({ kind: "Levels" })).toBe(true);
    const edit = useEditor.getState().adjustEdit!;
    expect(edit.target).toBe("layer");
    expect(edit.histogram?.length).toBe(4);
    expect(previews.at(-1)).toBeNull();   // an identity adjustment previews nothing
    const next = defaultAdjustment("Levels");
    next.levels.ranges[0] = { ...next.levels.ranges[0], outputWhite: 0 };
    useEditor.getState().updateAdjust({ adjustment: next });
    expect((previews.at(-1) as any).preview).toBe("DragAdjustment");   // a slider tick: the quick preview first
    expect(calls).toEqual([]);
    useEditor.getState().commitAdjust();
    expect(previews.at(-1)).toBeNull();
    expect(calls).toEqual([{ type: "ApplyAdjustment", id: "A", adjustment: next }]);
    expect(useEditor.getState().adjustEdit).toBeNull();
  });

  it("an unchanged panel commits nothing and cancel clears the preview", () => {
    const { calls, previews } = install([layer("A")], "A");
    useEditor.getState().beginAdjust({ kind: "Levels" });
    useEditor.getState().commitAdjust();
    expect(calls).toEqual([]);
    expect(useEditor.getState().beginAdjust({ kind: "GaussianBlur" })).toBe(true);
    expect(useEditor.getState().adjustEdit).toMatchObject({ adjustment: null, params: { filter: "GaussianBlur", radius: 1 } });   // a filter panel
    useEditor.getState().updateAdjust({ params: { filter: "GaussianBlur", radius: 4 } });
    expect(previews.at(-1)).toEqual({ preview: "Filter", layer: "A", params: { filter: "GaussianBlur", radius: 4 } });
    useEditor.getState().cancelAdjust();
    expect(previews.at(-1)).toBeNull();
    expect(calls).toEqual([]);
    expect(useEditor.getState().adjustEdit).toBeNull();
  });

  it("editing an adjustment layer previews through the plan and commits SetAdjustment", () => {
    const adjustment = defaultAdjustment("Curves");
    const { calls, previews } = install([layer("A"), layer("J", { hasPixels: false, pixelsWidth: 0, adjustment })], "J");
    expect(useEditor.getState().beginAdjust({ kind: "Curves", layerId: "J", target: "adjustmentLayer" })).toBe(true);
    const next = defaultAdjustment("Curves");
    next.curves.channels[0] = [{ x: 0, y: 255 }, { x: 255, y: 255 }];
    useEditor.getState().updateAdjust({ adjustment: next });
    expect(previews).toEqual([]);   // no pixel preview: the plan carries it
    expect(useEditor.getState().previewEdit()).toEqual({ kind: "adjustment", id: "J", adjustment: next });
    useEditor.getState().commitAdjust();
    expect(calls).toEqual([{ type: "SetAdjustment", id: "J", adjustment: next }]);
    expect(useEditor.getState().previewEdit()).toBeNull();
  });

  it("a panel owns the document: other commands, undo and redo are refused while it is open", () => {
    const { calls, history } = install([layer("A")], "A");
    useEditor.getState().beginAdjust({ kind: "Levels" });
    useEditor.getState().run({ type: "AddBlankLayer" });
    useEditor.getState().undo();
    useEditor.getState().redo();
    expect(calls).toEqual([]);
    expect(history).toEqual([]);   // the engine's undo/redo must never be reached, not just no-op
    expect(useEditor.getState().error).toMatch(/Apply or cancel/);
    expect(useEditor.getState().adjustEdit).not.toBeNull();
  });

  it("a panel owns the document: beginTransform is refused too (an Alt-drag duplicate must not record while a panel is open)", () => {
    const { calls } = install([layer("A")], "A");
    useEditor.getState().beginAdjust({ kind: "Levels" });
    expect(useEditor.getState().beginTransform({ persistent: true, duplicate: true })).toBe(false);
    expect(useEditor.getState().transformEdit).toBeNull();
    expect(calls).toEqual([]);
  });

  it("switching documents clears the panel and clears the preview on the document it belonged to, not the one being switched to", () => {
    const { previewDocs } = install([layer("A")], "A");
    useEditor.setState((s) => ({ documents: { ...s.documents, E: document([layer("B")], "B") }, order: [...s.order, "E"] }));
    useEditor.getState().beginAdjust({ kind: "Levels" });
    expect(useEditor.getState().adjustEdit).not.toBeNull();
    useEditor.getState().setActive("E");
    expect(useEditor.getState().adjustEdit).toBeNull();
    expect(useEditor.getState().activeId).toBe("E");
    // The clearing setPreview must target D (where the panel was), not E (where we are going).
    expect(previewDocs.at(-1)).toBe("D");
  });

  it("closing a background document leaves a panel open on the active document untouched", () => {
    const { previewDocs, closed } = install([layer("A")], "A");
    useEditor.setState((s) => ({ documents: { ...s.documents, E: document([layer("B")], "B") }, order: [...s.order, "E"] }));
    useEditor.getState().beginAdjust({ kind: "Levels" });
    expect(useEditor.getState().adjustEdit).not.toBeNull();
    const previewCallsBefore = previewDocs.length;
    useEditor.getState().closeDocument("E");
    expect(closed).toEqual(["E"]);
    expect(useEditor.getState().adjustEdit).not.toBeNull();      // D's panel is untouched
    expect(previewDocs.length).toBe(previewCallsBefore);         // no setPreview call for the close at all
    expect(useEditor.getState().documents.E).toBeUndefined();
    expect(useEditor.getState().activeId).toBe("D");
  });

  it("refuses to open on a folder, a hidden layer or a layer with no pixels", () => {
    install([layer("F", { isGroup: true, hasPixels: false })], "F");
    expect(useEditor.getState().beginAdjust({ kind: "Levels" })).toBe(false);
    install([layer("H", { visible: false })], "H");
    expect(useEditor.getState().beginAdjust({ kind: "Levels" })).toBe(false);
    install([layer("B", { hasPixels: false, pixelsWidth: 0 })], "B");
    expect(useEditor.getState().beginAdjust({ kind: "Levels" })).toBe(false);
    expect(useEditor.getState().canAdjust()).toBe(false);
  });

  it("reopening a customised adjustment layer unchanged commits nothing", () => {
    // Regression for isAdjustIdentity comparing against defaultAdjustment(kind) instead of
    // edit.original: a customised adjustment layer is never equal to the identity default, so
    // that comparison judged every no-op reopen "changed" and issued a spurious SetAdjustment.
    const custom = defaultAdjustment("Curves");
    custom.curves.channels[0] = [{ x: 0, y: 40 }, { x: 255, y: 220 }];
    const { calls } = install([layer("A"), layer("J", { hasPixels: false, pixelsWidth: 0, adjustment: custom })], "J");
    useEditor.getState().beginAdjust({ kind: "Curves", layerId: "J", target: "adjustmentLayer" });
    useEditor.getState().commitAdjust();
    expect(calls).toEqual([]);
  });

  it("knows the default settings, and asks the engine which destructive ones do nothing", () => {
    expect(defaultAdjustment("Grain").grainSettings?.amount).toBe(25);
    expect(defaultFilterParams("MotionBlur")).toEqual({ filter: "MotionBlur", angle: 0, distance: 10 });
    const grain = defaultAdjustment("Grain");
    const asked: unknown[] = [];
    // A destructive edit takes the engine's answer, whatever the default looks like...
    expect(isAdjustIdentity({ adjustment: grain, params: null, original: null }, (a) => { asked.push(a); return false; })).toBe(false);
    expect(asked).toEqual([grain]);
    expect(isAdjustIdentity({ adjustment: defaultAdjustment("Levels"), params: null, original: null }, () => true)).toBe(true);
    expect(previewRequestFor({ target: "layer", layerId: "A", kind: "Grain", adjustment: grain, params: null, original: null, preview: true, sampleMode: null, histogram: null }, () => false))
      .toEqual({ preview: "Adjustment", layer: "A", adjustment: grain });
    // ...and an adjustment layer compares with the settings it opened with, never the engine's rule.
    expect(isAdjustIdentity({ adjustment: grain, params: null, original: grain }, () => false)).toBe(true);
  });

  it("Auto Levels reuses the histogram the panel opened with instead of recompositing", () => {
    const { histograms, autos } = install([layer("A")], "A");
    useEditor.getState().beginAdjust({ kind: "Levels" });
    const opened = useEditor.getState().adjustEdit!.histogram;
    useEditor.getState().autoLevels("Contrast");
    expect(histograms).toEqual(["histogram"]);   // computed once, when the panel opened
    expect(autos).toEqual([{ bins: opened, mode: "Contrast" }]);
    expect(useEditor.getState().adjustEdit!.adjustment!.levels.ranges[0].black).toBe(10);
  });

  it("each destructive Grain panel draws its own seed", () => {
    install([layer("A")], "A");
    const random = vi.spyOn(Math, "random").mockReturnValue(0.5);
    useEditor.getState().beginAdjust({ kind: "Grain" });
    random.mockRestore();
    expect(useEditor.getState().adjustEdit!.adjustment!.grainSettings!.seed).toBe(Math.floor(0.5 * 0xffffffff));
  });

  it("a refused OK keeps the panel open with the user's settings and its preview", () => {
    const { previews } = install([layer("A")], "A");
    const engine = useEditor.getState().engine as unknown as { execute: () => never };
    engine.execute = () => { throw new Error("adjustment settings out of range"); };
    openPreviewingLevels();
    const edit = useEditor.getState().adjustEdit;
    useEditor.getState().commitAdjust();
    expect(useEditor.getState().adjustEdit).toEqual(edit);
    expect(useEditor.getState().error).toMatch(/out of range/);
    expect((previews.at(-1) as any)?.preview).toBe("Adjustment");   // the preview is back on the canvas
  });

  it("Reset keeps what a panel cannot choose, and adds nothing an untouched layer lacks", () => {
    const grainLayer = { ...defaultAdjustment("Grain"), grainSettings: { amount: 60, size: 4, roughness: 10, seed: 7 } };
    expect(resetAdjustment(grainLayer, grainLayer).grainSettings).toEqual({ amount: 25, size: 1.5, roughness: 50, seed: 7 });
    const destructiveGrain = { ...defaultAdjustment("Grain"), grainSettings: { amount: 60, size: 4, roughness: 10, seed: 9 } };
    expect(resetAdjustment(destructiveGrain, null).grainSettings!.seed).toBe(9);
    const colours = { shadows: { red: 0.2, green: 0.1, blue: 0 }, highlights: { red: 1, green: 0.9, blue: 0.5 }, reversed: true };
    const mapLayer = { ...defaultAdjustment("Gradient Map"), gradientMapSettings: colours };
    expect(resetAdjustment(mapLayer, mapLayer).gradientMapSettings).toEqual({ ...colours, reversed: false });
    const exposureLayer = defaultAdjustment("Exposure");
    delete exposureLayer.exposureSettings;   // as AddAdjustmentLayer makes it: absent means neutral
    const edited = { ...exposureLayer, exposureSettings: { exposure: 2, offset: 0, gamma: 1 } };
    expect(resetAdjustment(edited, exposureLayer)).toEqual(exposureLayer);
  });

  it("resets the 1.2.6 kinds to the Mac's defaults, keeping Add Noise's seed and adding no key the layer lacked", () => {
    // The Mac's literal defaults (ImageAdjustments.swift:117-126, :148-157), not the constants
    // under test, so a wrong default in types.ts fails here.
    const macBlackWhite = { reds: 40, yellows: 60, greens: 40, cyans: 60, blues: 20, magentas: 80, tint: false, tintHue: 40, tintSaturation: 20 };
    const macColorBalance = { shadowCyanRed: 0, shadowMagentaGreen: 0, shadowYellowBlue: 0, midCyanRed: 0, midMagentaGreen: 0, midYellowBlue: 0,
      highlightCyanRed: 0, highlightMagentaGreen: 0, highlightYellowBlue: 0, preserveLuminosity: true };
    const bw = { ...defaultAdjustment("Black & White"), blackWhiteSettings: { ...DEFAULT_BLACK_WHITE, reds: 150, tint: true } };
    expect(resetAdjustment(bw, bw).blackWhiteSettings).toEqual(macBlackWhite);
    const fresh = defaultAdjustment("Black & White");
    expect(resetAdjustment({ ...fresh, blackWhiteSettings: { ...DEFAULT_BLACK_WHITE, reds: 150 } }, fresh).blackWhiteSettings).toBeUndefined();
    const cb = { ...defaultAdjustment("Color Balance"), colorBalanceSettings: { ...DEFAULT_COLOR_BALANCE, midYellowBlue: -40, preserveLuminosity: false } };
    expect(resetAdjustment(cb, cb).colorBalanceSettings).toEqual(macColorBalance);
    const blur = { ...defaultAdjustment("Gaussian Blur"), blurRadius: 24 };
    expect(resetAdjustment(blur, blur).blurRadius).toBe(10);
    const motion = { ...defaultAdjustment("Motion Blur"), motionAngle: 30, motionDistance: 80 };
    expect(resetAdjustment(motion, motion)).toMatchObject({ motionAngle: 0, motionDistance: 10 });
    const noise = { ...defaultAdjustment("Add Noise"), noiseAmount: 60, noiseGaussian: true, noiseMonochromatic: true, noiseSeed: 4242 };
    expect(resetAdjustment(noise, noise)).toMatchObject({ noiseAmount: 10, noiseGaussian: false, noiseMonochromatic: false, noiseSeed: 4242 });
  });

  it("Reset then OK on an untouched adjustment layer records nothing", () => {
    const exposureLayer = defaultAdjustment("Exposure");
    delete exposureLayer.exposureSettings;
    const { calls } = install([layer("A"), layer("J", { hasPixels: false, pixelsWidth: 0, adjustment: exposureLayer })], "J");
    useEditor.getState().beginAdjust({ kind: "Exposure", layerId: "J", target: "adjustmentLayer" });
    const edit = useEditor.getState().adjustEdit!;
    useEditor.getState().updateAdjust({ adjustment: resetAdjustment(edit.adjustment!, edit.original) });
    useEditor.getState().commitAdjust();
    expect(calls).toEqual([]);
  });
});

/** Opens a destructive Levels panel whose settings actually preview something. */
function openPreviewingLevels() {
  useEditor.getState().beginAdjust({ kind: "Levels" });
  const next = defaultAdjustment("Levels");
  next.levels.ranges[0] = { ...next.levels.ranges[0], white: 128 };
  useEditor.getState().updateAdjust({ adjustment: next });
}

describe("a panel owns the document: every other way in is refused", () => {
  beforeEach(() => useEditor.setState({ adjustEdit: null, error: null }));

  it("a layer selection is refused, so it cannot run SetActiveLayer and clear the preview", () => {
    const { calls, previews } = install([layer("A"), layer("B")], "A");
    openPreviewingLevels();
    const previewCalls = previews.length;
    useEditor.getState().selectLayers(["B"], "B");
    expect(calls).toEqual([]);   // no SetActiveLayer: every engine execute clears the preview
    expect(previews.length).toBe(previewCalls);
    expect(useEditor.getState().selectedLayerIds).toEqual(["A"]);
    expect(useEditor.getState().error).toMatch(/Apply or cancel/);
    useEditor.getState().setMaskSelected(true);
    expect(useEditor.getState().maskSelected).toBe(false);
  });

  it("collapsing the folder around the active layer is refused along with the selection it makes", () => {
    const { calls } = install([layer("F", { isGroup: true, hasPixels: false }), layer("A", { parentId: "F" })], "A");
    openPreviewingLevels();
    useEditor.getState().toggleCollapsed("F");
    expect(calls).toEqual([]);
    expect(useEditor.getState().collapsed.D ?? []).toEqual([]);
    expect(useEditor.getState().error).toMatch(/Apply or cancel/);
  });

  it("a sheet does not open over a panel", () => {
    install([layer("A")], "A");
    openPreviewingLevels();
    useEditor.getState().openSheet({ kind: "canvasSize" });
    expect(useEditor.getState().sheet).toBeNull();
    expect(useEditor.getState().error).toMatch(/Apply or cancel/);
  });

  it("importing into the document is refused", async () => {
    const { imports } = install([layer("A")], "A");
    openPreviewingLevels();
    await importImages(["C:/b.png"]);
    expect(imports).toEqual([]);
    expect(useEditor.getState().error).toMatch(/Apply or cancel/);
    useEditor.getState().cancelAdjust();
    await importImages(["C:/b.png"]);
    expect(imports).toEqual(["D"]);   // the same import goes through once the panel is gone
  });

  it("opening a panel drops the crop rectangle, and Enter and Escape reach only the panel", () => {
    const { calls } = install([layer("A")], "A");
    useEditor.getState().setTool("crop");
    expect(useEditor.getState().cropRect).not.toBeNull();   // the tool seeds a full-canvas rectangle
    openPreviewingLevels();
    expect(useEditor.getState().cropRect).toBeNull();        // as macOS's beginFilter calls cancelCrop
    // A rectangle dragged while the panel is open still must not answer the panel's keys.
    useEditor.getState().setCropRect({ x: 0, y: 0, width: 5, height: 5 });
    runAction("apply");
    runAction("cancel");
    expect(calls).toEqual([]);
    expect(useEditor.getState().error).toBeNull();
    expect(useEditor.getState().cropRect).toEqual({ x: 0, y: 0, width: 5, height: 5 });
    expect(useEditor.getState().adjustEdit).not.toBeNull();
  });

  it("clicking the active document's tab keeps its panel and its preview", () => {
    const { previews, previewDocs } = install([layer("A")], "A");
    openPreviewingLevels();
    const previewCalls = previewDocs.length;
    useEditor.getState().setActive("D");
    expect(useEditor.getState().adjustEdit).not.toBeNull();
    expect(previewDocs.length).toBe(previewCalls);
    expect(previews.at(-1)).not.toBeNull();
  });

  it("closing a background tab closes it without switching to it, so the active panel stays open", async () => {
    const { closed, previewDocs } = install([layer("A")], "A");
    useEditor.setState((s) => ({ documents: { ...s.documents, E: document([layer("B")], "B") }, order: [...s.order, "E"] }));
    openPreviewingLevels();
    const previewCalls = previewDocs.length;
    expect(await closeProject("E")).toBe(true);
    expect(closed).toEqual(["E"]);
    expect(useEditor.getState().activeId).toBe("D");
    expect(useEditor.getState().adjustEdit).not.toBeNull();
    expect(previewDocs.length).toBe(previewCalls);
  });
});
/** A Levels adjustment whose white point is `white`: not identity, so it previews. */
function levelsWithWhite(white: number): LayerAdjustment {
  const a = defaultAdjustment("Levels");
  a.levels.ranges[0] = { ...a.levels.ranges[0], white };
  return a;
}
const kinds = (list: (PreviewRequest | null)[]) => list.map((r) => r?.preview ?? null);

describe("a colour adjustment previews quickly while dragging and at full quality once settled", () => {
  beforeEach(() => useEditor.setState({ adjustEdit: null, error: null }));

  it("a tick previews the drag size at once and the settled size after SETTLE_MS", () => {
    const { previews } = install([layer("A")], "A");
    useEditor.getState().beginAdjust({ kind: "Levels" });
    const before = previews.length;
    useEditor.getState().updateAdjust({ adjustment: levelsWithWhite(200) });
    expect(kinds(previews.slice(before))).toEqual(["DragAdjustment"]);
    expect(useEditor.getState().previewSettling()).toBe(true);
    vi.advanceTimersByTime(SETTLE_MS - 1);
    expect(kinds(previews.slice(before))).toEqual(["DragAdjustment"]);
    vi.advanceTimersByTime(1);
    expect(previews.at(-1)).toEqual({ preview: "Adjustment", layer: "A", adjustment: levelsWithWhite(200) });
    expect(useEditor.getState().previewSettling()).toBe(false);
  });

  it("a new tick within the window cancels the pending settled request", () => {
    const { previews } = install([layer("A")], "A");
    useEditor.getState().beginAdjust({ kind: "Levels" });
    const before = previews.length;
    useEditor.getState().updateAdjust({ adjustment: levelsWithWhite(200) });
    vi.advanceTimersByTime(SETTLE_MS - 50);
    useEditor.getState().updateAdjust({ adjustment: levelsWithWhite(180) });
    vi.advanceTimersByTime(SETTLE_MS - 1);   // past the first tick's deadline, short of the second's
    expect(kinds(previews.slice(before))).toEqual(["DragAdjustment", "DragAdjustment"]);
    vi.advanceTimersByTime(1);
    expect(kinds(previews.slice(before))).toEqual(["DragAdjustment", "DragAdjustment", "Adjustment"]);
    expect((previews.at(-1) as any).adjustment).toEqual(levelsWithWhite(180));
  });

  it("the settled request uses the panel's settings when it fires, not when it was scheduled", () => {
    const { previews } = install([layer("A")], "A");
    useEditor.getState().beginAdjust({ kind: "Levels" });
    useEditor.getState().updateAdjust({ adjustment: levelsWithWhite(200) });
    const edit = useEditor.getState().adjustEdit!;
    useEditor.setState({ adjustEdit: { ...edit, adjustment: levelsWithWhite(150) } });
    vi.advanceTimersByTime(SETTLE_MS);
    expect(previews.at(-1)).toEqual({ preview: "Adjustment", layer: "A", adjustment: levelsWithWhite(150) });
  });

  it("OK, Cancel, Preview off and leaving or closing the document all clear the pending request", () => {
    const leave: [string, () => void][] = [
      ["OK", () => useEditor.getState().commitAdjust()],
      ["Cancel", () => useEditor.getState().cancelAdjust()],
      ["Preview off", () => useEditor.getState().setAdjustPreview(false)],
      ["switching documents", () => useEditor.getState().setActive("E")],
      ["closing the document", () => useEditor.getState().closeDocument("D")],
      ["opening another document", () => useEditor.getState().openDocument("E")],
    ];
    for (const [label, action] of leave) {
      const { previews } = install([layer("A")], "A");
      useEditor.setState((s) => ({ documents: { ...s.documents, E: document([layer("B")], "B") }, order: [...s.order, "E"] }));
      useEditor.getState().beginAdjust({ kind: "Levels" });
      useEditor.getState().updateAdjust({ adjustment: levelsWithWhite(200) });
      action();
      expect(useEditor.getState().previewSettling(), label).toBe(false);
      const after = previews.length;
      vi.advanceTimersByTime(SETTLE_MS * 2);
      expect(previews.length, label).toBe(after);   // no settled preview lands afterwards
    }
  });

  it("opening a panel shows the settled quality straight away", () => {
    const { previews } = install([layer("A")], "A");
    useEditor.getState().beginAdjust({ kind: "Grain" });   // Grain's starting settings preview at once
    expect(kinds(previews)).toEqual(["Adjustment"]);
    expect(useEditor.getState().previewSettling()).toBe(false);
  });

  it("a filter has one quality and is not debounced", () => {
    const { previews } = install([layer("A")], "A");
    useEditor.getState().beginAdjust({ kind: "GaussianBlur" });
    useEditor.getState().updateAdjust({ params: { filter: "GaussianBlur", radius: 4 } });
    expect(previews.at(-1)?.preview).toBe("Filter");
    expect(useEditor.getState().previewSettling()).toBe(false);
  });
});