import { useState } from "react";
import { Sheet } from "./Sheet";
import { useEditor } from "../state/store";
import type { Sampling } from "../engine/types";

export function ImageSizeSheet() {
  const s = useEditor();
  const doc = s.documents[s.activeId!];
  const [width, setWidth] = useState(doc.width);
  const [height, setHeight] = useState(doc.height);
  const [resolution, setResolution] = useState(doc.resolution);
  const [locked, setLocked] = useState(true);
  const [sampling, setSampling] = useState<Sampling>("High quality");
  const ok = (n: number) => Number.isFinite(n) && Math.round(n) >= 1 && Math.round(n) <= 30_000;
  const valid = ok(width) && ok(height) && Number.isFinite(resolution) && resolution >= 1 && resolution <= 9600;
  return (
    <Sheet title="Image Size" primary="Apply" canConfirm={valid} onCancel={s.closeSheet}
      onConfirm={() => { s.run({ type: "ImageSize", width: Math.round(width), height: Math.round(height), resolution, sampling }); s.closeSheet(); }}>
      <label>Width <input aria-label="Width" type="number" value={width} onChange={(e) => { const v = Number(e.target.value); setWidth(v); if (locked) setHeight(Math.round(v * doc.height / doc.width)); }} /> px</label>
      <label>Height <input aria-label="Height" type="number" value={height} onChange={(e) => { const v = Number(e.target.value); setHeight(v); if (locked) setWidth(Math.round(v * doc.width / doc.height)); }} /> px</label>
      <label>Resolution <input aria-label="Resolution" type="number" value={resolution} onChange={(e) => setResolution(Number(e.target.value))} /> pixels/inch</label>
      <label><input type="checkbox" checked={locked} onChange={(e) => setLocked(e.target.checked)} /> Constrain proportions</label>
      <label>Resample <select value={sampling} onChange={(e) => setSampling(e.target.value as Sampling)}><option>Nearest</option><option>Smooth</option><option>High quality</option></select></label>
    </Sheet>
  );
}
