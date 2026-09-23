import { useEffect, useState } from "react";
import { useEditor } from "../state/store";
import { importImages, openProject } from "../actions/files";
import { runAction } from "../shortcuts/useShortcuts";
import { activeLayer } from "../state/selection";
import {
  addAdjustmentLayer, addMaskToActive, blurMaskOfActive, canClipActive, canEditAdjustment, canInvert, canMoveActiveBy,
  deleteMaskOfActive, deleteSelected, editAdjustmentLayer, fillMaskOfActive, flipSelected, invertMaskOfActive,
  mergeTitle, toggleMaskEnabled, toggleMaskLink,
} from "../actions/layers";
import { ADJUSTMENT_KINDS } from "../engine/types";

type Item = { id: string; label: string; run(): void; enabled?: boolean } | "separator";

export function MenuBar() {
  const s = useEditor();
  const hasDoc = s.activeId !== null;
  // Computed once, guarded, and reused below so an item's `enabled` flag never has to
  // index `s.documents` with a possibly-null activeId (which threw when no document was open).
  const activeDoc = s.activeId ? s.documents[s.activeId] : null;
  const active = activeDoc ? activeLayer(activeDoc) : null;
  const hasMask = !!active?.hasMask;
  const [recent, setRecent] = useState<string[]>([]);
  // Refetch only when the bridge changes, the open-document list changes, or a file
  // action explicitly records a new recent package (`recentTick`) - not on every store
  // update via `s.documents`, which changed on nearly every edit and refetched recents
  // far more often than they could actually change.
  useEffect(() => { s.bridge?.recentPackages().then(setRecent); }, [s.bridge, s.order, s.recentTick]);
  const menus: { title: string; items: Item[] }[] = [
    { title: "File", items: [
      { id: "new", label: "New Canvas...", run: () => runAction("new") },
      { id: "open", label: "Open Project...", run: () => runAction("open") },
      ...recent.map((p, i) => ({ id: `recent-${i}`, label: `Open Recent: ${s.bridge!.baseName(p)}`, run: () => void openProject(p) })),
      // No keyboard shortcut (and so no ActionId) covers importing images, so this one
      // keeps calling the action directly instead of going through runAction.
      { id: "import", label: "Import Images...", run: () => void importImages() },
      "separator",
      { id: "save", label: "Save", run: () => runAction("save"), enabled: hasDoc },
      { id: "save-as", label: "Save As...", run: () => runAction("save-as"), enabled: hasDoc },
      { id: "export-png", label: "Export PNG...", run: () => runAction("export-png"), enabled: hasDoc },
      { id: "export-jpeg", label: "Export JPEG...", run: () => runAction("export-jpeg"), enabled: hasDoc },
      "separator",
      { id: "close", label: "Close", run: () => runAction("close"), enabled: hasDoc },
    ] },
    { title: "Edit", items: [
      // A pending transform owns the gesture: macOS's `canUseHistory` requires
      // `transformEdit == nil`, so both items grey out until it commits or cancels.
      { id: "undo", label: "Undo", run: () => runAction("undo"), enabled: hasDoc && !s.transformEdit && !!activeDoc?.canUndo },
      { id: "redo", label: "Redo", run: () => runAction("redo"), enabled: hasDoc && !s.transformEdit && !!activeDoc?.canRedo },
    ] },
    { title: "Layer", items: [
      { id: "layer-new", label: "New Layer", run: () => runAction("new-layer"), enabled: hasDoc },
      { id: "layer-new-folder", label: "New Folder", run: () => runAction("new-folder"), enabled: hasDoc },
      { id: "layer-duplicate", label: "Duplicate Layer", run: () => runAction("duplicate"), enabled: hasDoc },
      { id: "layer-group", label: "Group Layers", run: () => runAction("group"), enabled: hasDoc },
      { id: "layer-merge", label: mergeTitle(), run: () => runAction("merge"), enabled: hasDoc },
      "separator",
      ...ADJUSTMENT_KINDS.map((kind) => ({
        id: `layer-adjustment-${kind.toLowerCase().replace(/[^a-z]+/g, "-")}`,
        label: `New ${kind} Adjustment...`, run: () => addAdjustmentLayer(kind), enabled: hasDoc && !s.adjustEdit,
      })),
      { id: "layer-edit-adjustment", label: "Edit Adjustment...", run: () => editAdjustmentLayer(), enabled: canEditAdjustment() },
      "separator",
      { id: "layer-mask-reveal", label: "Add Mask (Reveal All)", run: () => addMaskToActive(true), enabled: hasDoc && !!active && !hasMask },
      { id: "layer-mask-hide", label: "Add Mask (Hide All)", run: () => addMaskToActive(false), enabled: hasDoc && !!active && !hasMask },
      { id: "layer-mask-delete", label: "Delete Mask", run: () => deleteMaskOfActive(), enabled: hasMask },
      { id: "layer-mask-toggle", label: active?.maskEnabled === false ? "Enable Mask" : "Disable Mask", run: () => toggleMaskEnabled(), enabled: hasMask },
      { id: "layer-mask-link", label: active?.maskLinked === false ? "Link Mask" : "Unlink Mask", run: () => toggleMaskLink(), enabled: hasMask },
      { id: "layer-mask-invert", label: "Invert Mask", run: () => invertMaskOfActive(), enabled: hasMask },
      { id: "layer-mask-fill-white", label: "Fill Mask White", run: () => fillMaskOfActive(true), enabled: hasMask },
      { id: "layer-mask-fill-black", label: "Fill Mask Black", run: () => fillMaskOfActive(false), enabled: hasMask },
      { id: "layer-mask-blur", label: "Blur/Feather Mask...", run: () => blurMaskOfActive(), enabled: hasMask },
      "separator",
      { id: "layer-clip", label: "Create/Release Clipping Mask", run: () => runAction("clip"), enabled: hasDoc && canClipActive() },
      "separator",
      { id: "layer-flip-h", label: "Flip Layer Horizontal", run: () => flipSelected(true), enabled: hasDoc },
      { id: "layer-flip-v", label: "Flip Layer Vertical", run: () => flipSelected(false), enabled: hasDoc },
      { id: "layer-up", label: "Bring Forward", run: () => runAction("layer-up"), enabled: hasDoc && canMoveActiveBy(1) },
      { id: "layer-down", label: "Send Backward", run: () => runAction("layer-down"), enabled: hasDoc && canMoveActiveBy(-1) },
      "separator",
      { id: "layer-delete", label: "Delete Layer", run: () => deleteSelected(), enabled: hasDoc },
    ] },
    { title: "Filter", items: [
      { id: "filter-gaussian-blur", label: "Gaussian Blur...", run: () => s.beginAdjust({ kind: "GaussianBlur" }), enabled: s.canAdjust() },
      { id: "filter-motion-blur", label: "Motion Blur...", run: () => s.beginAdjust({ kind: "MotionBlur" }), enabled: s.canAdjust() },
      { id: "filter-add-noise", label: "Add Noise...", run: () => s.beginAdjust({ kind: "AddNoise" }), enabled: s.canAdjust() },
      { id: "filter-lens-correction", label: "Lens Correction...", run: () => s.beginAdjust({ kind: "LensCorrection" }), enabled: s.canAdjust() },
    ] },
    { title: "Image", items: [
      { id: "canvas-size", label: "Canvas Size...", run: () => runAction("canvas-size"), enabled: hasDoc },
      { id: "image-size", label: "Image Size...", run: () => runAction("image-size"), enabled: hasDoc },
      "separator",
      { id: "image-levels", label: "Levels...", run: () => runAction("levels"), enabled: s.canAdjust() },
      { id: "image-curves", label: "Curves...", run: () => runAction("curves"), enabled: s.canAdjust() },
      { id: "image-hue-saturation", label: "Hue/Saturation...", run: () => runAction("hue-saturation"), enabled: s.canAdjust() },
      { id: "image-exposure", label: "Exposure...", run: () => s.beginAdjust({ kind: "Exposure" }), enabled: s.canAdjust() },
      { id: "image-gradient-map", label: "Gradient Map...", run: () => s.beginAdjust({ kind: "Gradient Map" }), enabled: s.canAdjust() },
      { id: "image-grain", label: "Grain...", run: () => s.beginAdjust({ kind: "Grain" }), enabled: s.canAdjust() },
      { id: "image-invert", label: s.maskSelected ? "Invert Mask" : "Invert", run: () => runAction("invert"), enabled: canInvert() && !s.adjustEdit },
      "separator",
      // Flip has no keyboard shortcut (and so no ActionId either); keep calling the
      // command directly, same as Import above.
      { id: "flip-h", label: "Flip Canvas Horizontal", run: () => s.run({ type: "FlipCanvas", horizontal: true }), enabled: hasDoc },
      { id: "flip-v", label: "Flip Canvas Vertical", run: () => s.run({ type: "FlipCanvas", horizontal: false }), enabled: hasDoc },
    ] },
    { title: "View", items: [
      { id: "zoom-in", label: "Zoom In", run: () => runAction("zoom-in"), enabled: hasDoc },
      { id: "zoom-out", label: "Zoom Out", run: () => runAction("zoom-out"), enabled: hasDoc },
      { id: "fit", label: "Fit on Screen", run: () => runAction("fit"), enabled: hasDoc },
      { id: "actual", label: "Actual Size", run: () => runAction("actual"), enabled: hasDoc },
    ] },
  ];
  const [openMenu, setOpenMenu] = useState<string | null>(null);
  return (
    <div className="menubar" onMouseLeave={() => setOpenMenu(null)}>
      {menus.map((m) => (
        <div key={m.title} className="menu" onMouseEnter={() => openMenu && setOpenMenu(m.title)}>
          <button data-testid={`menubar-${m.title.toLowerCase()}`} onClick={() => setOpenMenu(openMenu === m.title ? null : m.title)}>{m.title}</button>
          {openMenu === m.title && (
            <div className="menu-items">
              {m.items.map((it, i) => it === "separator" ? <hr key={i} /> : (
                <button key={it.id} data-testid={`menu-${it.id}`} disabled={it.enabled === false || s.busy} onClick={() => { setOpenMenu(null); it.run(); }}>{it.label}</button>
              ))}
            </div>
          )}
        </div>
      ))}
    </div>
  );
}
