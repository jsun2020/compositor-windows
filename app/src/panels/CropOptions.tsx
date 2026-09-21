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
  // No rectangle means nothing to apply: the frame is not drawn and both buttons are disabled,
  // as macOS's CropControls does. The rectangle is seeded when the crop tool is chosen and
  // cleared by Apply and Cancel, so applying a crop always makes the frame disappear.
  const rect = s.cropRect;
  const apply = () => { if (!rect) return; s.run({ type: "Crop", ...rect }); s.setCropRect(null); };
  return (
    <div className="tool-options" data-testid="crop-options">
      <label>Ratio <select value={s.cropRatio} onChange={(e) => { const choice = e.target.value as CropRatio; s.setCropRatio(choice); const r = ratioValue(choice, doc); if (r !== null && rect) s.setCropRect(applyRatio(rect, r)); }}>
        {RATIOS.map((r) => <option key={r} value={r}>{r}</option>)}
      </select></label>
      <span data-testid="crop-size">{rect ? `${Math.round(rect.width)} x ${Math.round(rect.height)}` : "no selection"}</span>
      <button data-testid="crop-cancel" disabled={!rect} onClick={() => s.setCropRect(null)}>Cancel</button>
      <button data-testid="crop-apply" className="primary" disabled={!rect} onClick={apply}>Apply</button>
    </div>
  );
}
