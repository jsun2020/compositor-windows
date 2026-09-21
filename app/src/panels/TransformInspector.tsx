import { useEffect, useState } from "react";
import { useEditor } from "../state/store";
import type { TransformEdit } from "../state/store";
import type { DocumentState, LayerTransform, Sampling } from "../engine/types";
import { activeLayer, canTransform, editedShape, groupBox, transformsAsGroup } from "../state/selection";
import { scalePercent, scaledToPercent } from "../tools/transform-geometry";

interface PixelSize { width: number; height: number; }

/** The layer's own pixel dimensions (what Scale % is measured against), the group box
 * size for a group/multi-selection, or null for a mask (which has no pixel size of its
 * own to scale relative to). */
function pixelSizeFor(doc: DocumentState, transformEdit: TransformEdit | null, selected: string[], maskSelected: boolean): PixelSize | null {
  if (transformEdit) {
    if (transformEdit.kind === "mask") return null;
    if (transformEdit.kind === "group") return { width: transformEdit.box.size[0], height: transformEdit.box.size[1] };
    const layer = doc.layers.find((l) => l.id === transformEdit.id);
    return layer ? { width: layer.pixelsWidth, height: layer.pixelsHeight } : null;
  }
  if (transformsAsGroup(doc, selected)) {
    const box = groupBox(doc, selected);
    return box ? { width: box.size[0], height: box.size[1] } : null;
  }
  const layer = activeLayer(doc);
  if (!layer) return null;
  if (maskSelected && layer.hasMask && !layer.maskLinked) return null;
  return { width: layer.pixelsWidth, height: layer.pixelsHeight };
}

function round(v: number): number { return Math.round(v * 1000) / 1000; }

function NumberField({ label, value, onCommit }: { label: string; value: number; onCommit: (v: number) => void }) {
  const [text, setText] = useState(() => String(round(value)));
  const [lastCommitted, setLastCommitted] = useState(value);
  useEffect(() => { setText(String(round(value))); setLastCommitted(value); }, [value]);
  const commit = () => {
    const v = Number(text);
    if (Number.isFinite(v) && v !== lastCommitted) { setLastCommitted(v); onCommit(v); }
    else { setText(String(round(lastCommitted))); }
  };
  return (
    <label>{label}
      <input aria-label={label} type="number" value={text}
        onChange={(e) => setText(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => { if (e.key === "Enter") { e.preventDefault(); commit(); } }} />
    </label>
  );
}

export function TransformInspector() {
  const s = useEditor();
  const doc = s.activeId ? s.documents[s.activeId] : null;
  if (!doc || s.tool !== "move" || s.sheet !== null) return null;
  if (!s.transformEdit && !canTransform(doc, s.selectedLayerIds, s.maskSelected)) return null;
  const shape = editedShape(doc, s.transformEdit, s.selectedLayerIds, s.maskSelected);
  if (!shape) return null;
  const pixel = pixelSizeFor(doc, s.transformEdit, s.selectedLayerIds, s.maskSelected);
  // Mask-editing state doesn't wait for a pending edit: it's true as soon as the mask chip
  // alone is selected, just like the move tool's own Ctrl-corner and Alt-drag handling.
  const activeLayerForTitle = activeLayer(doc);
  const maskAlone = s.maskSelected && !!activeLayerForTitle && activeLayerForTitle.hasMask && !activeLayerForTitle.maskLinked;
  const isMaskEdit = s.transformEdit ? s.transformEdit.kind === "mask" : maskAlone;
  const apply = (mutate: (t: LayerTransform) => LayerTransform) => {
    const st = useEditor.getState();
    const had = !!st.transformEdit;
    if (!had && !st.beginTransform({ persistent: false })) return;
    const draft = mutate(useEditor.getState().transformEdit!.draft);
    st.previewTransform(draft, useEditor.getState().transformEdit!.corners);
    if (!had) st.commitTransform();
  };
  const field = (label: string, value: number, set: (v: number, t: LayerTransform) => LayerTransform) => (
    <NumberField key={label} label={label} value={value} onCommit={(v) => apply((t) => set(v, t))} />
  );
  const t = shape.transform;
  return (
    <div className="tool-options" data-testid="transform-inspector">
      <span>{isMaskEdit ? "Transform Mask" : "Transform"}</span>
      {field("X", t.origin[0], (v, t) => ({ ...t, origin: [v, t.origin[1]] }))}
      {field("Y", t.origin[1], (v, t) => ({ ...t, origin: [t.origin[0], v] }))}
      {pixel && field("Scale", scalePercent(t, pixel), (v, t) => scaledToPercent(t, v, pixel))}
      {field("Angle", t.rotation, (v, t) => ({ ...t, rotation: v % 360 }))}
      <label>Sampling
        <select aria-label="Sampling" value={t.sampling} onChange={(e) => apply((t) => ({ ...t, sampling: e.target.value as Sampling }))}>
          <option>Nearest</option><option>Smooth</option><option>High quality</option>
        </select>
      </label>
      {s.transformEdit?.corners && <>
        <button data-testid="transform-cancel" onClick={() => s.cancelTransform()}>Cancel</button>
        <button data-testid="transform-apply" className="primary" onClick={() => s.commitTransform()}>Apply</button>
      </>}
    </div>
  );
}
