import { useEffect, useState } from "react";
import { useEditor } from "../state/store";
import type { LayerState } from "../engine/types";

const GROUP_MARKER = String.fromCharCode(0x25b8) + " ";

function depth(layer: LayerState, all: LayerState[]): number {
  let d = 0; let p = layer.parentId;
  while (p && d < 64) { d++; p = all.find((l) => l.id === p)?.parentId ?? null; }
  return d;
}

export function LayersList() {
  const s = useEditor();
  const doc = s.activeId ? s.documents[s.activeId] : null;
  const [renaming, setRenaming] = useState<{ id: string; name: string } | null>(null);
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if ((e.key === "Delete" || e.key === "Backspace") && doc?.activeLayerId && !(e.target instanceof HTMLInputElement) && !s.sheet) s.run({ type: "DeleteLayer", id: doc.activeLayerId });
    };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [doc, s]);
  if (!doc) return <div className="layers" />;
  return (
    <div className="layers">
      <div className="layers-header"><span>Layers</span><button data-testid="layer-add" onClick={() => s.run({ type: "AddBlankLayer" })}>+</button></div>
      {[...doc.layers].reverse().map((l) => (
        <div key={l.id} data-testid="layer-row" className={"layer-row" + (l.id === doc.activeLayerId ? " active" : "")}
          style={{ paddingLeft: 8 + depth(l, doc.layers) * 14 }}
          onClick={() => s.run({ type: "SetActiveLayer", id: l.id })}
          onDoubleClick={() => setRenaming({ id: l.id, name: l.name })}>
          <input type="checkbox" aria-label={`Visible ${l.name}`} checked={l.visible} onClick={(e) => e.stopPropagation()} onChange={(e) => s.run({ type: "SetLayerVisible", id: l.id, visible: e.target.checked })} />
          {renaming?.id === l.id ? (
            <input autoFocus value={renaming.name} onChange={(e) => setRenaming({ id: l.id, name: e.target.value })}
              onBlur={() => { if (renaming.name.trim()) s.run({ type: "RenameLayer", id: l.id, name: renaming.name }); setRenaming(null); }}
              onKeyDown={(e) => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); if (e.key === "Escape") setRenaming(null); }} />
          ) : <span>{l.isGroup ? GROUP_MARKER : ""}{l.name}</span>}
        </div>
      ))}
    </div>
  );
}
