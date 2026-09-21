import { useEffect, useState } from "react";
import { useEditor } from "../state/store";
import { activeLayer } from "../state/selection";
import { BLEND_MODES, addMaskToActive, deleteMaskOfActive, invertMaskOfActive, setBlendModeOfActive, setOpacityOfSelected, toggleMaskEnabled, toggleMaskLink } from "../actions/layers";
import type { BlendMode } from "../engine/types";

export function LayerProperties() {
  const doc = useEditor((s) => (s.activeId ? s.documents[s.activeId] : null));
  const layer = doc ? activeLayer(doc) : null;
  const [opacity, setOpacity] = useState(100);
  useEffect(() => { setOpacity(Math.round((layer?.opacity ?? 1) * 100)); }, [layer?.id, layer?.opacity]);
  if (!doc || !layer) return <div className="layer-properties" />;
  const folder = layer.isGroup;
  return (
    <div className="layer-properties">
      <label>Opacity <input aria-label="Opacity" type="number" min={0} max={100} value={opacity} disabled={folder}
        onChange={(e) => { const v = Number(e.target.value); setOpacity(v); if (Number.isFinite(v)) setOpacityOfSelected(Math.min(100, Math.max(0, v)) / 100); }} /> %</label>
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
