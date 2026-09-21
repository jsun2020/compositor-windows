import { useEffect } from "react";
import { matchShortcut, type ActionId } from "./keymap";
import { useEditor } from "../state/store";
import { closeActive, exportPng, openProject, saveProject, saveProjectAs } from "../actions/files";
import { isEditableTarget } from "./target";
import { nudgeDelta } from "../tools/transform-session";
import type { Corners, PointTuple } from "../engine/types";
import { activeLayer } from "../state/selection";
import { addFolder, cycleBlendMode, deleteSelected, duplicateSelected, groupSelected, mergeSelected, moveActiveBy, setOpacityOfSelected, toggleClippingOfActive } from "../actions/layers";

const NUDGE_KEYS: Partial<Record<ActionId, string>> = { "nudge-left": "ArrowLeft", "nudge-right": "ArrowRight", "nudge-up": "ArrowUp", "nudge-down": "ArrowDown" };

export function runAction(id: ActionId, shift = false): void {
  const s = useEditor.getState();
  const doc = s.activeId ? s.documents[s.activeId] : null;
  const vp = s.activeId ? s.viewports[s.activeId] : null;
  const zoomBy = (f: number) => { if (doc && vp) { vp.setZoom(vp.zoom * f, vp.center, { width: doc.width, height: doc.height }); s.invalidate(); } };
  const nudgeKey = NUDGE_KEYS[id];
  if (nudgeKey) {
    if (!doc || s.tool !== "move") return;
    const delta = nudgeDelta(nudgeKey, shift);
    if (!delta) return;
    if (s.transformEdit) {
      const e = s.transformEdit;
      const draft = { ...e.draft, origin: [e.draft.origin[0] + delta.dx, e.draft.origin[1] + delta.dy] as PointTuple };
      const corners = e.corners ? (e.corners.map(([x, y]) => [x + delta.dx, y + delta.dy]) as Corners) : null;
      s.previewTransform(draft, corners);
    } else if (s.selectedLayerIds.length) {
      const layer = activeLayer(doc);
      const maskAlone = s.maskSelected && !!layer && layer.hasMask && !layer.maskLinked;
      if (maskAlone) {
        // Nudge the mask's own placement, not the layer: one SetMaskPlacement per key press.
        if (s.beginTransform({ persistent: false })) {
          const e = useEditor.getState().transformEdit!;
          const draft = { ...e.draft, origin: [e.draft.origin[0] + delta.dx, e.draft.origin[1] + delta.dy] as PointTuple };
          s.previewTransform(draft, e.corners);
          s.commitTransform();
        }
      } else {
        s.run({ type: "NudgeLayers", ids: s.selectedLayerIds, dx: delta.dx, dy: delta.dy });
      }
    }
    return;
  }
  switch (id) {
    case "new": s.openSheet({ kind: "new" }); break;
    case "open": void openProject(); break;
    case "save": void saveProject(); break;
    case "save-as": void saveProjectAs(); break;
    case "export-png": void exportPng(); break;
    case "export-jpeg": if (doc) s.openSheet({ kind: "jpeg" }); break;
    case "close": void closeActive(); break;
    // Undo is inert while a transform is pending, as macOS's `canUseHistory` makes it
    // (EditorSession: it requires `transformEdit == nil`). Without this an undo can move or
    // delete the layer the pending edit points at, and Enter then commits corners derived
    // from geometry that no longer exists, or fails with "no layer".
    case "undo": if (!s.transformEdit) s.undo(); break;
    case "redo": if (!s.transformEdit) s.redo(); break;
    case "new-layer": if (doc) s.run({ type: "AddBlankLayer" }); break;
    case "canvas-size": if (doc) s.openSheet({ kind: "canvasSize" }); break;
    case "image-size": if (doc) s.openSheet({ kind: "imageSize" }); break;
    case "zoom-in": zoomBy(1.25); break;
    case "zoom-out": zoomBy(0.8); break;
    case "fit": if (doc && vp) { vp.fit({ width: doc.width, height: doc.height }); s.invalidate(); } break;
    case "actual": if (doc && vp) { vp.setZoom(window.devicePixelRatio || 1, vp.center, { width: doc.width, height: doc.height }); s.invalidate(); } break;
    case "tool-move": s.setTool("move"); break;
    case "tool-hand": s.setTool("hand"); break;
    case "tool-zoom": s.setTool("zoom"); break;
    case "tool-crop": s.setTool("crop"); break;
    case "apply":
      if (doc && s.tool === "crop") { const r = s.cropRect; if (r) { s.run({ type: "Crop", ...r }); s.setCropRect(null); } }
      else if (s.transformEdit) s.commitTransform();
      break;
    case "cancel":
      if (s.tool === "crop") s.setCropRect(null);
      else if (s.transformEdit) s.cancelTransform();
      break;
    case "new-folder": addFolder(); break;
    case "duplicate": duplicateSelected(); break;
    case "group": groupSelected(); break;
    case "merge": mergeSelected(); break;
    case "clip": toggleClippingOfActive(); break;
    case "layer-up": moveActiveBy(1); break;
    case "layer-down": moveActiveBy(-1); break;
    case "blend-next": cycleBlendMode(true); break;
    case "blend-prev": cycleBlendMode(false); break;
    case "delete-layer": if (doc && !s.sheet) deleteSelected(); break;
    default:
      if (id.startsWith("opacity-")) { if (s.tool === "move" && doc) typeOpacityDigit(Number(id.slice(8))); }
  }
}

// Module-level: a second digit pressed within 600ms of the first combines with it
// (2 then 5 -> 25%), matching the macOS app's typeOpacityDigit. `apply` is injectable
// so unit tests can assert on the computed opacity without a store.
let digitBuffer: { digit: number; at: number } | null = null;
export function typeOpacityDigit(digit: number, now = Date.now(), apply = setOpacityOfSelected): void {
  let percent: number;
  if (digitBuffer && now - digitBuffer.at < 600) { percent = digitBuffer.digit * 10 + digit; digitBuffer = null; }
  else { percent = digit === 0 ? 100 : digit * 10; digitBuffer = { digit, at: now }; }
  apply(Math.min(100, percent) / 100);
}

/**
 * True when this keydown must be left alone rather than routed to a shortcut. A focused
 * <button> (a tool-rail button, a menu item, a panel footer button, ...) is the common
 * case: `isEditableTarget` treats every HTMLButtonElement as editable so Enter/Space
 * don't double-fire it while it also owns an app-level shortcut, but that guard used to
 * block *every* key while any button had focus - after a mouse click on any panel
 * button, Ctrl+Z, Ctrl+S, the opacity digits and Delete all went dead until focus moved
 * elsewhere. A focused button now only blocks the two keys that activate it (Enter and
 * Space); every other editable target (inputs, selects, textareas, contentEditable)
 * still blocks all keys, via the unchanged `isEditableTarget`.
 */
export function isShortcutBlocked(e: KeyboardEvent): boolean {
  const t = e.target;
  return t instanceof HTMLButtonElement ? (e.key === "Enter" || e.key === " ") : isEditableTarget(t);
}

export function useShortcuts(): void {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (isShortcutBlocked(e)) return;
      // A sheet owns interaction while it is open; Enter/Escape for it are handled by the
      // sheet's own key listener (see Sheet.tsx), not here.
      if (useEditor.getState().sheet) return;
      const id = matchShortcut(e);
      if (!id) return;
      e.preventDefault();
      runAction(id, e.shiftKey);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
}
