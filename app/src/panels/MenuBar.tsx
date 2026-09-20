import { useEffect, useState } from "react";
import { useEditor } from "../state/store";
import { importImages, openProject } from "../actions/files";
import { runAction } from "../shortcuts/useShortcuts";

type Item = { id: string; label: string; run(): void; enabled?: boolean } | "separator";

export function MenuBar() {
  const s = useEditor();
  const hasDoc = s.activeId !== null;
  // Computed once, guarded, and reused below so an item's `enabled` flag never has to
  // index `s.documents` with a possibly-null activeId (which threw when no document was open).
  const activeDoc = s.activeId ? s.documents[s.activeId] : null;
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
      { id: "undo", label: "Undo", run: () => runAction("undo"), enabled: hasDoc && !!activeDoc?.canUndo },
      { id: "redo", label: "Redo", run: () => runAction("redo"), enabled: hasDoc && !!activeDoc?.canRedo },
    ] },
    { title: "Image", items: [
      { id: "canvas-size", label: "Canvas Size...", run: () => runAction("canvas-size"), enabled: hasDoc },
      { id: "image-size", label: "Image Size...", run: () => runAction("image-size"), enabled: hasDoc },
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
