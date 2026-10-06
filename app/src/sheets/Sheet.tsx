import { useEffect, type ReactNode } from "react";

export function Sheet(props: { title: string; primary: string; canConfirm: boolean; fieldOwnsEnter?: boolean; dismissOnly?:boolean; onConfirm(): void; onCancel(): void; children: ReactNode }) {
  useEffect(() => {
    const key = (e: KeyboardEvent) => { if (e.key === "Escape") props.onCancel(); const field=props.fieldOwnsEnter && e.target instanceof HTMLElement && ["INPUT","SELECT","TEXTAREA","BUTTON"].includes(e.target.tagName); if (e.key === "Enter" && props.canConfirm && !field) props.onConfirm(); };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [props]);
  return (
    <div className="sheet-backdrop" role="dialog" aria-label={props.title}>
      <div className="sheet">
        <h2>{props.title}</h2>
        <div className="sheet-body">{props.children}</div>
        <div className="sheet-buttons">
          {!props.dismissOnly&&<button onClick={props.onCancel}>Cancel</button>}
          <button className="primary" disabled={!props.canConfirm} onClick={props.onConfirm}>{props.primary}</button>
        </div>
      </div>
    </div>
  );
}
