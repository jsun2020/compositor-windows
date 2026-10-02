import type { DocumentState, LayerState, LayerTransform } from "../engine/types";
import { boundsOfPoints, cornersOf, fromTuple, type P } from "../tools/transform-geometry";
import type { TransformEdit } from "./store";

export function activeLayer(state: DocumentState): LayerState | null { return state.layers.find((l) => l.id === state.activeLayerId) ?? null; }
export function visibleIds(state: DocumentState): Set<string> {
  const byId = new Map(state.layers.map((l) => [l.id, l]));
  const out = new Set<string>();
  for (const layer of state.layers) {
    let node: LayerState | undefined = layer; let ok = true; let steps = 0;
    while (node && steps++ < 65) { if (!node.visible) { ok = false; break; } node = node.parentId ? byId.get(node.parentId) : undefined; }
    if (ok) out.add(layer.id);
  }
  return out;
}
export function transformsAsGroup(state: DocumentState, selected: string[]): boolean {
  if (selected.length > 1) return true;
  return selected.length === 1 && (state.layers.find((l) => l.id === selected[0])?.isGroup ?? false);
}
export function groupMembers(state: DocumentState, selected: string[]): LayerState[] {
  const visible = visibleIds(state); const byId = new Map(state.layers.map((l) => [l.id, l]));
  return state.layers.filter((l) => {
    if (l.isGroup || !l.hasPixels || !visible.has(l.id)) return false;
    let current: string | null = l.id; let steps = 0;
    while (current && steps++ < 65) { if (selected.includes(current)) return true; current = byId.get(current)?.parentId ?? null; }
    return false;
  });
}
export function groupBox(state: DocumentState, selected: string[]): LayerTransform | null {
  const pts = groupMembers(state, selected).flatMap((l) => cornersOf(l.transform));
  if (pts.length === 0) return null;
  const b = boundsOfPoints(pts);
  return { origin: [b.x, b.y], size: [Math.max(1, b.width), Math.max(1, b.height)], rotation: 0, flipX: false, flipY: false, sampling: "High quality" };
}
export function canTransform(state: DocumentState, selected: string[], maskSelected: boolean): boolean {
  if (transformsAsGroup(state, selected)) return groupMembers(state, selected).length > 0;
  const layer = activeLayer(state);
  if (!layer || layer.isGroup) return false;
  if (maskSelected && layer.hasMask && !layer.maskLinked) return visibleIds(state).has(layer.id);
  return layer.hasPixels && visibleIds(state).has(layer.id);
}

export interface EditedShape { transform: LayerTransform; corners: P[] | null; }
/**
 * The shape currently being transformed: a pending edit's draft (and its corners, once a
 * distortion is in progress), otherwise the group box for a multi-layer/group selection,
 * otherwise the active layer's transform, or its mask placement when the mask alone is
 * selected and unlinked from the layer.
 */
export function editedShape(state: DocumentState, transformEdit: TransformEdit | null, selected: string[], maskSelected: boolean): EditedShape | null {
  if (transformEdit) return { transform: transformEdit.draft, corners: transformEdit.corners ? transformEdit.corners.map(fromTuple) : null };
  if (transformsAsGroup(state, selected)) {
    const box = groupBox(state, selected);
    return box ? { transform: box, corners: null } : null;
  }
  const layer = activeLayer(state);
  if (!layer || layer.isGroup) return null;
  if (!maskSelected && layer.hasPixels && !layer.adjustment && selected.length === 1 && state.selection?.bounds && !state.selection.empty) {
    const b = state.selection.bounds;
    return { transform: { origin: [Math.floor(b.x), Math.floor(b.y)], size: [Math.max(1, Math.ceil(b.x + b.width) - Math.floor(b.x)), Math.max(1, Math.ceil(b.y + b.height) - Math.floor(b.y))], rotation: 0, flipX: false, flipY: false, sampling: "High quality" }, corners: null };
  }
  if (maskSelected && layer.hasMask && !layer.maskLinked) return { transform: layer.maskPlacement ?? layer.transform, corners: null };
  return { transform: layer.transform, corners: null };
}
