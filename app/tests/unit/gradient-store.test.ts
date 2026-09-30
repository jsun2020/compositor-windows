import { beforeEach, describe, expect, it } from "vitest";
import { BUSY_MESSAGE, DEFAULT_PALETTE, JOB_PIXELS, useEditor } from "../../src/state/store";
import { DEFAULT_GRADIENT, gradientStops, snapped45 } from "../../src/state/gradient-edit";
import { runAction, typeOpacityDigit } from "../../src/shortcuts/useShortcuts";
import { fillActive } from "../../src/actions/layers";
import { importImages } from "../../src/actions/files";
import type { Command, DocumentState, LayerState, PreviewRequest } from "../../src/engine/types";
import type { EngineClient } from "../../src/engine/client";
import type { JobClient, JobResult } from "../../src/engine/jobs";
import type { ShellBridge } from "../../src/shell/bridge";

// The Gradient tool's pending edit (Gradient.swift; EditorSession.swift:573-590; GradientTests' rules).
const RED = { red: 1, green: 0, blue: 0 }, BLUE = { red: 0, green: 0, blue: 1 };

describe("the gradient's stops and Shift's steps", () => {
  it("runs from the foreground to the background or to itself transparent, reversed on request", () => {
    expect(gradientStops(DEFAULT_GRADIENT, RED, BLUE)).toEqual([[1, 0, 0, 1], [1, 0, 0, 0]]);
    expect(gradientStops({ ...DEFAULT_GRADIENT, style: "Foreground to Background" }, RED, BLUE)).toEqual([[1, 0, 0, 1], [0, 0, 1, 1]]);
    expect(gradientStops({ ...DEFAULT_GRADIENT, style: "Foreground to Background", reversed: true }, RED, BLUE)).toEqual([[0, 0, 1, 1], [1, 0, 0, 1]]);
  });
  it("holds the moved end to the nearest eighth of a turn at the same distance", () => {
    // (10, 3) from (0, 0) is 16.7 degrees: flat, 10.44 long.
    const flat = snapped45({ x: 10, y: 3 }, { x: 0, y: 0 });
    expect(flat.x).toBeCloseTo(Math.hypot(10, 3), 12);
    expect(flat.y).toBeCloseTo(0, 12);
    // (-7, -9) from (1, 1) is -128 degrees from +x: -135, so down-left at 45 degrees.
    const diagonal = snapped45({ x: -7, y: -9 }, { x: 1, y: 1 });
    const length = Math.hypot(8, 10);
    expect(diagonal.x).toBeCloseTo(1 - length / Math.SQRT2, 12);
    expect(diagonal.y).toBeCloseTo(1 - length / Math.SQRT2, 12);
  });
});

function layer(patch: Partial<LayerState> = {}): LayerState {
  return { id: "A", name: "A", visible: true, isGroup: false, parentId: null, opacity: 1, blendMode: "Normal",
    transform: { origin: [0, 0], size: [40, 30], rotation: 0, flipX: false, flipY: false, sampling: "High quality" },
    pixelsWidth: 40, pixelsHeight: 30, pixelsRevision: 1, hasPixels: true, hasMask: true, maskWidth: 40, maskHeight: 30,
    maskRevision: 1, maskEnabled: true, maskLinked: true, maskSourceId: null, maskPlacement: null, maskBackground: 255, ...patch };
}
function doc(l: LayerState): DocumentState {
  return { id: "D", documentId: "D", width: 40, height: 30, resolution: 72, activeLayerId: l.id, canUndo: true, canRedo: false, isModified: false,
    undoDepth: 3, undoEntryId: 3, path: null, guides: [], undrawn: [], selection: null, layers: [l, { ...l, id: "B", name: "B" }] };
}
let log: string[] = [];
let previews: (PreviewRequest | null)[] = [];
function install(l: LayerState = layer()) {
  log = []; previews = [];
  const state = doc(l);
  const engine = {
    state: () => state,
    execute: (_id: string, c: Command) => { log.push(`execute ${JSON.stringify(c)}`); return { structure: true, canvas: false, layers: [] }; },
    setPreview: (_id: string, r: PreviewRequest | null) => { previews.push(r); return { structure: true, canvas: false, layers: [] }; },
    undo: () => { log.push("undo"); return { structure: true, canvas: false, layers: [] }; },
    redo: () => { log.push("redo"); return { structure: true, canvas: false, layers: [] }; },
    storedPixels: () => useEditor.getState().documents.D.layers[0].pixelsWidth * useEditor.getState().documents.D.layers[0].pixelsHeight,
    editPixels: () => useEditor.getState().documents.D.layers[0].pixelsWidth * useEditor.getState().documents.D.layers[0].pixelsHeight,
    jobInput: () => { log.push("job input"); return { input: "{}", pixels: new ArrayBuffer(4), mask: null }; },
  } as unknown as EngineClient;
  const jobs = { run: () => new Promise(() => {}) } as unknown as JobClient;
  useEditor.setState({ engine, jobs, jobPixels: JOB_PIXELS, activeId: "D", documents: { D: state }, order: ["D"], selectedLayerIds: [l.id], maskSelected: false,
    working: false, palette: { ...DEFAULT_PALETTE, foreground: RED, background: BLUE }, gradientOptions: DEFAULT_GRADIENT, gradientEdit: null,
    adjustEdit: null, transformEdit: null, error: null, tool: "gradient", cropRect: null, sheet: null, colorPicker: null });
}
const s = () => useEditor.getState();
const draw = (from: [number, number], to: [number, number]) => {
  s().beginGradient({ x: from[0], y: from[1] });
  s().moveGradient({ end: { x: to[0], y: to[1] } }, true);
  s().endGradientDrag();
};
const last = () => previews.at(-1) as Extract<PreviewRequest, { preview: "Gradient" }>;

describe("a layer click while a job's result is to come (final review minor 1)", () => {
  beforeEach(() => install());
  it("waits, as a command does: nothing executed (the engine would clear the preview the canvas keeps), the selection kept", () => {
    useEditor.setState({ working: true });
    s().selectLayers(["B"], "B");
    expect([log, s().selectedLayerIds, s().error]).toEqual([[], ["A"], BUSY_MESSAGE]);
    useEditor.setState({ working: false, error: null });
    s().selectLayers(["B"], "B");
    expect(log).toEqual([`execute ${JSON.stringify({ type: "SetActiveLayer", id: "B" })}`]);
  });
  it("a mask chip clicked meanwhile targets nothing either: the layer's click is refused and so is the target (re-review residual)", () => {
    useEditor.setState({ working: true });
    // What LayersList's mask chip does: choose the layer, then target its mask.
    s().selectLayers(["A"], "A");
    s().setMaskSelected(true);
    expect([s().maskSelected, s().maskTargeted(), s().error]).toEqual([false, false, BUSY_MESSAGE]);
  });
  it("a click that applies a pending gradient through the worker waits too: no SetActiveLayer to clear the preview the job keeps (re-review residual)", () => {
    draw([5, 6], [30, 6]);
    const sent: string[] = [];
    const jobs = { run: (_channel: string, msg: { command?: string }) => { if (msg.command) sent.push(JSON.parse(msg.command).type as string); return new Promise(() => {}); } } as unknown as JobClient;
    useEditor.setState({ jobPixels: 1, jobs });
    const shown = previews.length;
    s().selectLayers(["B"], "B");
    expect(sent, "the gradient went to the worker").toEqual(["Gradient"]);
    expect(log.filter((l) => l.includes("SetActiveLayer"))).toEqual([]);
    expect([s().selectedLayerIds, s().error, s().working]).toEqual([["A"], BUSY_MESSAGE, true]);
    expect(previews.length, "nothing cleared the gradient's preview").toBe(shown);
  });
});

describe("a pending gradient", () => {
  beforeEach(() => install());
  it("previews from a reduced copy while dragged and at full quality once let go, and records nothing", () => {
    s().beginGradient({ x: 5, y: 6 });
    s().moveGradient({ end: { x: 30, y: 6 } }, true);
    expect(last()).toEqual({ preview: "Gradient", layer: "A", mask: false, dragging: true,
      gradient: { shape: "Linear", start: [5, 6], end: [30, 6], from: [1, 0, 0, 1], to: [1, 0, 0, 0], opacity: 1 } });
    s().endGradientDrag();
    expect(last().dragging).toBe(false);
    expect(log).toEqual([]);
  });
  it("leaves nothing pending after a click without a line", () => {
    draw([5, 6], [5.3, 6.3]);
    expect(s().gradientEdit).toBeNull();
    expect(previews.at(-1)).toBeNull();
  });
  it("is applied by Return as one Gradient, and dropped by Escape", () => {
    draw([5, 6], [30, 6]);
    runAction("apply");
    expect(log).toEqual([`execute ${JSON.stringify({ type: "Gradient", id: "A", mask: false, gradient: last().gradient })}`]);
    expect(s().gradientEdit).toBeNull();
    draw([1, 1], [9, 9]);
    runAction("cancel");
    expect(s().gradientEdit).toBeNull();
    expect(log.length).toBe(1);
    expect(previews.at(-1)).toBeNull();
  });
  it("is discarded by the first Undo, and the next Undo undoes", () => {
    draw([5, 6], [30, 6]);
    s().undo();
    expect([s().gradientEdit, log]).toEqual([null, []]);
    s().undo();
    expect(log).toEqual(["undo"]);
  });
  it("a new line on the same target replaces the pending one, recording nothing (fix round 1, M-4)", () => {
    draw([5, 6], [30, 6]);
    draw([1, 1], [20, 20]);
    expect(log).toEqual([]);
    expect(last().gradient.start).toEqual([1, 1]);
  });
  it("is applied before a change of tool, of layer, of target, or any other command", () => {
    const paint = () => log.filter((l) => l.includes('"Gradient"')).length;
    draw([5, 6], [30, 6]); s().setTool("move"); expect(paint()).toBe(1);
    s().setTool("gradient");
    draw([5, 6], [30, 6]); s().selectLayers(["B"], "B"); expect(paint()).toBe(2);
    install();
    draw([5, 6], [30, 6]); s().setMaskSelected(true); expect(paint()).toBe(1);
    install();
    draw([5, 6], [30, 6]); s().run({ type: "AddBlankLayer" });
    expect(log.map((l) => JSON.parse(l.slice("execute ".length)).type)).toEqual(["Gradient", "AddBlankLayer"]);
  });
  it("is drawn again when its settings or the palette change, in the new colours", () => {
    draw([5, 6], [30, 6]);
    s().setGradientOptions({ shape: "Radial", style: "Foreground to Background", opacity: 0.4 });
    expect(last().gradient).toMatchObject({ shape: "Radial", from: [1, 0, 0, 1], to: [0, 0, 1, 1], opacity: 0.4 });
    expect(last().dragging).toBe(false);
    s().swapPalette();
    expect(last().gradient).toMatchObject({ from: [0, 0, 1, 1], to: [1, 0, 0, 1] });
  });
  it("paints a targeted mask in black and white", () => {
    useEditor.setState({ maskSelected: true });
    draw([5, 6], [30, 6]);
    expect(last()).toMatchObject({ mask: true, gradient: { from: [0, 0, 0, 1], to: [0, 0, 0, 0] } });
  });
  it("does not start where nothing can be painted, and goes to the job worker on a large layer", () => {
    install(layer({ visible: false }));
    expect(s().beginGradient({ x: 1, y: 1 })).toBe(false);
    install();
    useEditor.setState({ jobPixels: 40 * 30 - 1 });
    draw([5, 6], [30, 6]);
    s().commitGradient();
    expect(log).toEqual(["job input"]);
  });
  it("chooses the job worker by the stored layer's size, not the reduced preview the state shows", () => {
    // The state reports a previewed layer at its preview's size: 10 x 7.5 of the stored 40 x 30.
    const shown = { ...s().documents.D, layers: s().documents.D.layers.map((l) => ({ ...l, pixelsWidth: 10, pixelsHeight: 8 })) };
    useEditor.setState({ jobPixels: 100, engine: { ...s().engine!, state: () => shown, storedPixels: () => 40 * 30 } as never });
    draw([5, 6], [30, 6]);
    expect(s().usesJob("A")).toBe(true);
  });
  it("sends a gradient to the job worker by the pixels it paints: a small layer's mask grown to a large canvas counts grown", () => {
    // 40 x 30 pixels, but on its mask the gradient paints the mask grown to a 6000 x 4000 canvas (Task 14a).
    useEditor.setState({ jobPixels: 40 * 30, engine: { ...s().engine!, editPixels: (_d: string, _l: string, mask: boolean) => (mask ? 6000 * 4000 : 40 * 30) } as never });
    draw([5, 6], [30, 6]);
    s().commitGradient();
    expect(log).toEqual([`execute ${JSON.stringify({ type: "Gradient", id: "A", mask: false, gradient: last().gradient })}`]);
    log.length = 0;
    useEditor.setState({ maskSelected: true });
    draw([5, 6], [30, 6]);
    s().commitGradient();
    expect(log).toEqual(["job input"]);
  });
  it("refuses a gradient too large to paint before anything is sent", () => {
    useEditor.setState({ maskSelected: true, engine: { ...s().engine!, editPixels: () => { throw new Error("too large"); } } as never });
    draw([5, 6], [30, 6]);
    s().commitGradient();
    expect([log, s().gradientEdit, s().error, previews.at(-1)]).toEqual([[], null, "too large", null]);
  });
  it("clears the pending preview when the job's own input throws (fix round 3, M-3)", () => {
    useEditor.setState({ jobPixels: 1, engine: { ...s().engine!, jobInput: () => { throw new Error("boom"); } } as never });
    draw([5, 6], [30, 6]);
    expect(last()).not.toBeNull(); // the pending gradient's preview shows before the commit is attempted
    s().commitGradient();
    expect(previews.at(-1)).toBeNull();
  });
  it("applies a pending gradient before a fill, refusing the fill while the gradient's own job runs (fix round 1, I-1)", () => {
    draw([5, 6], [30, 6]);
    const jobsSent: string[] = [];
    const jobs = { run: (_channel: string, msg: { command?: string }) => { if (msg.command) jobsSent.push(JSON.parse(msg.command).type as string); return new Promise(() => {}); } } as unknown as JobClient;
    useEditor.setState({ jobPixels: 1, jobs });
    fillActive(false);
    expect(jobsSent).toEqual(["Gradient"]);
    expect(s().gradientEdit).toBeNull();
  });
  it("applies a pending gradient through the worker, then refuses the command while that job runs (final review I-1)", async () => {
    draw([5, 6], [30, 6]);
    const sent: string[] = []; const installed: string[] = [];
    let finish: (r: JobResult | null) => void = () => {};
    const jobs = { run: (_channel: string, msg: { command?: string }) => { if (msg.command) sent.push(JSON.parse(msg.command).type as string); return new Promise<JobResult | null>((r) => { finish = r; }); } } as unknown as JobClient;
    const engine = { ...s().engine!, installJob: (_d: string, layerId: string) => { installed.push(layerId); return { structure: true, canvas: false, layers: [] }; } } as unknown as EngineClient;
    useEditor.setState({ jobPixels: 1, jobs, engine });
    expect(s().run({ type: "Deselect" })).toBe(false);
    expect(s().error).toBe(BUSY_MESSAGE);
    expect(log, "the gradient's job input, and no Deselect").toEqual(["job input"]);
    expect(sent).toEqual(["Gradient"]);
    finish({ header: "OUT", pixels: null, mask: null });
    await new Promise((r) => setTimeout(r, 0));
    expect([installed, s().working]).toEqual([["A"], false]);
    expect(s().run({ type: "Deselect" })).toBe(true);
    expect(log.at(-1)).toBe(`execute ${JSON.stringify({ type: "Deselect" })}`);
  });
  it("applies a pending gradient through the worker, then refuses an import while that job runs (final review I-1)", async () => {
    draw([5, 6], [30, 6]);
    const imported: string[] = [];
    const jobs = { run: () => new Promise(() => {}) } as unknown as JobClient;
    const engine = { ...s().engine!, importImage: (doc: string | null) => { imported.push(String(doc)); return "D"; } } as unknown as EngineClient;
    const bridge = { readFile: async () => new Uint8Array([1]), baseName: (p: string) => p, pickImportImages: async () => ["C:/b.png"] } as unknown as ShellBridge;
    useEditor.setState({ jobPixels: 1, jobs, engine, bridge, busy: false });
    await importImages(["C:/b.png"]);
    expect([imported, s().error, s().working]).toEqual([[], BUSY_MESSAGE, true]);
    expect(log).toEqual(["job input"]);
  });
  it("applies a pending gradient before opening an adjustment panel (fix round 1, I-1)", () => {
    draw([5, 6], [30, 6]); // small: commits at once, on the UI thread, not through the worker
    const engine = { ...s().engine!, histogram: () => [[0, 0], [0, 0], [0, 0], [0, 0]], adjustmentIsIdentity: () => false } as unknown as EngineClient;
    useEditor.setState({ engine });
    expect(s().beginAdjust({ kind: "Levels" })).toBe(true);
    expect(log.some((l) => l.includes('"Gradient"'))).toBe(true);
    expect(s().gradientEdit).toBeNull();
    expect(s().adjustEdit).not.toBeNull();
  });
  it("takes the digit keys as its opacity, at least 1 %, and Tab as its shape", () => {
    typeOpacityDigit(4, 1000, (v) => s().setGradientOptions({ opacity: Math.max(0.01, v) }));
    expect(s().gradientOptions.opacity).toBe(0.4);
    runAction("opacity-0");
    runAction("opacity-0");
    expect(s().gradientOptions.opacity).toBe(0.01);
    runAction("cycle-tool-mode");
    expect(s().gradientOptions.shape).toBe("Radial");
  });
});
