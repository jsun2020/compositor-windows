import type { DocumentState, LayerState } from "../engine/types";

export interface Row { layer: LayerState; depth: number; visible: boolean; collapsed: boolean; }

export function layerRows(state: DocumentState, collapsedIds: string[]): Row[] {
  const children = new Map<string | null, LayerState[]>();
  for (const l of state.layers) { const list = children.get(l.parentId) ?? []; list.push(l); children.set(l.parentId, list); }
  const out: Row[] = [];
  const visit = (parent: string | null, depth: number, visible: boolean) => {
    if (depth > 64) return;
    for (const layer of [...(children.get(parent) ?? [])].reverse()) {
      const effective = visible && layer.visible; const collapsed = collapsedIds.includes(layer.id);
      out.push({ layer, depth, visible: effective, collapsed });
      if (layer.isGroup && !collapsed) visit(layer.id, depth + 1, effective);
    }
  };
  visit(null, 0, true);
  return out;
}

export interface DropTarget { parent: string | null; above: string | null; atBottom: boolean; }
export function dropTarget(rows: Row[], index: number, zone: "above" | "below" | "into"): DropTarget {
  const row = rows[index];
  if (zone === "into" && row.layer.isGroup) return { parent: row.layer.id, above: null, atBottom: false };
  if (zone === "above") return { parent: row.layer.parentId, above: row.layer.id, atBottom: false };
  // Below: the next row in the same parent, skipping this row's descendants.
  for (let j = index + 1; j < rows.length; j++) {
    if (rows[j].depth < row.depth) break;
    if (rows[j].depth === row.depth && rows[j].layer.parentId === row.layer.parentId) return { parent: row.layer.parentId, above: rows[j].layer.id, atBottom: false };
  }
  return { parent: row.layer.parentId, above: null, atBottom: true };
}
