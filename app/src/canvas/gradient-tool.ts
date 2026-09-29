import { useEditor } from "../state/store";
import { GRADIENT_HANDLE_PX, hasLine, snapped45 } from "../state/gradient-edit";

type P = { x: number; y: number };

/** The pending line's ends in view px, when it has a line (`gradientLine`, TransformOverlay.swift:83-88). */
export function gradientLine(): { start: P; end: P } | null {
  const s = useEditor.getState(); const e = s.gradientEdit;
  if (!e || !hasLine(e) || !s.activeId) return null;
  const vp = s.viewports[s.activeId], d = s.documents[s.activeId], size = { width: d.width, height: d.height };
  return { start: vp.viewPoint(e.start, size), end: vp.viewPoint(e.end, size) };
}

/** The Gradient tool's pointer (EditorCanvas.swift:2023-2031, :1539-1546, :1680-1683): a press within
 * 10 view px of an end of the pending line grabs it (the end first), otherwise starts a new line and
 * drags its end; Shift holds the dragged end to 45 degree steps about the other. Each move previews
 * from a reduced copy, at most once a frame; the release previews at full quality. Returns the cleanup. */
export function installGradientTool(el: HTMLElement, spaceHeld: () => boolean): () => void {
  let handle: "start" | "end" | null = null;
  let pending: PointerEvent | null = null;
  let frame = 0;
  const view = (e: PointerEvent): P => { const r = el.getBoundingClientRect(); return { x: e.clientX - r.left, y: e.clientY - r.top }; };
  const docPoint = (at: P): P => {
    const s = useEditor.getState(); const d = s.documents[s.activeId!];
    return s.viewports[s.activeId!].documentPoint(at, { width: d.width, height: d.height });
  };
  const apply = (e: PointerEvent) => {
    const s = useEditor.getState(); const edit = s.gradientEdit;
    if (!handle || !edit || !s.activeId) return;
    let pixel = docPoint(view(e));
    if (e.shiftKey) pixel = snapped45(pixel, handle === "start" ? edit.end : edit.start);
    s.moveGradient(handle === "start" ? { start: pixel } : { end: pixel }, true);
  };
  const down = (e: PointerEvent) => {
    const s = useEditor.getState();
    if (s.tool !== "gradient" || e.button !== 0 || spaceHeld() || !s.activeId) return;
    // No press starts or grabs a line while a panel owns the document or a job's result is still to
    // come (fix round 1, I-1): the Mac's `canPaint` requires neither (EditorSession.swift:608).
    if (s.panelOwnsDocument() || s.working) return;
    const at = view(e);
    const line = gradientLine();
    if (line && Math.hypot(at.x - line.end.x, at.y - line.end.y) <= GRADIENT_HANDLE_PX) handle = "end";
    else if (line && Math.hypot(at.x - line.start.x, at.y - line.start.y) <= GRADIENT_HANDLE_PX) handle = "start";
    else handle = s.beginGradient(docPoint(at)) ? "end" : null;
    if (handle) el.setPointerCapture(e.pointerId);
    s.repaintOverlay();
  };
  const move = (e: PointerEvent) => {
    if (!handle) return;
    pending = e;
    if (!frame) frame = requestAnimationFrame(() => { frame = 0; if (pending) apply(pending); pending = null; });
  };
  const up = (e: PointerEvent) => {
    if (!handle) return;
    if (frame) { cancelAnimationFrame(frame); frame = 0; pending = null; }
    apply(e);
    handle = null;
    useEditor.getState().endGradientDrag();
  };
  const cancel = () => { if (frame) { cancelAnimationFrame(frame); frame = 0; } pending = null; if (handle) { handle = null; useEditor.getState().endGradientDrag(); } };
  el.addEventListener("pointerdown", down); el.addEventListener("pointermove", move); el.addEventListener("pointerup", up);
  el.addEventListener("pointercancel", cancel);
  return () => {
    cancel();
    el.removeEventListener("pointerdown", down); el.removeEventListener("pointermove", move); el.removeEventListener("pointerup", up);
    el.removeEventListener("pointercancel", cancel);
  };
}
