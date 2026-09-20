import { useEditor, type CropRatio } from "../state/store";
import { applyRatio, ratioValue } from "../tools/crop-tool";

const RATIOS: CropRatio[] = ["None", "Original", "1:1", "4:3", "16:9"];

export function CropOptions() {
  const s = useEditor();
  const doc = s.activeId ? s.documents[s.activeId] : null;
  // Hidden while a sheet is open too: a sheet's own primary button is also labeled
  // "Apply", and having both mounted at once makes accessible-name queries ambiguous
  // (and a modal sheet should own interaction while it is up).
  if (s.tool !== "crop" || !doc || s.sheet !== null) return null;
  const rect = s.cropRect ?? { x: 0, y: 0, width: doc.width, height: doc.height };
  const apply = () => { s.run({ type: "Crop", ...rect }); s.setCropRect(null); };
  return (
    <div className="tool-options" data-testid="crop-options">
      <label>Ratio <select value={s.cropRatio} onChange={(e) => { const choice = e.target.value as CropRatio; s.setCropRatio(choice); const r = ratioValue(choice, doc); if (r !== null) s.setCropRect(applyRatio(rect, r)); }}>
        {RATIOS.map((r) => <option key={r} value={r}>{r}</option>)}
      </select></label>
      <span>{Math.round(rect.width)} x {Math.round(rect.height)}</span>
      <button data-testid="crop-cancel" onClick={() => s.setCropRect(null)}>Cancel</button>
      <button data-testid="crop-apply" className="primary" onClick={apply}>Apply</button>
    </div>
  );
}
