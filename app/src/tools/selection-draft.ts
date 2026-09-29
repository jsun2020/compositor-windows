import type { Command, PointTuple, SelectionMode, SelectionShape } from "../engine/types";

/** The three selection tools (Phase 4a): the Marquee (M), the Lasso (L) and the Magic Wand (W). */
export type SelectionTool = "marquee" | "lasso" | "wand";
export type MarqueeKind = "Rectangle" | "Ellipse";
export type LassoKind = "Freehand" | "Polygonal";
export interface P { x: number; y: number; }
export interface Box { x: number; y: number; width: number; height: number; }

export function isSelectionTool(tool: string): tool is SelectionTool { return tool === "marquee" || tool === "lasso" || tool === "wand"; }

/** Alt (with or without Shift) subtracts, Shift adds, otherwise the options bar's choice
 * (`selectionMode(shift:option:)`, Selection.swift:123-125). Ctrl is the Mac's Cmd, Alt its Option. */
export function selectionMode(choice: SelectionMode, shift: boolean, alt: boolean): SelectionMode {
  return alt ? "Subtract" : shift ? "Add" : choice;
}

/** Swift's `rounded()`: halves away from zero (JavaScript's `Math.round` takes -2.5 to -2). */
const rounded = (v: number) => Math.sign(v) * Math.round(Math.abs(v));

/** The box a drag from `anchor` to `point` spans, in whole pixels (`DragBox.rect`, Selection.swift:93-105):
 * `square` evens the sides, `fromCenter` grows it around the anchor. The anchor is already whole. */
export function dragBox(anchor: P, point: P, square: boolean, fromCenter: boolean): Box {
  let dx = rounded(point.x) - anchor.x, dy = rounded(point.y) - anchor.y;
  if (square) {
    const side = Math.max(Math.abs(dx), Math.abs(dy));
    dx = dx < 0 ? -side : side;
    dy = dy < 0 ? -side : side;
  }
  return fromCenter
    ? { x: anchor.x - Math.abs(dx), y: anchor.y - Math.abs(dy), width: Math.abs(dx) * 2, height: Math.abs(dy) * 2 }
    : { x: Math.min(anchor.x, anchor.x + dx), y: Math.min(anchor.y, anchor.y + dy), width: Math.abs(dx), height: Math.abs(dy) };
}

/** A click this close (view px) to a polygon's first corner closes it (EditorCanvas.swift:1996). */
export const CLOSE_RADIUS_PX = 8;
/** Freehand points closer than this (document px) to the last one are skipped (Selection.swift:164-167). */
export const MIN_STEP = 0.25;

/** An outline being drawn with the Marquee or the Lasso (`LassoDraft`, Selection.swift:107-116), in
 * document pixels. Its mode is fixed at the press; `cursor` is the Polygonal Lasso's rubber band. */
export class SelectionDraft {
  points: P[];
  cursor: P | null = null;
  /** The Marquee's whole-pixel starting corner. */
  readonly anchor: P | null;
  /** Shift squares the Marquee only once pressed afresh: a Shift held at the press chose Add
   * (EditorCanvas.swift:1949-1956, :1963-1964). */
  private squareArmed: boolean;
  private constructor(readonly kind: SelectionShape, readonly mode: SelectionMode, at: P, shiftAtPress: boolean) {
    const marquee = kind === "Rectangle" || kind === "Ellipse";
    this.anchor = marquee ? { x: Math.round(at.x), y: Math.round(at.y) } : null;
    this.points = [this.anchor ?? at];
    this.squareArmed = !shiftAtPress;
  }
  static begin(kind: SelectionShape, mode: SelectionMode, at: P, shiftAtPress = false): SelectionDraft {
    return new SelectionDraft(kind, mode, at, shiftAtPress);
  }
  get isMarquee(): boolean { return this.anchor !== null; }
  /** The Marquee's box to `pixel`: the four corners of a whole-pixel box (`dragMarquee`). Never from
   * the centre: Alt subtracts instead (EditorCanvas.swift:1949-1956). */
  dragMarquee(pixel: P, shift: boolean): void {
    if (!this.anchor || !Number.isFinite(pixel.x) || !Number.isFinite(pixel.y)) return;
    if (!shift) this.squareArmed = true;
    const b = dragBox(this.anchor, pixel, this.squareArmed && shift, false);
    this.points = [{ x: b.x, y: b.y }, { x: b.x + b.width, y: b.y }, { x: b.x + b.width, y: b.y + b.height }, { x: b.x, y: b.y + b.height }];
  }
  /** A Freehand point, or a Polygonal corner (`extendLasso`). */
  extend(pixel: P): void {
    if (!Number.isFinite(pixel.x) || !Number.isFinite(pixel.y)) return;
    const last = this.points[this.points.length - 1];
    if (last && Math.hypot(pixel.x - last.x, pixel.y - last.y) < MIN_STEP) return;
    this.points.push(pixel);
  }
  moveCursor(pixel: P | null): void { this.cursor = pixel; }
  /** Backspace drops the last corner; false when none is left, and the draft goes (`removeLastLassoPoint`). */
  removeLast(): boolean { this.points.pop(); return this.points.length > 0; }
  /** A Polygonal click: a double-click, or a click within `CLOSE_RADIUS_PX` of the first corner with
   * three corners down, closes; anything else adds a corner. `firstInView` is the first corner on screen. */
  click(pixel: P, view: P, firstInView: P, clickCount: number): "close" | "extend" {
    if (clickCount >= 2 || (this.points.length >= 3 && Math.hypot(view.x - firstInView.x, view.y - firstInView.y) <= CLOSE_RADIUS_PX)) return "close";
    this.extend(pixel);
    return "extend";
  }
  /** The command that closes the outline (`finishLasso`): the engine deselects in Replace when it
   * encloses nothing, and changes nothing otherwise. */
  finish(antialiased: boolean): Command {
    const points = this.points.map((p) => [p.x, p.y] as PointTuple);
    return { type: "SelectShape", kind: this.kind, points, mode: this.mode, antialiased };
  }
}

/** Dragging the outline itself (`dragSelection`, EditorCanvas.swift:1932-1942): the offset from the
 * press, whole pixels; Shift keeps it on the axis the drag has gone further along. */
export function outlineOffset(start: P, pixel: P, shift: boolean): { dx: number; dy: number } {
  let dx = pixel.x - start.x, dy = pixel.y - start.y;
  if (shift) { if (Math.abs(dx) >= Math.abs(dy)) dy = 0; else dx = 0; }
  return { dx: Math.round(dx) + 0, dy: Math.round(dy) + 0 };
}
