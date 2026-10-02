// Dragging a project tab along the strip to reorder it, as Compositor 1.4.5 does (ProjectTabs.swift at
// v1.4.5: TabReorderState :21-43, renderX :147-155, handleReorder :157-173, makeReorderState :175-188,
// commitReorder :192-205; ProjectWorkspace.moveTab, ProjectWorkspace.swift:47-55). Kept free of React
// so the strip and its tests share one definition.

/** A tab where the strip lays it out, in pixels from the strip's left edge. */
export interface TabSlot { id: string; x: number; width: number; }

/** A tab mid-drag: the rest of the tabs laid out as if it were not there (`compactedX`), captured once
 * when the drag starts, and how far the pointer has moved since the press. */
export interface TabReorderState {
  id: string;
  others: string[];
  widths: Record<string, number>;
  compactedX: Record<string, number>;
  startX: number;
  originX: number;
  spacing: number;
  translation: number;
  targetIndex: number;
}

/** A press becomes a drag once it has moved this many pixels (`DragGesture(minimumDistance: 3)`, :353, :161). */
export const TAB_DRAG_THRESHOLD = 3;

/** `order` with `id` moved to `index` in the final order, clamped to the ends; unchanged when `id` is not
 * a tab or already sits there (`moveTab`). Chrome, not a document edit: it never touches undo. */
export function reorderedTabs(order: string[], id: string, index: number): string[] {
  const from = order.indexOf(id);
  if (from < 0) return order;
  const target = Math.min(Math.max(0, index), order.length - 1);
  if (target === from) return order;
  const next = order.filter((o) => o !== id);
  next.splice(target, 0, id);
  return next;
}

/** The drag's starting state: the other tabs packed from the first tab's x with `spacing` between them. */
export function makeReorderState(id: string, visible: TabSlot[], spacing: number): TabReorderState {
  const widths: Record<string, number> = {};
  for (const s of visible) widths[s.id] = s.width;
  const others = visible.map((s) => s.id).filter((o) => o !== id);
  const startX = visible[0]?.x ?? 0;
  const compactedX: Record<string, number> = {};
  let x = startX;
  for (const o of others) { compactedX[o] = x; x += (widths[o] ?? 0) + spacing; }
  const originX = visible.find((s) => s.id === id)?.x ?? startX;
  return { id, others, widths, compactedX, startX, originX, spacing, translation: 0, targetIndex: 0 };
}

/** The gap the dragged tab is nearest: slot k opens where the k-th other tab sits, or after the last one.
 * The first of two equally near slots wins, as Swift's `min(by:)` keeps the first. */
export function nearestSlot(s: TabReorderState): number {
  const x = s.originX + s.translation;
  const last = s.others[s.others.length - 1];
  const end = last !== undefined ? (s.compactedX[last] ?? s.startX) + (s.widths[last] ?? 0) + s.spacing : s.startX;
  const slots = [...s.others.map((o) => s.compactedX[o] ?? s.startX), end];
  let best = 0;
  for (let i = 1; i < slots.length; i++) if (Math.abs(slots[i] - x) < Math.abs(slots[best] - x)) best = i;
  return best;
}

/** The state with the pointer `translation` pixels from the press, and the drop target that gives. */
export function dragged(s: TabReorderState, translation: number): TabReorderState {
  const moved = { ...s, translation };
  return { ...moved, targetIndex: nearestSlot(moved) };
}

/** Where a tab draws mid-drag: the dragged one follows the pointer, held inside the strip; the others open
 * a gap at the drop target. `slotX` is where it sits when nothing is dragged. */
export function renderX(s: TabReorderState | null, id: string, slotX: number, contentWidth: number): number {
  if (!s) return slotX;
  if (id === s.id) {
    const width = s.widths[id] ?? 0;
    return Math.min(Math.max(s.originX + s.translation, s.startX), Math.max(s.startX, contentWidth - width));
  }
  const x = s.compactedX[id];
  const index = s.others.indexOf(id);
  if (x === undefined || index < 0) return slotX;
  return index >= s.targetIndex ? x + (s.widths[s.id] ?? 0) + s.spacing : x;
}

/** The index in `order` the dragged tab moves to when let go (`commitReorder`), or null when it is gone. */
export function commitTarget(order: string[], s: TabReorderState): number | null {
  const from = order.indexOf(s.id);
  if (from < 0) return null;
  let target: number;
  const neighbour = s.targetIndex < s.others.length ? order.indexOf(s.others[s.targetIndex]) : -1;
  const last = s.others.length > 0 ? order.indexOf(s.others[s.others.length - 1]) : -1;
  if (neighbour >= 0) target = neighbour;
  else if (last >= 0) target = last + 1;
  else target = from;
  if (from < target) target -= 1;
  return target;
}
