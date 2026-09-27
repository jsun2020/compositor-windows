import { useState, type DragEvent } from "react";
import { useEditor } from "../state/store";
import { layerRows, dropTarget, type Row } from "./layer-rows";
import { ContextMenu, type MenuItem } from "./ContextMenu";
import { addFolder, addMaskToActive, canClipActive, deleteSelected, duplicateSelected, editAdjustmentLayer, flipSelected, groupSelected, loadSelection, mergeSelected, mergeTitle, placeDropped, toggleClippingOfActive } from "../actions/layers";

/** Ctrl-click on a thumbnail loads it as a selection, Ctrl-Shift adds and Ctrl-Alt subtracts
 * (`loadMode`, NativeLayerList.swift:1007-1019, :1235-1238); Ctrl-click elsewhere in a row still
 * multi-selects. True when it loaded. */
function loadOnCtrlClick(e: React.MouseEvent, id: string, mask: boolean): boolean {
  if (!(e.ctrlKey || e.metaKey)) return false;
  loadSelection(id, mask, e.altKey ? "Subtract" : e.shiftKey ? "Add" : "Replace");
  return true;
}

type Zone = "above" | "below" | "into";
function zoneFor(e: DragEvent, row: Row): Zone {
  const r = (e.currentTarget as HTMLElement).getBoundingClientRect(); const y = (e.clientY - r.top) / r.height;
  if (row.layer.isGroup && y > 0.25 && y < 0.75) return "into";
  return y < 0.5 ? "above" : "below";
}

export function LayersList() {
  const s = useEditor();
  const doc = s.activeId ? s.documents[s.activeId] : null;
  const collapsed = s.activeId ? s.collapsed[s.activeId] ?? [] : [];
  const [renaming, setRenaming] = useState<{ id: string; name: string } | null>(null);
  const [menu, setMenu] = useState<{ x: number; y: number } | null>(null);
  const [drag, setDrag] = useState<{ id: string } | null>(null);
  const [over, setOver] = useState<{ index: number; zone: Zone } | null>(null);
  // The Delete/Backspace key is handled by the global shortcut hook (delete-layer in
  // keymap.ts / useShortcuts.ts) so it is routed through exactly one place; this panel
  // no longer has its own window key listener for it.
  if (!doc) return <div className="layers" />;
  const rows = layerRows(doc, collapsed);
  // The highlight must promise only drops the engine will accept: dragging a folder into its
  // own subtree, or onto the row it already sits above, is refused by `place_layer` and used
  // to light up anyway, so the row moved back after the drop with no explanation.
  const canDropOn = (id: string, index: number, zone: Zone): boolean => {
    if (!s.engine) return false;
    const target = dropTarget(rows, index, zone);
    if (target.above === id) return false;
    return s.engine.canPlace(doc.id, id, target.parent);
  };
  const select = (e: React.MouseEvent, row: Row) => {
    const id = row.layer.id;
    if (e.shiftKey && doc.activeLayerId) {
      const a = rows.findIndex((r) => r.layer.id === doc.activeLayerId), b = rows.findIndex((r) => r.layer.id === id);
      const [lo, hi] = a < b ? [a, b] : [b, a];
      s.selectLayers(rows.slice(lo, hi + 1).map((r) => r.layer.id), doc.activeLayerId);
    } else if (e.ctrlKey || e.metaKey) {
      const next = s.selectedLayerIds.includes(id) ? s.selectedLayerIds.filter((x) => x !== id) : [...s.selectedLayerIds, id];
      s.selectLayers(next, next.includes(id) ? id : next[0] ?? null);
    } else if (!s.selectedLayerIds.includes(id) || s.selectedLayerIds.length !== 1) s.selectLayers([id], id);
  };
  const menuItems = (): MenuItem[] => [
    { id: "duplicate", label: "Duplicate Layer", run: duplicateSelected },
    { id: "group", label: "Group Layers", run: groupSelected },
    { id: "merge", label: mergeTitle(), run: mergeSelected },
    "separator",
    { id: "mask-reveal", label: "Add Mask (Reveal All)", run: () => addMaskToActive(true) },
    { id: "mask-hide", label: "Add Mask (Hide All)", run: () => addMaskToActive(false) },
    { id: "clip", label: "Create/Release Clipping Mask", run: toggleClippingOfActive, disabled: !canClipActive() },
    "separator",
    { id: "flip-h", label: "Flip Horizontal", run: () => flipSelected(true) },
    { id: "flip-v", label: "Flip Vertical", run: () => flipSelected(false) },
    "separator",
    { id: "delete", label: "Delete", run: deleteSelected },
  ];
  return (
    <div className="layers">
      <div className="layers-header"><span>Layers</span></div>
      <div className="layer-rows" onDragLeave={() => setOver(null)}>
        {rows.map((row, index) => {
          const l = row.layer; const selected = s.selectedLayerIds.includes(l.id);
          return (
            <div key={l.id} data-testid="layer-row" data-layer-id={l.id} data-depth={row.depth} aria-selected={selected}
              data-drop-zone={over?.index === index ? over.zone : undefined}
              className={"layer-row" + (selected ? " selected" : "") + (l.id === doc.activeLayerId ? " active" : "") + (row.visible ? "" : " dimmed")}
              style={{ paddingLeft: 8 + row.depth * 14 }} draggable
              onClick={(e) => select(e, row)}
              onDoubleClick={() => { if (l.adjustment) editAdjustmentLayer(l.id); else setRenaming({ id: l.id, name: l.name }); }}
              onContextMenu={(e) => { e.preventDefault(); if (!selected) s.selectLayers([l.id], l.id); setMenu({ x: e.clientX, y: e.clientY }); }}
              onDragStart={(e) => { setDrag({ id: l.id }); e.dataTransfer.setData("text/plain", l.id); e.dataTransfer.effectAllowed = "copyMove"; }}
              onDragOver={(e) => {
                if (!drag || drag.id === l.id) return;
                const zone = zoneFor(e, row);
                if (!canDropOn(drag.id, index, zone)) { e.dataTransfer.dropEffect = "none"; setOver(null); return; }
                e.preventDefault(); setOver({ index, zone });
              }}
              onDrop={(e) => { e.preventDefault(); if (!drag) return; const zone = zoneFor(e, row); placeDropped(drag.id, dropTarget(rows, index, zone), e.altKey); setDrag(null); setOver(null); }}
              onDragEnd={() => { setDrag(null); setOver(null); }}>
              {l.isGroup ? <button data-testid={`collapse-${l.id}`} className="disclosure" onClick={(e) => { e.stopPropagation(); s.toggleCollapsed(l.id); }}>{row.collapsed ? ">" : "v"}</button> : <span className="disclosure-space" />}
              <input type="checkbox" aria-label={`Visible ${l.name}`} checked={l.visible} onClick={(e) => e.stopPropagation()} onChange={(e) => s.run({ type: "SetLayerVisible", id: l.id, visible: e.target.checked })} />
              {l.maskSourceId && <span className="clip-arrow" title="Clipped to the layer below">{">"}</span>}
              {/* "content", not "pixels": the latter's letters collide with the Transform inspector's
                  single-letter "X" field under Playwright's substring accessible-name matching
                  (getByLabel("X") would otherwise also match "... pixels"). */}
              {l.adjustment
                ? <button data-testid={`adjustment-chip-${l.id}`} className="chip chip-adjustment" aria-label={`${l.adjustment.kind} adjustment`}
                    aria-pressed={l.id === doc.activeLayerId} onClick={(e) => { e.stopPropagation(); s.selectLayers([l.id], l.id); }} />
                : <button data-testid={`target-pixels-${l.id}`} className={"chip" + (l.isGroup ? " chip-folder" : " chip-pixels")} aria-label={`${l.name} content`} aria-pressed={l.id === doc.activeLayerId && !s.maskSelected} onClick={(e) => { e.stopPropagation(); if (loadOnCtrlClick(e, l.id, false)) return; s.selectLayers([l.id], l.id); s.setMaskSelected(false); }} />}
              {l.hasMask && <button data-testid={`target-mask-${l.id}`} className={"chip chip-mask" + (l.maskEnabled ? "" : " disabled")} aria-label={`${l.name} mask`} aria-pressed={l.id === doc.activeLayerId && s.maskSelected} onClick={(e) => { e.stopPropagation(); if (loadOnCtrlClick(e, l.id, true)) return; s.selectLayers([l.id], l.id); s.setMaskSelected(true); }} />}
              {renaming?.id === l.id ? (
                <input autoFocus value={renaming.name} onClick={(e) => e.stopPropagation()} onChange={(e) => setRenaming({ id: l.id, name: e.target.value })}
                  onBlur={() => { if (renaming.name.trim()) s.run({ type: "RenameLayer", id: l.id, name: renaming.name.trim() }); setRenaming(null); }}
                  onKeyDown={(e) => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); if (e.key === "Escape") setRenaming(null); }} />
              ) : <span className="layer-name">{l.name}</span>}
            </div>
          );
        })}
      </div>
      <div className="layers-footer">
        <button data-testid="layer-add" title="New layer" onClick={() => s.run({ type: "AddBlankLayer" })}>+</button>
        <button data-testid="layer-add-folder" title="New folder" onClick={addFolder}>[ ]</button>
        <button data-testid="layer-add-mask" title="Add mask" onClick={() => addMaskToActive(true)}>M</button>
        <button data-testid="layer-delete" title="Delete" onClick={deleteSelected}>x</button>
      </div>
      {menu && <ContextMenu at={menu} items={menuItems()} onClose={() => setMenu(null)} />}
    </div>
  );
}
