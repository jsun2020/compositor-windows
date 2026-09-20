import { useState } from "react";
import { Sheet } from "./Sheet";
import { newCanvas } from "../actions/files";
import { useEditor } from "../state/store";

function dimension(text: string): number | null { const n = Number(text.trim()); return Number.isInteger(n) && n >= 1 && n <= 30_000 ? n : null; }

export function NewCanvasSheet() {
  const [width, setWidth] = useState("1920");
  const [height, setHeight] = useState("1080");
  const close = useEditor((s) => s.closeSheet);
  const w = dimension(width), h = dimension(height);
  return (
    <Sheet title="New Canvas" primary="Create" canConfirm={w !== null && h !== null} onCancel={close} onConfirm={() => newCanvas(w!, h!)}>
      <label>Width <input aria-label="Width" value={width} onChange={(e) => setWidth(e.target.value)} /> px</label>
      <label>Height <input aria-label="Height" value={height} onChange={(e) => setHeight(e.target.value)} /> px</label>
    </Sheet>
  );
}
