import type { DocumentState, LayerState, LayerTransform } from "../engine/types";
import { boundsOfPoints, cornersOf } from "../tools/transform-geometry";

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
