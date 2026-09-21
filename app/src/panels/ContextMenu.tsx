import { useEffect, useRef } from "react";
export type MenuItem = { id: string; label: string; run(): void; disabled?: boolean } | "separator";
export function ContextMenu({ at, items, onClose }: { at: { x: number; y: number }; items: MenuItem[]; onClose(): void }) {
  // The latest `onClose` is read through a ref rather than captured in the effect. Callers
  // pass an inline arrow, so a new identity arrives on every parent render; with `onClose` in
  // the dependency list the listeners were torn down and re-registered each time.
  const close = useRef(onClose);
  close.current = onClose;
  useEffect(() => {
    const pointer = () => close.current();
    const key = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      // Escape belongs to the open menu alone. Without this the global shortcut handler also
      // sees it and cancels a pending distortion or clears the crop rectangle while the user
      // was only dismissing the menu.
      e.preventDefault();
      e.stopPropagation();
      close.current();
    };
    window.addEventListener("pointerdown", pointer);
    // Capture phase, so this runs before the shortcut listener bound on window.
    window.addEventListener("keydown", key, true);
    return () => { window.removeEventListener("pointerdown", pointer); window.removeEventListener("keydown", key, true); };
  }, []);
  return (
    <div className="context-menu" style={{ left: at.x, top: at.y }} onPointerDown={(e) => e.stopPropagation()} role="menu">
      {items.map((it, i) => it === "separator" ? <hr key={i} /> : <button key={it.id} role="menuitem" data-testid={`ctx-${it.id}`} disabled={it.disabled} onClick={() => { onClose(); it.run(); }}>{it.label}</button>)}
    </div>
  );
}
