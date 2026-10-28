import { useState } from "react";
import { useEditor, type CropRatio } from "../state/store";
import { applyRatio, normalizeCustomRatioText, parseCustomRatio, ratioValue } from "../tools/crop-tool";

const RATIO_GROUPS: CropRatio[][] = [["Free", "Canvas"], ["1:1"], ["4:3", "16:9", "2.39:1"], ["3:4", "9:16"]];
const PRESET = new Set(RATIO_GROUPS.flat());

export function CropOptions() {
  const s = useEditor();
  const doc = s.activeId ? s.documents[s.activeId] : null;
  const [customOpen, setCustomOpen] = useState(false);
  const [customText, setCustomText] = useState("4:5");
  if (s.tool !== "crop" || !doc || s.sheet !== null) return null;
  const rect = s.cropRect;
  const apply = () => { if (!rect) return; s.run({ type: "Crop", ...rect }); s.setCropRect(null); };
  const choose = (choice: CropRatio) => {
    if (choice === "Custom") {
      if (parseCustomRatio(s.cropRatio) !== null) setCustomText(s.cropRatio);
      setCustomOpen(true);
      return;
    }
    setCustomOpen(false);
    s.setCropRatio(choice);
    const r = ratioValue(choice, doc);
    if (r !== null && rect) s.setCropRect(applyRatio(rect, r));
  };
  const applyCustom = () => {
    const parsed = parseCustomRatio(customText);
    if (parsed === null) return;
    const normalized = normalizeCustomRatioText(customText);
    if (normalized === null) return;
    s.setCropRatio(normalized);
    if (rect) s.setCropRect(applyRatio(rect, parsed));
    setCustomOpen(false);
  };
  const cancelCustom = () => { setCustomOpen(false); };
  const current = s.cropRatio === "None" || s.cropRatio === "Original" ? (s.cropRatio === "Original" ? "Canvas" : "Free") : s.cropRatio;
  const selectValue = customOpen ? "Custom" : current;
  return (
    <div className="tool-options" data-testid="crop-options">
      <label>Ratio <select aria-label="Ratio" value={selectValue} onChange={(e) => choose(e.target.value as CropRatio)}>
        {RATIO_GROUPS.map((group, i) => (
          <optgroup key={i} label={i === 0 ? "Free / Canvas" : i === 1 ? "Square" : i === 2 ? "Wide" : "Tall"}>
            {group.map((r) => <option key={r} value={r}>{r}</option>)}
          </optgroup>
        ))}
        {!PRESET.has(current) && !customOpen && <option value={current}>{current}</option>}
        <option value="Custom">Custom...</option>
      </select></label>
      {customOpen && (
        <span className="crop-custom-ratio" data-testid="crop-custom-ratio">
          <input aria-label="Custom ratio width:height" value={customText} onChange={(e) => setCustomText(e.target.value)}
            onKeyDown={(e) => { if (e.key === "Enter") applyCustom(); }} placeholder="Width:Height" />
          <button type="button" aria-label="Cancel custom ratio" data-testid="crop-custom-cancel" onClick={cancelCustom}>Cancel</button>
          <button type="button" className="primary" aria-label="Apply custom ratio" data-testid="crop-custom-apply"
            disabled={parseCustomRatio(customText) === null} onClick={applyCustom}>Apply</button>
        </span>
      )}
      <span data-testid="crop-size">{rect ? `${Math.round(rect.width)} x ${Math.round(rect.height)}` : "no selection"}</span>
      <button data-testid="crop-cancel" disabled={!rect} onClick={() => s.setCropRect(null)}>Cancel</button>
      <button data-testid="crop-apply" className="primary" disabled={!rect} onClick={apply}>Apply</button>
    </div>
  );
}
