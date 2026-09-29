import { useEditor } from "../state/store";
import type { PaletteColor } from "../tools/color";

/** Where a press on the canvas samples colour (EditorCanvas.swift:1419-1424): into the open colour
 * picker, whatever the tool; else null. */
export function samplingInto(): "picker" | null {
  return useEditor.getState().colorPicker ? "picker" : null;
}

/** The colour the sample ring shows as sampled: the picker's working colour. */
function current(): PaletteColor | null {
  return useEditor.getState().pickerColor();
}

/** Samples the canvas under the pointer while it is pressed (`sampleColor`, EditorCanvas.swift:2040-2055):
 * a press reads the pixel under it and a drag keeps reading, at most once a frame; the ring shows the
 * colour sampled over the colour the press began with, until release. Registered in the capture phase
 * so no tool gesture starts under it. Returns the cleanup. */
export function installSampling(el: HTMLElement, spaceHeld: () => boolean): () => void {
  let original: PaletteColor | null = null;
  let pending: PointerEvent | null = null;
  let frame = 0;
  const view = (e: PointerEvent) => { const r = el.getBoundingClientRect(); return { x: e.clientX - r.left, y: e.clientY - r.top }; };
  const sample = (e: PointerEvent) => {
    const s = useEditor.getState(); if (!s.activeId || !original) return;
    const vp = s.viewports[s.activeId], d = s.documents[s.activeId];
    const at = view(e);
    s.sampleIntoPicker(vp.documentPoint(at, { width: d.width, height: d.height }));
    const sampled = current();
    if (sampled) s.setSampleRing({ at, sampled, original });
  };
  const down = (e: PointerEvent) => {
    if (e.button !== 0 || spaceHeld() || !samplingInto()) return;
    e.stopImmediatePropagation();
    original = current();
    el.setPointerCapture(e.pointerId);
    sample(e);
  };
  const move = (e: PointerEvent) => {
    if (!original) return;
    e.stopImmediatePropagation();
    pending = e;
    if (!frame) frame = requestAnimationFrame(() => { frame = 0; if (pending) sample(pending); pending = null; });
  };
  const up = (e: PointerEvent) => {
    if (!original) return;
    e.stopImmediatePropagation();
    if (frame) { cancelAnimationFrame(frame); frame = 0; if (pending) sample(pending); pending = null; }
    original = null;
    useEditor.getState().setSampleRing(null);
  };
  el.addEventListener("pointerdown", down, { capture: true });
  el.addEventListener("pointermove", move, { capture: true });
  el.addEventListener("pointerup", up, { capture: true });
  el.addEventListener("pointercancel", up, { capture: true });
  return () => {
    if (frame) cancelAnimationFrame(frame);
    el.removeEventListener("pointerdown", down, { capture: true });
    el.removeEventListener("pointermove", move, { capture: true });
    el.removeEventListener("pointerup", up, { capture: true });
    el.removeEventListener("pointercancel", up, { capture: true });
  };
}
