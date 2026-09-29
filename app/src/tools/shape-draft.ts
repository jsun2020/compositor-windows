import type { ShapeSpec } from "../engine/types";
import { dragBox, type P } from "./selection-draft";
import { snapped45 } from "../state/gradient-edit";

/** The Shape tool's kinds, in the order Shift+U and Tab step through them (`ShapeKind.allCases`). */
export type ShapeKind = "Rectangle" | "Ellipse" | "Line";
export const SHAPE_KINDS: ShapeKind[] = ["Rectangle", "Ellipse", "Line"];
/** The tool's settings (EditorSession.swift:236-240): the kind, a rectangle's corner radius and a
 * line's width, in document pixels; the bar's sliders reach 200 and 100, its fields 5000. */
export interface ShapeOptions { kind: ShapeKind; cornerRadius: number; lineWidth: number; }
export const DEFAULT_SHAPE: ShapeOptions = { kind: "Rectangle", cornerRadius: 0, lineWidth: 4 };
export const SHAPE_FIELD_MAX = 5000;

/** A shape being dragged out (`ShapeDraft`): its kind, where it began (whole pixels), its box, a line's
 * free end, and the corner radius fixed when the drag began (rectangles only). */
export interface ShapeDraft { kind: ShapeKind; anchor: P; rect: { x: number; y: number; width: number; height: number }; end: P | null; cornerRadius: number; }

/** Swift's `rounded()`: halves away from zero. */
const swiftRound = (v: number) => Math.sign(v) * Math.round(Math.abs(v));

export function nextShapeKind(kind: ShapeKind): ShapeKind { return SHAPE_KINDS[(SHAPE_KINDS.indexOf(kind) + 1) % SHAPE_KINDS.length]; }

/** A press: the draft starts at the pixel corner nearest `point` (`beginShape`). */
export function beginShape(point: P, options: ShapeOptions): ShapeDraft {
  const anchor = { x: swiftRound(point.x), y: swiftRound(point.y) };
  return { kind: options.kind, anchor, rect: { ...anchor, width: 0, height: 0 }, end: null, cornerRadius: options.kind === "Rectangle" ? options.cornerRadius : 0 };
}

/** A drag to `point` (`dragShape`): Shift squares a box, or holds a line to 45 degree steps; Alt grows
 * the box from its centre. A line's end is where the pointer is, not rounded. */
export function dragShape(draft: ShapeDraft, point: P, square: boolean, fromCenter: boolean): ShapeDraft {
  if (draft.kind === "Line") {
    const end = square ? snapped45(point, draft.anchor) : point;
    return { ...draft, end, rect: dragBox(draft.anchor, end, false, fromCenter) };
  }
  return { ...draft, rect: dragBox(draft.anchor, point, square, fromCenter) };
}

/** What finishing the draft makes (`finishShape`): the engine's shape, or null when its box is under a
 * pixel on a side (a click), as the Mac makes nothing then. A line's box is its ends grown by half its
 * width. */
export function shapeSpec(draft: ShapeDraft, lineWidth: number): ShapeSpec | null {
  if (draft.kind === "Line") {
    const end = draft.end ?? draft.anchor;
    if (Math.abs(end.x - draft.anchor.x) + lineWidth < 1 || Math.abs(end.y - draft.anchor.y) + lineWidth < 1) return null;
    return { kind: "Line", start: [draft.anchor.x, draft.anchor.y], end: [end.x, end.y], width: lineWidth };
  }
  if (draft.rect.width < 1 || draft.rect.height < 1) return null;
  return draft.kind === "Rectangle" ? { kind: "Rectangle", rect: draft.rect, cornerRadius: draft.cornerRadius } : { kind: "Ellipse", rect: draft.rect };
}
