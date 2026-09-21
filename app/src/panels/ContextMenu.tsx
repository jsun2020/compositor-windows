import { useEffect } from "react";
export type MenuItem = { id: string; label: string; run(): void; disabled?: boolean } | "separator";
export function ContextMenu({ at, items, onClose }: { at: { x: number; y: number }; items: MenuItem[]; onClose(): void }) {
  useEffect(() => {
    const close = () => onClose(); const key = (e: KeyboardEvent) => { if (e.key === "Escape") onClose(); };
    window.addEventListener("pointerdown", close); window.addEventListener("keydown", key);
    return () => { window.removeEventListener("pointerdown", close); window.removeEventListener("keydown", key); };
  }, [onClose]);
  return (
    <div className="context-menu" style={{ left: at.x, top: at.y }} onPointerDown={(e) => e.stopPropagation()} role="menu">
      {items.map((it, i) => it === "separator" ? <hr key={i} /> : <button key={it.id} role="menuitem" data-testid={`ctx-${it.id}`} disabled={it.disabled} onClick={() => { onClose(); it.run(); }}>{it.label}</button>)}
    </div>
  );
}
