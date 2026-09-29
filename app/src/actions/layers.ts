import { colorTuple } from "../tools/color";
import { useEditor } from "../state/store";
import { activeLayer, visibleIds } from "../state/selection";
import type { AdjustmentKind, BlendMode, SelectionMode } from "../engine/types";
import { isEditableKind } from "../engine/types";
import type { DropTarget } from "../panels/layer-rows";

/** Every blend mode in the order the Mac's menu lists them and Shift+= / Shift+- steps through them
 * (`LayerBlendMode.allCases`, LayerAppearance.swift:4-13): Normal, then the darkening, lightening,
 * contrast, comparative and component groups. */
export const BLEND_MODES: BlendMode[] = ["Normal", "Darken", "Multiply", "Color Burn", "Linear Burn", "Lighten", "Screen", "Color Dodge",
  "Linear Dodge (Add)", "Overlay", "Soft Light", "Hard Light", "Vivid Light", "Linear Light", "Pin Light", "Hard Mix",
  "Difference", "Exclusion", "Subtract", "Divide", "Hue", "Saturation", "Color", "Luminosity"];

function ctx() { const s = useEditor.getState(); const doc = s.activeId ? s.documents[s.activeId] : null; return doc && s.engine ? { s, doc, engine: s.engine, selected: s.selectedLayerIds, active: activeLayer(doc) } : null; }

export function deleteSelected(): void {
  const c = ctx(); if (!c || c.selected.length === 0) return;
  c.s.commitTransform();
  const dependents = c.engine.clipDependents(c.doc.id, c.selected);
  let bake = false;
  if (dependents.length > 0) bake = window.confirm("This layer supplies a clipping mask.\n\nOK bakes the masked look into the dependent layers. Cancel removes the links instead.");
  c.s.run({ type: "DeleteLayers", ids: c.selected, bake });
}
export function duplicateSelected(): void { const c = ctx(); if (!c?.active || c.active.isGroup) return; c.s.commitTransform(); c.s.run({ type: "DuplicateLayer", id: c.active.id }); }
export function groupSelected(): void { const c = ctx(); if (!c) return; c.s.commitTransform(); c.s.run({ type: "GroupLayers", ids: c.selected }); }
export function mergeSelected(): void { const c = ctx(); if (!c) return; c.s.commitTransform(); if (c.engine.mergeAction(c.doc.id, c.selected)) c.s.run({ type: "MergeLayers", ids: c.selected }); }
export function mergeTitle(): string { const c = ctx(); return (c && c.engine.mergeAction(c.doc.id, c.selected)) || "Merge Down"; }
export function addFolder(): void { const c = ctx(); if (!c) return; c.s.commitTransform(); c.s.run({ type: "AddGroup" }); }
// Adding or deleting a mask is a structural change, so it commits any pending edit first,
// like every other action here. Without that, a pending mask move survives the mask it moves
// and Enter later fails with "the layer has no mask"; macOS disables both menu items while a
// transform is pending (canEditLayers requires transformEdit == nil).
// With a selection, Add Mask paints the opposite tone through it and uses it up, one undo step
// ("Add Mask from Selection", LayerMask.swift:228-260); every Add Mask entry point on the Mac goes
// through that one `addMask`.
export function addMaskToActive(revealing: boolean): void {
  const c = ctx(); if (!c?.active || c.active.hasMask) return; c.s.commitTransform();
  const ok = c.s.run(c.doc.selection ? { type: "AddMaskFromSelection", id: c.active.id, revealing } : { type: "AddMask", id: c.active.id, revealing });
  if (ok) c.s.setMaskSelected(true);
}
/** Whether Delete may clear through the selection now (`canPaint`, EditorSession+Brush.swift:5-11):
 * one layer targeted, shown, a pixel layer or an enabled mask, a selection with something in it,
 * and no crop rectangle pending (`canEditLayers`: `cropRect == nil`). */
export function canClearSelected(): boolean {
  const c = ctx(); if (!c?.active || !c.doc.selection || c.doc.selection.empty || c.selected.length !== 1 || c.s.panelOwnsDocument() || c.s.cropRect) return false;
  if (!visibleIds(c.doc).has(c.active.id)) return false;
  return c.s.maskSelected && c.active.hasMask ? c.active.maskEnabled : !c.active.isGroup && c.active.hasPixels;
}
/** Delete / Backspace (`deleteKeyPressed`, SelectionEdits.swift:60-63): with a selection, clears the
 * selected pixels, or fills the targeted mask white there ("Clear" / "Fill Mask"); an edit that may
 * not paint now does nothing. Without one, deletes the selected layers as before. */
export function deleteKeyPressed(): void {
  const c = ctx(); if (!c) return;
  if (!c.doc.selection) { deleteSelected(); return; }
  if (!canClearSelected()) return;
  c.s.run({ type: "ClearSelectedPixels", id: c.active!.id, mask: c.s.maskSelected && c.active!.hasMask });
}
/** Ctrl-click on a thumbnail, or Select > Layer's Pixels / Mask's Black Areas (MaskTracing.swift:73-94). */
export function loadSelection(id: string, mask: boolean, mode: SelectionMode = "Replace"): void {
  const c = ctx(); if (!c) return;
  const antialiased = c.s.selectionOptions.antialiased;
  c.s.run(mask ? { type: "LoadMaskSelection", id, mode, antialiased } : { type: "LoadLayerSelection", id, mode, antialiased });
}
export function deleteMaskOfActive(): void { const c = ctx(); if (!c?.active?.hasMask) return; c.s.commitTransform(); c.s.run({ type: "DeleteMask", id: c.active.id }); c.s.setMaskSelected(false); }
export function toggleMaskEnabled(): void { const c = ctx(); if (!c?.active?.hasMask) return; c.s.run({ type: "SetMaskEnabled", id: c.active.id, enabled: !c.active.maskEnabled }); }
export function toggleMaskLink(): void { const c = ctx(); if (!c?.active?.hasMask) return; c.s.commitTransform(); c.s.run({ type: "SetMaskLinked", id: c.active.id, linked: !c.active.maskLinked }); }
export function invertMaskOfActive(): void { const c = ctx(); if (!c?.active?.hasMask) return; c.s.run({ type: "InvertMask", id: c.active.id }); }
export function fillMaskOfActive(white: boolean): void { const c = ctx(); if (!c?.active?.hasMask) return; c.s.run({ type: "FillMask", id: c.active.id, white }); }
export function blurMaskOfActive(): void {
  const c = ctx(); if (!c?.active?.hasMask) return;
  const text = window.prompt("Blur radius (pixels)", "4"); if (text === null) return;
  const radius = Number(text); if (!Number.isFinite(radius) || radius <= 0) { c.s.setError("Enter a radius greater than 0."); return; }
  c.s.run({ type: "BlurMask", id: c.active.id, radius });
}
// Guarded here rather than at each call site, so the Ctrl+Alt+G shortcut, the Layer menu item
// and the context menu all behave alike. macOS guards inside `toggleClippingMask` the same way
// and returns silently; nothing below to clip to is a no-op, not an error worth a banner.
export function toggleClippingOfActive(): void { const c = ctx(); if (!c?.active || c.active.isGroup || !canClipActive()) return; c.s.run({ type: "ToggleClipping", id: c.active.id }); }
export function canClipActive(): boolean { const c = ctx(); return !!c?.active && !c.active.isGroup && c.engine.canToggleClipping(c.doc.id, c.active.id); }
export function flipSelected(horizontal: boolean): void { const c = ctx(); if (!c) return; c.s.commitTransform(); c.s.run({ type: "FlipLayers", ids: c.selected, horizontal }); }
/** Whether the active layer has a sibling `offset` steps away to swap with. Mirrors macOS's
 * `canMoveActiveLayer(by:)`: siblings are the layers sharing its parent, in array order. */
export function canMoveActiveBy(offset: number): boolean {
  const c = ctx(); if (!c?.active) return false;
  const siblings = c.doc.layers.filter((l) => l.parentId === c.active!.parentId);
  const index = siblings.findIndex((l) => l.id === c.active!.id);
  return index >= 0 && index + offset >= 0 && index + offset < siblings.length;
}
// Already at the top or bottom of its stack is a no-op, not an error: macOS's
// `moveActiveLayer(by:)` guards on the same predicate and returns without opening a
// transaction. The engine op keeps returning Err for a programmatic caller.
export function moveActiveBy(offset: number): void { const c = ctx(); if (!c?.active || !canMoveActiveBy(offset)) return; c.s.run({ type: "MoveLayerBy", id: c.active.id, offset }); }
export function placeDropped(id: string, target: DropTarget, copy: boolean): void {
  const c = ctx(); if (!c) return;
  if (!c.engine.canPlace(c.doc.id, id, target.parent)) return;
  c.s.run(copy ? { type: "DuplicateLayerTo", id, parent: target.parent, above: target.above, atBottom: target.atBottom } : { type: "PlaceLayer", id, parent: target.parent, above: target.above, atBottom: target.atBottom });
}
export function setOpacityOfSelected(opacity: number): void { const c = ctx(); if (!c) return; c.s.run({ type: "SetLayersOpacity", ids: c.selected, opacity }); }
export function setBlendModeOfActive(mode: BlendMode): void { const c = ctx(); if (!c?.active || c.active.isGroup) return; c.s.run({ type: "SetLayerBlendMode", id: c.active.id, mode }); }
export function cycleBlendMode(forward: boolean): void {
  const c = ctx(); if (!c?.active || c.active.isGroup) return;
  const i = BLEND_MODES.indexOf(c.active.blendMode);
  setBlendModeOfActive(BLEND_MODES[(i + (forward ? 1 : BLEND_MODES.length - 1)) % BLEND_MODES.length]);
}
/** Image > Invert: immediate, one undo step, on the mask when the mask chip is selected. */
export function invertActive(): void {
  const c = ctx(); if (!c?.active || !canInvert()) return;
  const mask = c.s.maskSelected && c.active.hasMask;
  if (!mask && !c.active.hasPixels) return;
  c.s.run({ type: "InvertPixels", id: c.active.id, mask });
}
export function canInvert(): boolean {
  const c = ctx(); if (!c?.active || c.active.isGroup) return false;
  // An empty selection inverts nothing (`canInvert`, SelectionEdits.swift:77-83).
  if (c.doc.selection?.empty) return false;
  return (c.s.maskSelected && c.active.hasMask) || c.active.hasPixels;
}
/** Layer > New <kind> Adjustment. Each Grain layer gets its own pattern, and a Gradient Map
 * starts from the image's foreground to its background (LayerAdjustment.swift:191). */
export function addAdjustmentLayer(kind: AdjustmentKind): void {
  const c = ctx(); if (!c) return;
  const seed = Math.floor(Math.random() * 0xffffffff);
  const { foreground, background } = c.s.palette;
  const ends = kind === "Gradient Map" ? { shadows: colorTuple(foreground), highlights: colorTuple(background) } : { shadows: null, highlights: null };
  c.s.run({ type: "AddAdjustmentLayer", kind, seed, ...ends });
  // The new layer is active; open its panel straight away, as macOS does; Invert has nothing to
  // set, so it just applies (LayerAdjustment.swift:203-204).
  const created = activeLayer(useEditor.getState().documents[c.doc.id]);
  if (created?.adjustment && isEditableKind(kind)) useEditor.getState().beginAdjust({ kind, layerId: created.id, target: "adjustmentLayer" });
}
export function editAdjustmentLayer(id?: string): void {
  const c = ctx(); if (!c) return;
  const layer = id ? c.doc.layers.find((l) => l.id === id) : c.active;
  // Invert has nothing to set, so no panel (AdjustmentKind.isEditable, LayerAdjustment.swift:28).
  if (!layer?.adjustment || !isEditableKind(layer.adjustment.kind)) return;
  c.s.beginAdjust({ kind: layer.adjustment.kind, layerId: layer.id, target: "adjustmentLayer" });
}
export function canEditAdjustment(): boolean {
  const c = ctx();
  return !!c?.active?.adjustment && isEditableKind(c.active.adjustment.kind) && !c.s.panelOwnsDocument();
}
