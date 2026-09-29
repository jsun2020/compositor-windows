import { beforeEach, describe, expect, it } from "vitest";
import { DEFAULT_PALETTE, JOB_PIXELS, useEditor } from "../../src/state/store";
import { canClearSelected, canPaint, deleteKeyPressed, fillActive } from "../../src/actions/layers";
import type { Command, DocumentState, LayerState, SelectionState } from "../../src/engine/types";
import type { EngineClient } from "../../src/engine/client";
import type { JobClient } from "../../src/engine/jobs";

// Fill (SelectionEdits.swift:40-58) and the rule that gates it (`canPaint`, EditorSession+Brush.swift:5-11).
function layer(patch: Partial<LayerState> = {}): LayerState {
  return { id: "A", name: "A", visible: true, isGroup: false, parentId: null, opacity: 1, blendMode: "Normal",
    transform: { origin: [0, 0], size: [40, 30], rotation: 0, flipX: false, flipY: false, sampling: "High quality" },
    pixelsWidth: 40, pixelsHeight: 30, pixelsRevision: 1, hasPixels: true, hasMask: true, maskWidth: 40, maskHeight: 30,
    maskRevision: 1, maskEnabled: true, maskLinked: true, maskSourceId: null, maskPlacement: null, maskBackground: 255, ...patch };
}
const SELECTION: SelectionState = { revision: 3, empty: false, bounds: { x: 1, y: 2, width: 5, height: 6 }, antialiased: true, feather: 0, points: 4 };
function doc(l: LayerState, selection: SelectionState | null = null): DocumentState {
  return { id: "D", documentId: "D", width: 40, height: 30, resolution: 72, activeLayerId: l.id, canUndo: false, canRedo: false, isModified: false,
    undoDepth: 0, undoEntryId: null, path: null, guides: [], undrawn: [], selection, layers: [l] };
}
let log: string[] = [];
function install(l: LayerState = layer(), selection: SelectionState | null = null) {
  log = [];
  const state = doc(l, selection);
  const engine = {
    state: () => state,
    execute: (_id: string, c: Command) => { log.push(JSON.stringify(c)); return { structure: true, canvas: false, layers: [] }; },
    storedPixels: () => useEditor.getState().documents.D.layers[0].pixelsWidth * useEditor.getState().documents.D.layers[0].pixelsHeight,
    jobInput: () => { log.push("job input"); return { input: "{}", pixels: new ArrayBuffer(4), mask: null }; },
    setPreview: () => ({ structure: true, canvas: false, layers: [] }),
    editPixels: () => useEditor.getState().documents.D.layers[0].pixelsWidth * useEditor.getState().documents.D.layers[0].pixelsHeight,
  } as unknown as EngineClient;
  const jobs = { run: () => new Promise(() => {}) } as unknown as JobClient;
  useEditor.setState({ engine, jobs, jobPixels: JOB_PIXELS, activeId: "D", documents: { D: state }, order: ["D"], selectedLayerIds: [l.id], maskSelected: false,
    working: false, palette: { ...DEFAULT_PALETTE, foreground: { red: 1, green: 0.5, blue: 0 }, background: { red: 0, green: 0, blue: 1 } },
    adjustEdit: null, transformEdit: null, error: null, tool: "move", cropRect: null, sheet: null, colorPicker: null });
}
const s = () => useEditor.getState();

describe("Fill", () => {
  beforeEach(() => install());
  it("fills the pixels with the foreground or background colour, and a targeted mask with its black or white", () => {
    fillActive(false);
    fillActive(true);
    useEditor.setState({ maskSelected: true });
    fillActive(false);
    fillActive(true);
    expect(log.map((c) => JSON.parse(c))).toEqual([
      { type: "Fill", id: "A", mask: false, color: [1, 0.5, 0] },
      { type: "Fill", id: "A", mask: false, color: [0, 0, 1] },
      { type: "Fill", id: "A", mask: true, color: [0, 0, 0] },
      { type: "Fill", id: "A", mask: true, color: [1, 1, 1] },
    ]);
  });
  it("sends a large layer to the job worker", () => {
    useEditor.setState({ jobPixels: 40 * 30 - 1 });
    fillActive(false);
    expect(log).toEqual(["job input"]);
    expect(s().working).toBe(true);
  });
  it("is refused wherever the Mac's canPaint is false", () => {
    const refusals: [string, () => void][] = [
      ["a folder's pixels", () => install(layer({ isGroup: true, hasPixels: false }))],
      ["an adjustment layer", () => install(layer({ adjustment: { kind: "Invert" } as never }))],
      ["a hidden layer", () => install(layer({ visible: false }))],
      ["a disabled mask as the target", () => { install(layer({ maskEnabled: false })); useEditor.setState({ maskSelected: true }); }],
      ["an empty selection", () => install(layer(), { ...SELECTION, empty: true })],
      ["two layers selected", () => { install(); useEditor.setState({ selectedLayerIds: ["A", "B"] }); }],
      ["an open panel", () => { install(); useEditor.setState({ adjustEdit: { kind: "Levels" } as never }); }],
      ["a job's result to come", () => { install(); useEditor.setState({ working: true }); }],
      ["a crop rectangle pending", () => { install(); useEditor.setState({ tool: "crop", cropRect: { x: 0, y: 0, width: 5, height: 5 } }); }],
    ];
    for (const [why, arrange] of refusals) {
      arrange();
      expect(canPaint(), why).toBe(false);
      fillActive(false);
      expect(log, why).toEqual([]);
    }
    // Their opposites paint: a folder's enabled mask as the target, and a selection with something in it.
    install(layer({ isGroup: true, hasPixels: false }));
    useEditor.setState({ maskSelected: true });
    expect(canPaint()).toBe(true);
    install(layer(), SELECTION);
    expect(canPaint()).toBe(true);
  });
  it("makes Delete on a targeted mask fill the selection with the mask's background colour", () => {
    install(layer(), SELECTION);
    useEditor.setState({ maskSelected: true, palette: { ...DEFAULT_PALETTE, maskPaintWhite: true } });
    deleteKeyPressed();
    expect(log.map((c) => JSON.parse(c))).toEqual([{ type: "Fill", id: "A", mask: true, color: [0, 0, 0] }]);
    install(layer(), SELECTION);
    deleteKeyPressed();
    expect(log.map((c) => JSON.parse(c))).toEqual([{ type: "ClearSelectedPixels", id: "A", mask: false }]);
  });
  it("decides the job worker by the pixels the fill paints, not the size the layer stores (ruling C1)", () => {
    // A blank layer stores nothing but paints its 6000 x 4000 canvas.
    install(layer({ hasPixels: false, pixelsWidth: 0, pixelsHeight: 0 }));
    useEditor.setState({ engine: { ...s().engine!, editPixels: () => 6000 * 4000 } as never });
    fillActive(false);
    expect(log).toEqual(["job input"]);
  });
  it("does not clear through the selection while a job's result is to come (final review minor 10)", () => {
    install(layer(), SELECTION);
    expect(canClearSelected()).toBe(true);
    useEditor.setState({ working: true });
    expect(canClearSelected()).toBe(false);
    deleteKeyPressed();
    expect(log).toEqual([]);
  });
  it("refuses a fill too large to paint before anything is sent", () => {
    useEditor.setState({ engine: { ...s().engine!, editPixels: () => { throw new Error("too large"); } } as never });
    fillActive(false);
    expect([log, s().error]).toEqual([[], "too large"]);
  });
});
