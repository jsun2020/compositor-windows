import { useEditor } from "../state/store";

function ctx() {
  const s = useEditor.getState();
  if (!s.engine || !s.bridge) throw new Error("Engine not ready");
  return { s, engine: s.engine, bridge: s.bridge };
}

async function guarded(work: () => Promise<void>): Promise<void> {
  const s = useEditor.getState();
  if (s.busy) return;
  s.setBusy(true);
  try { await work(); }
  catch (e) { useEditor.getState().setError(e instanceof Error ? e.message : String(e)); }
  finally { useEditor.getState().setBusy(false); }
}

export function newCanvas(width: number, height: number): void {
  const { s, engine } = ctx();
  const id = engine.newDocument(width, height, true);
  s.openDocument(id);
  s.closeSheet();
}

export async function openProject(path?: string): Promise<void> {
  await guarded(async () => {
    const { s, engine, bridge } = ctx();
    const target = path ?? (await bridge.pickOpenPackage());
    if (!target) return;
    const existing = s.order.find((id) => s.documents[id].path === target);
    if (existing) { s.setActive(existing); return; }
    const files = await bridge.readPackage(target);
    const id = engine.openPackage(files, target);
    s.openDocument(id);
    await bridge.addRecentPackage(target);
    useEditor.getState().bumpRecent();
  });
}

async function saveTo(path: string): Promise<void> {
  const { s, engine, bridge } = ctx();
  const id = s.activeId; if (!id) return;
  const files = engine.savePackage(id);
  await bridge.writePackage(path, files);
  engine.markSaved(id, path);
  s.refresh(id);
  await bridge.addRecentPackage(path);
  useEditor.getState().bumpRecent();
}

export async function saveProject(): Promise<void> {
  await guarded(async () => {
    const { s } = ctx();
    const doc = s.activeId ? s.documents[s.activeId] : null;
    if (!doc) return;
    if (doc.path) await saveTo(doc.path); else await saveAsFlow();
  });
}

/** Unguarded body shared by `saveProjectAs` and `saveProject`'s no-path fallback, so
 * neither entry point double-guards (sets busy / routes errors to the banner twice). */
async function saveAsFlow(): Promise<void> {
  const { s, bridge } = ctx();
  const doc = s.activeId ? s.documents[s.activeId] : null;
  if (!doc) return;
  const suggested = doc.path ? bridge.baseName(doc.path) : "Untitled";
  const path = await bridge.pickSavePackage(suggested);
  if (path) await saveTo(path);
}

export async function saveProjectAs(): Promise<void> {
  await guarded(saveAsFlow);
}

export async function importImages(paths?: string[], at?: { x: number; y: number }): Promise<void> {
  await guarded(async () => {
    // Import records history through `Engine::edit` without going through `store.run`, so the
    // commit that every other recording path gets has to happen here. Otherwise an import
    // during a pending distortion leaves the edit open across a structural change.
    useEditor.getState().commitTransform();
    const { s, engine, bridge } = ctx();
    const files = paths ?? (await bridge.pickImportImages());
    const failures: string[] = [];
    let target = s.activeId;
    for (const path of files) {
      try {
        const bytes = await bridge.readFile(path);
        const id = engine.importImage(target, bytes, bridge.baseName(path), target ? at ?? null : null);
        // Import does not go through `store.run`, so the reveal has to be called here: an
        // import with a folder active lands inside it (`import_raster` sets parent_id), and a
        // collapsed folder would otherwise leave the new layer active with no row.
        if (!target) { target = id; useEditor.getState().openDocument(id); }
        else {
          useEditor.getState().refresh(id);
          // Only for the document on screen, for the same reason `refresh` reconciles only
          // that one: a background import must not reopen folders the user collapsed here.
          if (id === useEditor.getState().activeId) useEditor.getState().revealActiveLayer();
        }
      } catch (e) { failures.push(`${bridge.baseName(path)}: ${e instanceof Error ? e.message : String(e)}`); }
    }
    if (failures.length) throw new Error(failures.join("\n"));
  });
}

export async function exportPng(): Promise<void> {
  await guarded(async () => {
    const { s, engine, bridge } = ctx();
    const doc = s.activeId ? s.documents[s.activeId] : null;
    if (!doc) return;
    const path = await bridge.pickExportFile(doc.path ? bridge.baseName(doc.path) : "Untitled", "png");
    if (!path) return;
    await bridge.writeFile(path, engine.exportPng(doc.id));
  });
}

/** Returns false when the user cancelled. */
export async function closeActive(): Promise<boolean> {
  const { s } = ctx();
  const doc = s.activeId ? s.documents[s.activeId] : null;
  if (!doc) return true;
  if (doc.isModified) {
    const save = window.confirm(`Save changes to ${doc.path ? s.bridge!.baseName(doc.path) : "Untitled"} before closing?\n\nOK saves, Cancel keeps the document open.`);
    if (!save) return false;
    await saveProject();
    if (useEditor.getState().documents[doc.id]?.isModified) return false;
  }
  useEditor.getState().closeDocument(doc.id);
  return true;
}
