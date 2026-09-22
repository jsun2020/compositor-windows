import { useEffect, type ReactNode } from "react";
import { useEditor } from "../state/store";
import { adjustTitle, defaultAdjustment, defaultFilterParams } from "../state/adjust-edit";
import { LevelsPanel } from "./LevelsPanel";
import { CurvesPanel } from "./CurvesPanel";

/** The floating, non-modal shell every adjustment and filter panel opens inside: it routes to
 * the right body by kind and owns Preview, Reset, Cancel and OK plus Enter and Escape. It must
 * float (`position: fixed` in styles.css) rather than sit in the document flow -- the eyedroppers
 * need the canvas underneath to stay clickable, and several e2e specs pin an exact browser
 * window height against the surrounding chrome, which an in-flow panel would change.
 *
 * Only Levels and Curves are wired to a real body here; Hue/Saturation and the filter panels
 * belong to later tasks in this plan and are not part of this one's file list, so an
 * unrecognised kind falls back to a plain placeholder instead of importing a module that does
 * not exist yet. */
export function AdjustPanel() {
  const s = useEditor();
  const edit = s.adjustEdit;
  useEffect(() => {
    if (!edit) return;
    const key = (e: KeyboardEvent) => {
      // A field keeps its own Enter (committing the value); the panel answers the second one.
      if (e.key === "Escape") { e.preventDefault(); useEditor.getState().cancelAdjust(); }
      if (e.key === "Enter" && !(e.target instanceof HTMLInputElement)) { e.preventDefault(); useEditor.getState().commitAdjust(); }
    };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [edit]);
  if (!edit) return null;
  const reset = () => {
    if (edit.params) s.updateAdjust({ params: defaultFilterParams(edit.params.filter) });
    else s.updateAdjust({ adjustment: defaultAdjustment(edit.adjustment!.kind) });
  };
  let body: ReactNode;
  if (!edit.params && edit.adjustment!.kind === "Levels") body = <LevelsPanel />;
  else if (!edit.params && edit.adjustment!.kind === "Curves") body = <CurvesPanel />;
  else body = <div className="adjust-placeholder">{adjustTitle(edit)} is not built yet.</div>;
  return (
    <div className="adjust-panel" data-testid="adjust-panel" role="dialog" aria-label={adjustTitle(edit)}>
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
