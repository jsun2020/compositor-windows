import { useEffect, useState } from "react";
import { useEditor } from "../state/store";
import { activeLayer } from "../state/selection";
import { BLEND_MODES, addMaskToActive, deleteMaskOfActive, invertMaskOfActive, setBlendModeOfActive, setOpacityOfSelected, toggleMaskEnabled, toggleMaskLink } from "../actions/layers";
import type { BlendMode } from "../engine/types";

export function LayerProperties() {
  const doc = useEditor((s) => (s.activeId ? s.documents[s.activeId] : null));
  const layer = doc ? activeLayer(doc) : null;
  const percent = Math.round((layer?.opacity ?? 1) * 100);
  // Text, not a number: typing "35" over a selected "100" passes through "3", and backspacing
  // passes through "". Committing on every keystroke would write an undo entry per character
  // and land on 0% for the empty string. macOS brackets the whole gesture into one "Layer
  // Opacity" entry (beginOpacityEdit / finishOpacityEdit), so this commits on blur and Enter,
  // the way TransformInspector's NumberField already does, and ignores an empty field.
  const [text, setText] = useState(() => String(percent));
  const [committed, setCommitted] = useState(percent);
  useEffect(() => { setText(String(percent)); setCommitted(percent); }, [layer?.id, percent]);
  if (!doc || !layer) return <div className="layer-properties" />;
  const folder = layer.isGroup;
  const commitOpacity = () => {
    const v = Number(text);
    if (text.trim() === "" || !Number.isFinite(v)) { setText(String(committed)); return; }
    const clamped = Math.min(100, Math.max(0, Math.round(v)));
    setText(String(clamped));
    if (clamped !== committed) { setCommitted(clamped); setOpacityOfSelected(clamped / 100); }
  };
  return (
    <div className="layer-properties">
      <label>Opacity <input aria-label="Opacity" type="number" min={0} max={100} value={text} disabled={folder}
        onChange={(e) => setText(e.target.value)}
        onBlur={commitOpacity}
        onKeyDown={(e) => { if (e.key === "Enter") { e.preventDefault(); commitOpacity(); } }} /> %</label>
      <label>Blend <select aria-label="Blend mode" value={layer.blendMode} disabled={folder} onChange={(e) => setBlendModeOfActive(e.target.value as BlendMode)}>
        {BLEND_MODES.map((m) => <option key={m} value={m}>{m}</option>)}
      </select></label>
      <div className="mask-buttons">
        {!layer.hasMask && <><button onClick={() => addMaskToActive(true)}>Add mask (reveal)</button><button onClick={() => addMaskToActive(false)}>Add mask (hide)</button></>}
        {layer.hasMask && <>
          <button onClick={toggleMaskEnabled}>{layer.maskEnabled ? "Disable mask" : "Enable mask"}</button>
          <button onClick={toggleMaskLink}>{layer.maskLinked ? "Unlink mask" : "Link mask"}</button>
          <button onClick={invertMaskOfActive}>Invert mask</button>
          <button onClick={deleteMaskOfActive}>Delete mask</button>
        </>}
      </div>
    </div>
  );
}
