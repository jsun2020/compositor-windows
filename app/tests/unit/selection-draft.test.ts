import { describe, expect, it } from "vitest";
import { dragBox, isSelectionTool, outlineOffset, SelectionDraft, selectionMode } from "../../src/tools/selection-draft";

// Ported from Compositor for Mac 1.2.10's CompositorTests/SelectionTests.swift: the gestures the
// canvas turns into one SelectShape command. What the command then does is engine-tested
// (engine/tests/selection_commands.rs).
const box = (d: SelectionDraft) => {
  const xs = d.points.map((p) => p.x), ys = d.points.map((p) => p.y);
  return { x: Math.min(...xs), y: Math.min(...ys), width: Math.max(...xs) - Math.min(...xs), height: Math.max(...ys) - Math.min(...ys) };
};

describe("selection modes", () => {
  it("Shift adds, Alt subtracts with or without Shift, else the options bar's choice (modifiersPickModeAndSelectionIsClippedToCanvas)", () => {
    expect(selectionMode("Replace", false, false)).toBe("Replace");
    expect(selectionMode("Replace", true, false)).toBe("Add");
    expect(selectionMode("Replace", true, true)).toBe("Subtract");
    expect(selectionMode("Replace", false, true)).toBe("Subtract");
    expect(selectionMode("Add", false, false)).toBe("Add");
    expect(isSelectionTool("marquee") && isSelectionTool("lasso") && isSelectionTool("wand") && !isSelectionTool("move")).toBe(true);
  });
});

describe("the Marquee", () => {
  it("draws whole-pixel rectangles in any direction (marqueeDrawsWholePixelRectanglesInAnyDirection)", () => {
    const d = SelectionDraft.begin("Rectangle", "Replace", { x: 60.4, y: 70.6 });
    d.dragMarquee({ x: 20.2, y: 30.3 }, false);
    expect(box(d)).toEqual({ x: 20, y: 30, width: 40, height: 41 });
    expect(d.finish(true)).toEqual({ type: "SelectShape", kind: "Rectangle", mode: "Replace", antialiased: true,
      points: [[20, 30], [60, 30], [60, 71], [20, 71]] });
  });

  it("squares with Shift and grows from the anchor when centred (marqueeShiftMakesSquaresAndCenteredDragsGrowFromTheAnchor)", () => {
    expect(dragBox({ x: 10, y: 10 }, { x: 40, y: 20 }, true, false)).toEqual({ x: 10, y: 10, width: 30, height: 30 });
    expect(dragBox({ x: 50, y: 50 }, { x: 60, y: 55 }, false, true)).toEqual({ x: 40, y: 45, width: 20, height: 10 });
    expect(dragBox({ x: 50, y: 50 }, { x: 45, y: 58 }, true, true)).toEqual({ x: 42, y: 42, width: 16, height: 16 });
  });

  it("an Alt drag subtracts and never grows from the centre (optionDraggingTheMarqueeSubtractsWithoutDrawingFromTheCenter)", () => {
    const d = SelectionDraft.begin("Rectangle", selectionMode("Replace", false, true), { x: 40, y: 40 });
    d.dragMarquee({ x: 60, y: 60 }, false);
    expect(d.mode).toBe("Subtract");
    expect(box(d)).toEqual({ x: 40, y: 40, width: 20, height: 20 });
  });

  it("a Shift held at the press adds; only a fresh Shift squares (shiftStartsAnAddAndOnlyAFreshShiftSquaresTheMarquee)", () => {
    const held = SelectionDraft.begin("Rectangle", selectionMode("Replace", true, false), { x: 40, y: 40 }, true);
    held.dragMarquee({ x: 70, y: 50 }, true);
    expect(held.mode).toBe("Add");
    expect(box(held)).toEqual({ x: 40, y: 40, width: 30, height: 10 });
    const fresh = SelectionDraft.begin("Rectangle", "Add", { x: 20, y: 60 }, true);
    fresh.dragMarquee({ x: 30, y: 65 }, true);
    fresh.dragMarquee({ x: 35, y: 68 }, false);
    fresh.dragMarquee({ x: 40, y: 70 }, true);
    expect(box(fresh)).toEqual({ x: 20, y: 60, width: 20, height: 20 });
  });

  it("an Ellipse sends its box's corners for the engine to fill with the oval", () => {
    const d = SelectionDraft.begin("Ellipse", "Replace", { x: 10, y: 20 });
    d.dragMarquee({ x: 70, y: 60 }, false);
    expect(d.finish(false)).toEqual({ type: "SelectShape", kind: "Ellipse", mode: "Replace", antialiased: false,
      points: [[10, 20], [70, 20], [70, 60], [10, 60]] });
  });
});

describe("the Lasso", () => {
  it("skips freehand points closer than a quarter pixel", () => {
    const d = SelectionDraft.begin("Freehand", "Replace", { x: 10, y: 10 });
    d.extend({ x: 10.1, y: 10.1 });
    d.extend({ x: 10.3, y: 10 });
    expect(d.points).toEqual([{ x: 10, y: 10 }, { x: 10.3, y: 10 }]);
  });

  it("drops a misplaced polygonal corner and closes on the first corner (polygonalCornersCanBeRemovedAndClosed)", () => {
    const d = SelectionDraft.begin("Polygonal", "Replace", { x: 10, y: 10 });
    const view = (p: { x: number; y: number }) => ({ x: p.x * 2, y: p.y * 2 });
    const click = (x: number, y: number, count = 1) => d.click({ x, y }, view({ x, y }), view(d.points[0]), count);
    expect(click(90, 10)).toBe("extend");
    expect(click(50, 50)).toBe("extend");
    expect(d.removeLast()).toBe(true);
    click(90, 90); click(10, 90);
    expect(d.points.length).toBe(4);
    expect(click(13, 13), "4.2 document px from the first corner is 8.5 view px at 2x: another corner").toBe("extend");
    d.removeLast();
    expect(click(12, 12), "5.7 view px: close").toBe("close");
    expect(d.finish(true)).toEqual({ type: "SelectShape", kind: "Polygonal", mode: "Replace", antialiased: true,
      points: [[10, 10], [90, 10], [90, 90], [10, 90]] });
  });

  it("a double-click closes, and Backspace on the only corner drops the draft", () => {
    const d = SelectionDraft.begin("Polygonal", "Add", { x: 0, y: 0 });
    expect(d.click({ x: 50, y: 0 }, { x: 50, y: 0 }, { x: 0, y: 0 }, 2)).toBe("close");
    const single = SelectionDraft.begin("Polygonal", "Replace", { x: 1, y: 1 });
    expect(single.removeLast()).toBe(false);
  });

  it("a click with no drag still sends its one point: the engine deselects (clickDeselectsAndSelectionStepsUndo)", () => {
    const d = SelectionDraft.begin("Freehand", "Replace", { x: 5, y: 5 });
    expect(d.finish(true)).toEqual({ type: "SelectShape", kind: "Freehand", mode: "Replace", antialiased: true, points: [[5, 5]] });
  });
});

describe("moving the outline", () => {
  it("moves in whole pixels, and Shift keeps to one axis (draggingMovesTheOutlineInWholePixelsAsOneUndo)", () => {
    expect(outlineOffset({ x: 20, y: 20 }, { x: 30.4, y: 49.6 }, false)).toEqual({ dx: 10, dy: 30 });
    expect(outlineOffset({ x: 20, y: 20 }, { x: 60.2, y: 60.4 }, false)).toEqual({ dx: 40, dy: 40 });
    expect(outlineOffset({ x: 20, y: 20 }, { x: 35, y: 26 }, true)).toEqual({ dx: 15, dy: 0 });
    expect(outlineOffset({ x: 20, y: 20 }, { x: 26, y: 5 }, true)).toEqual({ dx: 0, dy: -15 });
  });
});
