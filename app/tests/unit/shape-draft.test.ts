import { beforeEach, describe, expect, it } from "vitest";
import { beginShape, dragShape, DEFAULT_SHAPE, nextShapeKind, shapeSpec } from "../../src/tools/shape-draft";
import { DEFAULT_PALETTE, useEditor } from "../../src/state/store";
import { runAction } from "../../src/shortcuts/useShortcuts";
import type { Command, DocumentState } from "../../src/engine/types";
import type { EngineClient } from "../../src/engine/client";

// The Shape tool's drag, ported from the Mac's ShapeToolTests with their numbers.
const drag = (from: [number, number], to: [number, number], options = DEFAULT_SHAPE, square = false, fromCenter = false) =>
  dragShape(beginShape({ x: from[0], y: from[1] }, options), { x: to[0], y: to[1] }, square, fromCenter);

describe("the Shape tool's drag", () => {
  it("draws a box between whole pixels (aClickEscapeOrToolSwitchMakesNoLayer)", () => {
    expect(drag([20, 20], [50, 50]).rect).toEqual({ x: 20, y: 20, width: 30, height: 30 });
    // A fractional press and pointer land on the nearest corners.
    expect(drag([10.4, 9.6], [39.5, 29.2]).rect).toEqual({ x: 10, y: 10, width: 30, height: 19 });
  });
  it("squares with Shift and grows from the centre with Alt (ellipseLeavesItsCornersClear...)", () => {
    const d = drag([50, 40], [60, 45], { ...DEFAULT_SHAPE, kind: "Ellipse" }, true, true);
    expect(shapeSpec(d, 4)).toEqual({ kind: "Ellipse", rect: { x: 40, y: 30, width: 20, height: 20 } });
  });
  it("takes the corner radius for rectangles only, fixed when the drag begins (roundedRectangles...)", () => {
    expect(beginShape({ x: 5, y: 5 }, { ...DEFAULT_SHAPE, kind: "Ellipse", cornerRadius: 8 }).cornerRadius).toBe(0);
    const d = drag([10, 10], [50, 40], { ...DEFAULT_SHAPE, cornerRadius: 8 });
    expect(shapeSpec(d, 4)).toEqual({ kind: "Rectangle", rect: { x: 10, y: 10, width: 40, height: 30 }, cornerRadius: 8 });
  });
  it("makes nothing from a click, or from a box under a pixel on a side", () => {
    expect(shapeSpec(beginShape({ x: 20, y: 20 }, DEFAULT_SHAPE), 4)).toBeNull();
    expect(shapeSpec(drag([20, 20], [50, 20.4]), 4)).toBeNull();
    // A line's box is its ends grown by its width: a click with a 4 px line is a 4 x 4 dot, as on the Mac.
    const dot = beginShape({ x: 20, y: 20 }, { ...DEFAULT_SHAPE, kind: "Line" });
    expect(shapeSpec(dot, 4)).toEqual({ kind: "Line", start: [20, 20], end: [20, 20], width: 4 });
    expect(shapeSpec(dot, 0.5)).toBeNull();
  });
  it("keeps a line's end where the pointer is, and Shift holds it to 45 degrees about the start", () => {
    const line = { ...DEFAULT_SHAPE, kind: "Line" as const };
    expect(shapeSpec(drag([20.6, 120.4], [140.3, 121.2], line), 1)).toEqual({ kind: "Line", start: [21, 120], end: [140.3, 121.2], width: 1 });
    const d = drag([10, 10], [40, 12], line, true);
    expect(d.end!.y).toBeCloseTo(10, 12);
    expect(d.end!.x).toBeCloseTo(10 + Math.hypot(30, 2), 12);
  });
  it("steps Rectangle, Ellipse, Line and round again", () => {
    expect([nextShapeKind("Rectangle"), nextShapeKind("Ellipse"), nextShapeKind("Line")]).toEqual(["Ellipse", "Line", "Rectangle"]);
  });
});

describe("finishing a shape", () => {
  let log: Command[] = [];
  beforeEach(() => {
    log = [];
    const state = { id: "D", documentId: "D", width: 100, height: 80, resolution: 72, activeLayerId: null, canUndo: false, canRedo: false, isModified: false,
      undoDepth: 0, undoEntryId: null, path: null, guides: [], undrawn: [], selection: null, layers: [] } as DocumentState;
    const engine = { state: () => state, execute: (_id: string, c: Command) => { log.push(c); return { structure: true, canvas: false, layers: [] }; } } as unknown as EngineClient;
    useEditor.setState({ engine, activeId: "D", documents: { D: state }, order: ["D"], selectedLayerIds: [], maskSelected: true, working: false,
      palette: { ...DEFAULT_PALETTE, foreground: { red: 1, green: 0, blue: 0 } }, shapeOptions: DEFAULT_SHAPE, shapeDraft: null, tool: "shape",
      adjustEdit: null, transformEdit: null, gradientEdit: null, error: null, collapsed: {} });
  });
  it("sends one AddShape in the image's foreground colour, even with a mask targeted, and drops the draft", () => {
    const s = useEditor.getState();
    s.setShapeDraft(drag([10, 10], [40, 30]));
    s.finishShape();
    expect(log).toEqual([{ type: "AddShape", shape: { kind: "Rectangle", rect: { x: 10, y: 10, width: 30, height: 20 }, cornerRadius: 0 }, color: [1, 0, 0] }]);
    expect(useEditor.getState().shapeDraft).toBeNull();
  });
  it("is dropped by Escape, a tool change or a new kind, sending nothing; Shift+U steps the kind", () => {
    const s = useEditor.getState();
    s.setShapeDraft(drag([10, 10], [40, 30])); runAction("cancel");
    expect(useEditor.getState().shapeDraft).toBeNull();
    s.setShapeDraft(drag([10, 10], [40, 30])); s.setShapeOptions({ kind: "Line" });
    expect(useEditor.getState().shapeDraft).toBeNull();
    s.setShapeDraft(drag([10, 10], [40, 30])); s.setTool("move");
    expect(useEditor.getState().shapeDraft).toBeNull();
    expect(log).toEqual([]);
    runAction("shape-next");
    expect(useEditor.getState().tool).toBe("shape");
    runAction("shape-next");
    expect(useEditor.getState().shapeOptions.kind).toBe("Rectangle");
  });
});
