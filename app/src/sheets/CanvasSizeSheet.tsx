import { useMemo, useState } from "react";
import { Sheet } from "./Sheet";
import { useEditor } from "../state/store";
import { CANVAS_UNITS, CanvasSizeDraft, type CanvasUnit } from "../tools/canvas-size-draft";

export function CanvasSizeSheet() {
  const s = useEditor();
  const doc = s.documents[s.activeId!];
  const draft = useMemo(() => new CanvasSizeDraft(doc.width, doc.height, doc.resolution), [doc]);
  const [, bump] = useState(0);
  const [anchor, setAnchor] = useState(4);
  const [fill, setFill] = useState(false);
  const [color, setColor] = useState("#ffffff");
  const rerender = () => bump((n) => n + 1);
  const field = (widthAxis: boolean) => (
    <input aria-label={widthAxis ? "Width" : "Height"} type="number" value={String(Math.round(draft.displayed(widthAxis) * 1000) / 1000)}
      onChange={(e) => { const v = Number(e.target.value); if (Number.isFinite(v)) { draft.set(v, widthAxis); rerender(); } }} />
  );
  const apply = () => {
    const rgb = fill ? ([1, 3, 5].map((i) => parseInt(color.slice(i, i + 2), 16) / 255) as [number, number, number]) : null;
    s.run({ type: "CanvasSize", width: Math.round(draft.width), height: Math.round(draft.height), anchor, fill: rgb });
    s.closeSheet();
  };
  return (
    <Sheet title="Canvas Size" primary="Apply" canConfirm={draft.valid} onCancel={s.closeSheet} onConfirm={apply}>
      <div>Current: {doc.width} x {doc.height} px</div>
      <label>Width {field(true)}</label>
      <label>Height {field(false)}</label>
      <label>Unit <select value={draft.unit} onChange={(e) => { draft.unit = e.target.value as CanvasUnit; rerender(); }}>{CANVAS_UNITS.map((u) => <option key={u}>{u}</option>)}</select></label>
      <label><input type="checkbox" checked={draft.relative} onChange={(e) => { draft.relative = e.target.checked; rerender(); }} /> Relative</label>
      <label><input type="checkbox" checked={draft.locked} onChange={(e) => { draft.locked = e.target.checked; rerender(); }} /> Constrain proportions</label>
      <div className="anchor-grid">{Array.from({ length: 9 }, (_, i) => <button key={i} data-testid={`anchor-${i}`} className={anchor === i ? "active" : ""} onClick={() => setAnchor(i)}>{i === anchor ? "*" : "."}</button>)}</div>
      <label><input type="checkbox" checked={fill} onChange={(e) => setFill(e.target.checked)} /> Fill extension with color <input type="color" value={color} onChange={(e) => setColor(e.target.value)} disabled={!fill} /></label>
    </Sheet>
  );
}
