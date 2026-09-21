import { useEffect } from "react";
import { matchShortcut, type ActionId } from "./keymap";
import { useEditor } from "../state/store";
import { closeActive, exportPng, openProject, saveProject, saveProjectAs } from "../actions/files";
import { isEditableTarget } from "./target";
import { nudgeDelta } from "../tools/transform-session";
import type { Corners, PointTuple } from "../engine/types";
import { activeLayer } from "../state/selection";

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
    case "undo": s.undo(); break;
    case "redo": s.redo(); break;
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
      if (doc && s.tool === "crop") { const r = s.cropRect ?? { x: 0, y: 0, width: doc.width, height: doc.height }; s.run({ type: "Crop", ...r }); s.setCropRect(null); }
      else if (s.transformEdit) s.commitTransform();
      break;
    case "cancel":
      if (s.tool === "crop") s.setCropRect(null);
      else if (s.transformEdit) s.cancelTransform();
      break;
  }
}

export function useShortcuts(): void {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (isEditableTarget(e.target)) return;
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
