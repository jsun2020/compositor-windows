import { useState } from "react";
import { Sheet } from "./Sheet";
import { pastePixels } from "../actions/clipboard";
import { newCanvas } from "../actions/files";
import { useEditor } from "../state/store";

export type CanvasUnit = "px" | "in" | "cm" | "mm";
export function unitScale(unit: CanvasUnit, dpi: number): number { return unit === "px" ? 1 : dpi / (unit === "in" ? 1 : unit === "cm" ? 2.54 : 25.4); }
export function canvasDimension(text: string, unit: CanvasUnit, dpi: number): number | null {
  if (!text.trim()) return null;
  const value = Number(text), pixels = Math.round(value * unitScale(unit,dpi));
  return Number.isFinite(value) && (unit !== "px" || Number.isInteger(value)) && value > 0 && pixels >= 1 && pixels <= 30_000 ? pixels : null;
}

export function NewCanvasSheet() {
  const [width, setWidth] = useState("1920");
  const [height, setHeight] = useState("1080");
  const [unit,setUnit] = useState<CanvasUnit>("px"), [dpi,setDpi] = useState(72);
  const [background,setBackground] = useState("Transparent");
  const close = useEditor((s) => s.closeSheet);
  const w=canvasDimension(width,unit,dpi), h=canvasDimension(height,unit,dpi);
  const valid=w!==null && h!==null && w*h<=100_000_000;
  const changeUnit=(next:CanvasUnit)=>{const ratio=unitScale(unit,dpi)/unitScale(next,dpi);const converted=(value:string)=>next==="px"?String(Math.round(Number(value)*ratio)):String(Number((Number(value)*ratio).toFixed(6)));setWidth(converted(width));setHeight(converted(height));setUnit(next);};
  return (
    <Sheet title="New Canvas" primary="Create" canConfirm={valid} onCancel={close} onConfirm={() => newCanvas(w!,h!,dpi,background==="Transparent"?undefined:background==="White"?1:0)}>
      <label>Width <input aria-label="Width" value={width} onChange={(e) => setWidth(e.target.value)} /> {unit}</label>
      <label>Height <input aria-label="Height" value={height} onChange={(e) => setHeight(e.target.value)} /> {unit}</label>
      <label>Units <select aria-label="Units" value={unit} onChange={e=>changeUnit(e.target.value as CanvasUnit)}>{["px","in","cm","mm"].map(u=><option key={u}>{u}</option>)}</select></label>
      <label>Resolution <select aria-label="Resolution" value={dpi} onChange={e=>setDpi(Number(e.target.value))}><option value={72}>72 DPI</option><option value={300}>300 DPI</option></select></label>
      <label>Background <select aria-label="Background" value={background} onChange={e=>setBackground(e.target.value)}>{["Transparent","White","Black"].map(b=><option key={b}>{b}</option>)}</select></label>
      <button onClick={() => { close(); void pastePixels(true); }}>Paste Image as New Canvas</button>
      <p>{valid?`${w} × ${h} px` : "Choose dimensions up to 30,000 px per side and 100 megapixels."}</p>
    </Sheet>
  );
}
