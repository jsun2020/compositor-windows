import { describe, expect, it } from "vitest";
import { commitTarget, dragged, makeReorderState, nearestSlot, renderX, reorderedTabs, type TabSlot } from "../../src/state/tab-reorder";

// Compositor 1.4.5's tab reordering (ProjectTabs.swift and ProjectWorkspace.swift at v1.4.5).
describe("reorderedTabs (ProjectWorkspace.moveTab)", () => {
  it("reorders, clamps and ignores a tab already in place or unknown", () => {
    // moveTabReordersWithoutTouchingSelectionOrDocuments (ProjectWorkspaceTests.swift:126-145), step for step.
    let order = ["a", "b", "c"];
    order = reorderedTabs(order, "c", 0);
    expect(order).toEqual(["c", "a", "b"]);
    order = reorderedTabs(order, "a", 2);
    expect(order).toEqual(["c", "b", "a"]);
    order = reorderedTabs(order, "c", 99);
    expect(order).toEqual(["b", "a", "c"]);
    expect(reorderedTabs(order, "c", 2)).toBe(order);
    expect(reorderedTabs(order, "zz", 0)).toBe(order);
    expect(reorderedTabs(order, "b", -5)).toBe(order);
  });
});

// Three tabs 100, 60 and 80 wide with 4 between them, from x 10.
const W = { a: 100, b: 60, c: 80 }, GAP = 4, X0 = 10;
const layout: TabSlot[] = [
  { id: "a", x: X0, width: W.a },
  { id: "b", x: X0 + W.a + GAP, width: W.b },
  { id: "c", x: X0 + W.a + GAP + W.b + GAP, width: W.c },
];

describe("a tab drag (TabReorderState)", () => {
  it("packs the other tabs as if the dragged one were gone", () => {
    const s = makeReorderState("a", layout, GAP);
    expect(s.others).toEqual(["b", "c"]);
    expect(s.compactedX).toEqual({ b: X0, c: X0 + W.b + GAP });
    expect([s.startX, s.originX]).toEqual([X0, X0]);
  });

  it("drops at the slot nearest the dragged tab's left edge, the end included", () => {
    const s = makeReorderState("a", layout, GAP);
    const slots = [X0, X0 + W.b + GAP, X0 + W.b + GAP + W.c + GAP];
    for (const [translation, expected] of [[0, 0], [slots[1] - X0 - 1, 1], [slots[2] - X0 + 500, 2]] as const) {
      expect(nearestSlot({ ...s, translation }), `translation ${translation}`).toBe(expected);
    }
    // Halfway between two slots the first wins, as Swift's min(by:) keeps the first.
    expect(nearestSlot({ ...s, translation: (slots[0] + slots[1]) / 2 - X0 })).toBe(0);
    // Dragging c to the left end.
    expect(dragged(makeReorderState("c", layout, GAP), -1000).targetIndex).toBe(0);
  });

  it("commits to the neighbour's place, past the last one, or where it was", () => {
    const order = ["a", "b", "c"];
    // a let go at the end: after c, less the one it leaves (commitReorder, :196-204).
    const aToEnd = dragged(makeReorderState("a", layout, GAP), 1000);
    expect(reorderedTabs(order, "a", commitTarget(order, aToEnd)!)).toEqual(["b", "c", "a"]);
    // c let go at slot 0: before a.
    const cToStart = dragged(makeReorderState("c", layout, GAP), -1000);
    expect(reorderedTabs(order, "c", commitTarget(order, cToStart)!)).toEqual(["c", "a", "b"]);
    // b let go where it started: no change.
    const bStays = dragged(makeReorderState("b", layout, GAP), 2);
    expect(reorderedTabs(order, "b", commitTarget(order, bStays)!)).toBe(order);
    // The tab closed under the drag: nothing to commit.
    expect(commitTarget(["a", "c"], bStays)).toBeNull();
  });

  it("draws the dragged tab under the pointer inside the strip and opens a gap at the target", () => {
    const content = X0 + W.a + GAP + W.b + GAP + W.c;
    const s = dragged(makeReorderState("a", layout, GAP), 70);
    expect(renderX(s, "a", X0, content)).toBe(X0 + 70);
    expect(renderX(dragged(s, -50), "a", X0, content)).toBe(X0);
    expect(renderX(dragged(s, 5000), "a", X0, content)).toBe(content - W.a);
    // Target 1: b stays at the front, c moves over to make room for a.
    expect(s.targetIndex).toBe(1);
    expect(renderX(s, "b", layout[1].x, content)).toBe(X0);
    expect(renderX(s, "c", layout[2].x, content)).toBe(X0 + W.b + GAP + W.a + GAP);
    expect(renderX(null, "c", layout[2].x, content)).toBe(layout[2].x);
  });
});
