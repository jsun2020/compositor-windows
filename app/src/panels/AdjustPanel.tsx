import { useEffect, type ReactNode } from "react";
import { useEditor } from "../state/store";
import { adjustTitle, defaultFilterParams, resetAdjustment } from "../state/adjust-edit";
import { LevelsPanel } from "./LevelsPanel";
import { CurvesPanel } from "./CurvesPanel";
import { HueSaturationPanel } from "./HueSaturationPanel";
import { FilterPanel } from "./FilterPanel";

/** The floating, non-modal shell every adjustment and filter panel opens inside: it routes to
 * the right body by kind and owns Preview, Reset, Cancel and OK plus Enter and Escape. It must
 * float (`position: fixed` in styles.css) rather than sit in the document flow -- the eyedroppers
 * need the canvas underneath to stay clickable, and several e2e specs pin an exact browser
 * window height against the surrounding chrome, which an in-flow panel would change.
 *
 * Levels, Curves and Hue/Saturation are wired to their own bodies here; Exposure, Gradient Map,
 * Grain and the four filters (Gaussian Blur, Motion Blur, Add Noise, Lens Correction) all route
 * to FilterPanel, which switches over the open kind itself. */
// Form controls with their own Enter semantics: a field commits its own value, a `<select>`
// leaves the dropdown's native handling alone (or has no meaningful behaviour to preempt), and a
// focused button's Enter is already a click on that button, not on OK. The panel's own Enter
// answers everything else.
const OWN_ENTER = new Set(["INPUT", "SELECT", "TEXTAREA", "BUTTON"]);

export function AdjustPanel() {
  const s = useEditor();
  const edit = s.adjustEdit;
  useEffect(() => {
    if (!edit) return;
    const key = (e: KeyboardEvent) => {
      const ownsEnter = e.target instanceof HTMLElement && OWN_ENTER.has(e.target.tagName);
      if (e.key === "Escape") { e.preventDefault(); useEditor.getState().cancelAdjust(); }
      if (e.key === "Enter" && !ownsEnter) { e.preventDefault(); useEditor.getState().commitAdjust(); }
    };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [edit]);
  if (!edit) return null;
  const reset = () => {
    if (edit.params) s.updateAdjust({ params: defaultFilterParams(edit.params.filter),cameraRawBypass:[] });
    else s.updateAdjust({ adjustment: resetAdjustment(edit.adjustment!, edit.original) });
  };
  let body: ReactNode;
  if (!edit.params && edit.adjustment!.kind === "Levels") body = <LevelsPanel />;
  else if (!edit.params && edit.adjustment!.kind === "Curves") body = <CurvesPanel />;
  else if (!edit.params && edit.adjustment!.kind === "Hue/Saturation") body = <HueSaturationPanel />;
  else body = <FilterPanel />;
  return (
    <div className={`adjust-panel${edit.params?.filter === "CameraRaw" ? " camera-raw-panel" : ""}`} data-testid="adjust-panel" role="dialog" aria-label={adjustTitle(edit)}>
      <div className="adjust-header" data-testid="adjust-title">{adjustTitle(edit)}</div>
      <div className="adjust-body">{body}</div>
      <div className="adjust-footer">
        <label><input type="checkbox" data-testid="adjust-preview" checked={edit.preview} onChange={(e) => s.setAdjustPreview(e.target.checked)} /> Preview</label>
        <button data-testid="adjust-reset" onClick={reset}>Reset</button>
        <button data-testid="adjust-cancel" onClick={s.cancelAdjust}>Cancel</button>
        <button data-testid="adjust-ok" className="primary" onClick={s.commitAdjust}>OK</button>
      </div>
    </div>
  );
}
