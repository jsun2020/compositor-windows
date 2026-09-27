import { useEditor, SELECTION_AMOUNT_MAX, type SelectionAmountOperation } from "../state/store";
import type { SelectionMode } from "../engine/types";
import { NumberInput } from "./NumberInput";

const MODES: { mode: SelectionMode; label: string }[] = [{ mode: "Replace", label: "New" }, { mode: "Add", label: "Add" }, { mode: "Subtract", label: "Subtract" }];
const SAMPLE_SIZES = ["Point Sample", "3 by 3 Average", "5 by 5 Average"];

/** The selection tools' header (`LassoControls`, LassoControls.swift:3-155): the Marquee's shape or
 * the Lasso's kind, the mode (showing Shift / Alt while held, or an outline's own mode), the Magic
 * Wand's settings, Anti-alias where edges can be soft, and Expand / Contract / Feather with their
 * amounts; "Empty selection" and Deselect when there is a selection. */
export function SelectionOptions() {
  const s = useEditor();
  const doc = s.activeId ? s.documents[s.activeId] : null;
  if ((s.tool !== "marquee" && s.tool !== "lasso" && s.tool !== "wand") || !doc || s.sheet !== null) return null;
  const o = s.selectionOptions;
  const shown = s.selectionDraft?.mode ?? s.heldSelectionMode ?? o.mode;
  const editable = !s.panelOwnsDocument();
  const canModify = editable && s.hasSelection() && !s.selectionDraft;
  const modify = (operation: SelectionAmountOperation) => {
    const key = operation === "Expand" ? "expand" : operation === "Contract" ? "contract" : "feather";
    return (
      <span className="selection-modify">
        <button data-testid={`selection-${key}`} disabled={!canModify} onClick={(e) => { s.modifySelection(operation, o[key]); e.currentTarget.blur(); }}>{operation}</button>
        <NumberInput label={`${operation} amount`} testId={`selection-${key}-amount`} value={o[key]} min={1} max={SELECTION_AMOUNT_MAX[operation]} step={1}
          disabled={!canModify} onChange={(v) => s.setSelectionOptions({ [key]: Math.round(v) })} /> px
      </span>
    );
  };
  return (
    <div className="tool-options" data-testid="selection-options">
      <strong>{s.tool === "marquee" ? "Marquee" : s.tool === "wand" ? "Magic Wand" : "Lasso"}</strong>
      {s.tool === "marquee" && (
        <span className="segmented" title="Tab switches between Rectangle and Ellipse">
          {(["Rectangle", "Ellipse"] as const).map((k) => (
            <button key={k} data-testid={`marquee-${k.toLowerCase()}`} aria-pressed={o.marquee === k} onClick={(e) => { s.setSelectionOptions({ marquee: k }); e.currentTarget.blur(); }}>{k}</button>
          ))}
        </span>
      )}
      {s.tool === "lasso" && (
        <span className="segmented" title="Tab switches between Freehand and Polygonal">
          {(["Freehand", "Polygonal"] as const).map((k) => (
            <button key={k} data-testid={`lasso-${k.toLowerCase()}`} aria-pressed={o.lasso === k} onClick={(e) => { s.setSelectionOptions({ lasso: k }); e.currentTarget.blur(); }}>{k}</button>
          ))}
        </span>
      )}
      <span className="segmented" title="Hold Shift to add or Alt to subtract for one outline">
        {MODES.map(({ mode, label }) => (
          <button key={mode} data-testid={`selection-mode-${mode.toLowerCase()}`} aria-pressed={shown === mode} onClick={(e) => { s.setSelectionOptions({ mode }); e.currentTarget.blur(); }}>{label}</button>
        ))}
      </span>
      {s.tool === "wand" && (
        <>
          <label title="How far each colour channel (0-255) can differ from the clicked colour and still be selected">Tolerance{" "}
            <NumberInput label="Tolerance" testId="wand-tolerance" value={o.wand.tolerance} min={0} max={255} step={1} onChange={(v) => s.setSelectionOptions({ wand: { ...o.wand, tolerance: Math.round(v) } })} />
          </label>
          <select aria-label="Sample Size" data-testid="wand-sample-size" value={o.wand.sampleRadius} onChange={(e) => s.setSelectionOptions({ wand: { ...o.wand, sampleRadius: Number(e.target.value) } })}>
            {SAMPLE_SIZES.map((label, radius) => <option key={radius} value={radius}>{label}</option>)}
          </select>
          <select aria-label="Sample" data-testid="wand-sample" value={o.wand.allLayers ? "all" : "this"} onChange={(e) => s.setSelectionOptions({ wand: { ...o.wand, allLayers: e.target.value === "all" } })}>
            <option value="this">This Layer</option><option value="all">All Layers</option>
          </select>
          <label title="Select only similar pixels connected to the one you click; off selects them everywhere">
            <input type="checkbox" data-testid="wand-contiguous" checked={o.wand.contiguous} onChange={(e) => s.setSelectionOptions({ wand: { ...o.wand, contiguous: e.target.checked } })} /> Contiguous
          </label>
        </>
      )}
      {/* Rectangles snap to whole pixels, so Anti-alias does not apply to them (LassoControls.swift:48-52). */}
      {(s.tool !== "marquee" || o.marquee === "Ellipse") && (
        <label title="Smooth selection edges; turn off for hard pixel edges">
          <input type="checkbox" data-testid="selection-antialias" checked={o.antialiased} onChange={(e) => s.setSelectionOptions({ antialiased: e.target.checked })} /> Anti-alias
        </label>
      )}
      {modify("Expand")}
      {modify("Contract")}
      {modify("Feather")}
      {doc.selection && (
        <>
          {doc.selection.empty && <span data-testid="selection-empty" className="hint">Empty selection</span>}
          <button data-testid="selection-deselect" disabled={!editable} onClick={(e) => { s.run({ type: "Deselect" }); e.currentTarget.blur(); }}>Deselect</button>
        </>
      )}
    </div>
  );
}
