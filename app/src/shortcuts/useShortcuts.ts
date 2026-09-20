import { useEffect } from "react";
import { matchShortcut, type ActionId } from "./keymap";
import { useEditor } from "../state/store";
import { closeActive, exportPng, openProject, saveProject, saveProjectAs } from "../actions/files";
import { isEditableTarget } from "./target";

export function runAction(id: ActionId): void {
  const s = useEditor.getState();
  const doc = s.activeId ? s.documents[s.activeId] : null;
  const vp = s.activeId ? s.viewports[s.activeId] : null;
  const zoomBy = (f: number) => { if (doc && vp) { vp.setZoom(vp.zoom * f, vp.center, { width: doc.width, height: doc.height }); s.invalidate(); } };
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
    case "crop-apply": if (doc && s.tool === "crop") { const r = s.cropRect ?? { x: 0, y: 0, width: doc.width, height: doc.height }; s.run({ type: "Crop", ...r }); s.setCropRect(null); } break;
    case "crop-cancel": if (s.tool === "crop") s.setCropRect(null); break;
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
      runAction(id);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
}
