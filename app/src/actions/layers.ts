import { useEditor } from "../state/store";
import { activeLayer } from "../state/selection";
import type { BlendMode } from "../engine/types";
import type { DropTarget } from "../panels/layer-rows";

export const BLEND_MODES: BlendMode[] = ["Normal", "Multiply", "Screen", "Overlay", "Darken", "Lighten", "Difference", "Color Dodge", "Color Burn", "Hue", "Saturation", "Color", "Luminosity"];

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
export function addMaskToActive(revealing: boolean): void { const c = ctx(); if (!c?.active || c.active.hasMask) return; c.s.commitTransform(); c.s.run({ type: "AddMask", id: c.active.id, revealing }); c.s.setMaskSelected(true); }
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
export function toggleClippingOfActive(): void { const c = ctx(); if (!c?.active || c.active.isGroup) return; c.s.run({ type: "ToggleClipping", id: c.active.id }); }
export function canClipActive(): boolean { const c = ctx(); return !!c?.active && !c.active.isGroup && c.engine.canToggleClipping(c.doc.id, c.active.id); }
export function flipSelected(horizontal: boolean): void { const c = ctx(); if (!c) return; c.s.commitTransform(); c.s.run({ type: "FlipLayers", ids: c.selected, horizontal }); }
export function moveActiveBy(offset: number): void { const c = ctx(); if (!c?.active) return; c.s.run({ type: "MoveLayerBy", id: c.active.id, offset }); }
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
