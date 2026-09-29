import { useEditor } from "../state/store";
import { beginShape, dragShape } from "../tools/shape-draft";

/** The Shape tool's pointer (EditorCanvas.swift:1467-1469, :1532-1537, :1684-1687): a press starts a
 * draft at the nearest pixel corner, a drag shapes it (Shift squares a box or holds a line to 45
 * degrees, Alt grows it from its centre), the release makes the layer; a click makes nothing. The
 * draft is drawn on the overlay alone. Returns the cleanup. */
export function installShapeTool(el: HTMLElement, spaceHeld: () => boolean): () => void {
  let dragging = false;
  const docPoint = (e: PointerEvent) => {
    const s = useEditor.getState(); const d = s.documents[s.activeId!];
    const r = el.getBoundingClientRect();
    return s.viewports[s.activeId!].documentPoint({ x: e.clientX - r.left, y: e.clientY - r.top }, { width: d.width, height: d.height });
  };
  const down = (e: PointerEvent) => {
    const s = useEditor.getState();
    if (s.tool !== "shape" || e.button !== 0 || spaceHeld() || !s.activeId || s.working) return;
    if (s.panelOwnsDocument(true)) return;
    dragging = true;
    el.setPointerCapture(e.pointerId);
    s.setShapeDraft(beginShape(docPoint(e), s.shapeOptions));
  };
  const move = (e: PointerEvent) => {
    const s = useEditor.getState(); const draft = s.shapeDraft;
    if (!dragging || !draft) return;
    s.setShapeDraft(dragShape(draft, docPoint(e), e.shiftKey, e.altKey));
  };
  const up = (e: PointerEvent) => {
    if (!dragging) return;
    dragging = false;
    const s = useEditor.getState(); const draft = s.shapeDraft;
    if (!draft) return;
    s.setShapeDraft(dragShape(draft, docPoint(e), e.shiftKey, e.altKey));
    s.finishShape();
  };
  // A drag that lost its pointer makes nothing.
  const forget = () => { if (dragging) { dragging = false; useEditor.getState().setShapeDraft(null); } };
  el.addEventListener("pointerdown", down); el.addEventListener("pointermove", move); el.addEventListener("pointerup", up);
  el.addEventListener("pointercancel", forget);
  return () => {
    el.removeEventListener("pointerdown", down); el.removeEventListener("pointermove", move); el.removeEventListener("pointerup", up);
    el.removeEventListener("pointercancel", forget);
  };
}
