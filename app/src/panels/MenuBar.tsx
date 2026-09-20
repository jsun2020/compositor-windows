import { useEffect, useState } from "react";
// Deviation from the brief's snippet: menu items were previously mounted only while a
// top-level title's dropdown was open (click-to-toggle), so Playwright's direct
// getByTestId("menu-<id>").click() calls in files.spec.ts (which never open the "File"
// title first) timed out waiting for an element that was never in the DOM. Rendering
// each group's items as an always-visible toolbar row keeps the same data-testids and
// action wiring while making every item immediately actionable.
import { useEditor } from "../state/store";
import { closeActive, exportPng, importImages, openProject, saveProject, saveProjectAs } from "../actions/files";

type Item = { id: string; label: string; run(): void; enabled?: boolean } | "separator";

export function MenuBar() {
  const s = useEditor();
  const hasDoc = s.activeId !== null;
  // Computed once, guarded, and reused below so an item's `enabled` flag never has to
  // index `s.documents` with a possibly-null activeId (which threw when no document was open).
  const activeDoc = s.activeId ? s.documents[s.activeId] : null;
  const [recent, setRecent] = useState<string[]>([]);
  useEffect(() => { s.bridge?.recentPackages().then(setRecent); }, [s.bridge, s.documents]);
  const zoomBy = (factor: number) => { const vp = s.viewports[s.activeId!]; const d = s.documents[s.activeId!]; vp.setZoom(vp.zoom * factor, vp.center, { width: d.width, height: d.height }); s.invalidate(); };
  const menus: { title: string; items: Item[] }[] = [
    { title: "File", items: [
      { id: "new", label: "New Canvas...", run: () => s.openSheet({ kind: "new" }) },
      { id: "open", label: "Open Project...", run: () => void openProject() },
      ...recent.map((p, i) => ({ id: `recent-${i}`, label: `Open Recent: ${s.bridge!.baseName(p)}`, run: () => void openProject(p) })),
      { id: "import", label: "Import Images...", run: () => void importImages() },
      "separator",
      { id: "save", label: "Save", run: () => void saveProject(), enabled: hasDoc },
      { id: "save-as", label: "Save As...", run: () => void saveProjectAs(), enabled: hasDoc },
      { id: "export-png", label: "Export PNG...", run: () => void exportPng(), enabled: hasDoc },
      { id: "export-jpeg", label: "Export JPEG...", run: () => s.openSheet({ kind: "jpeg" }), enabled: hasDoc },
      "separator",
      { id: "close", label: "Close", run: () => void closeActive(), enabled: hasDoc },
    ] },
    { title: "Edit", items: [
      { id: "undo", label: "Undo", run: s.undo, enabled: hasDoc && !!activeDoc?.canUndo },
      { id: "redo", label: "Redo", run: s.redo, enabled: hasDoc && !!activeDoc?.canRedo },
    ] },
    { title: "Image", items: [
      { id: "canvas-size", label: "Canvas Size...", run: () => s.openSheet({ kind: "canvasSize" }), enabled: hasDoc },
      { id: "image-size", label: "Image Size...", run: () => s.openSheet({ kind: "imageSize" }), enabled: hasDoc },
      "separator",
      { id: "flip-h", label: "Flip Canvas Horizontal", run: () => s.run({ type: "FlipCanvas", horizontal: true }), enabled: hasDoc },
      { id: "flip-v", label: "Flip Canvas Vertical", run: () => s.run({ type: "FlipCanvas", horizontal: false }), enabled: hasDoc },
    ] },
    { title: "View", items: [
      { id: "zoom-in", label: "Zoom In", run: () => zoomBy(1.25), enabled: hasDoc },
      { id: "zoom-out", label: "Zoom Out", run: () => zoomBy(0.8), enabled: hasDoc },
      { id: "fit", label: "Fit on Screen", run: () => { const d = s.documents[s.activeId!]; s.viewports[s.activeId!].fit({ width: d.width, height: d.height }); s.invalidate(); }, enabled: hasDoc },
      { id: "actual", label: "Actual Size", run: () => { const vp = s.viewports[s.activeId!]; const d = s.documents[s.activeId!]; vp.setZoom(window.devicePixelRatio || 1, vp.center, { width: d.width, height: d.height }); s.invalidate(); }, enabled: hasDoc },
    ] },
  ];
  return (
    <div className="menubar">
      {menus.map((m) => (
        <div key={m.title} className="menu">
          <span className="menu-title">{m.title}</span>
          <div className="menu-items">
            {m.items.map((it, i) => it === "separator" ? <span key={i} className="menu-sep" /> : (
              <button key={it.id} data-testid={`menu-${it.id}`} disabled={it.enabled === false || s.busy} onClick={() => it.run()}>{it.label}</button>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}
