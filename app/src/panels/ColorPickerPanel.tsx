import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
import { pickerTitle, useEditor } from "../state/store";
import { cssColor, hexOf, hsbToRgb, parseHex, withRgb } from "../tools/color";
import { NumberInput } from "./NumberInput";

/** The side of the saturation / brightness field and the height of the hue strip (`fieldSize`). */
export const PICKER_FIELD = 256;
const clamp01 = (v: number) => Math.min(1, Math.max(0, v));
// Fields with their own Enter: a number or the hex field commits itself, a focused button is clicked.
const OWN_ENTER = new Set(["INPUT", "BUTTON"]);

/** The colour picker (ColorPickerSheet.swift): a saturation / brightness field, a hue strip, the new
 * colour, OK and Cancel, R / G / B and hex, and "Click the canvas to sample". It floats, so the canvas
 * stays visible and clickable; its title bar drags it, and it opens again where it was last left
 * (first centred on the canvas). Enter is OK and Escape Cancel, ahead of every other key handler. */
export function ColorPickerPanel() {
  const s = useEditor();
  const picker = s.colorPicker;
  const color = s.pickerColor();
  const [hexDraft, setHexDraft] = useState<string | null>(null);
  const panelRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!picker) return;
    const key = (e: KeyboardEvent) => {
      const own = e.target instanceof HTMLElement && OWN_ENTER.has(e.target.tagName);
      if (e.key === "Escape") { e.preventDefault(); e.stopImmediatePropagation(); useEditor.getState().closeColorPicker(false); }
      else if (e.key === "Enter" && !own) { e.preventDefault(); e.stopImmediatePropagation(); useEditor.getState().closeColorPicker(true); }
    };
    window.addEventListener("keydown", key, { capture: true });
    return () => window.removeEventListener("keydown", key, { capture: true });
  }, [!!picker]);
  // The first opening centres it on the canvas; after that it stays where it was left.
  useEffect(() => {
    if (!picker || s.pickerAt || !panelRef.current) return;
    const canvas = document.querySelector('[data-testid="canvas-view"]')?.getBoundingClientRect();
    const r = panelRef.current.getBoundingClientRect();
    const x = canvas ? canvas.left + (canvas.width - r.width) / 2 : 100, y = canvas ? canvas.top + (canvas.height - r.height) / 2 : 100;
    s.setPickerAt({ x: Math.max(0, Math.round(x)), y: Math.max(0, Math.round(y)) });
  }, [!!picker]);
  if (!picker || !color) return null;
  const hsb = picker.hsb;
  const drag = (apply: (x: number, y: number) => void) => (e: ReactPointerEvent<HTMLDivElement>) => {
    const el = e.currentTarget;
    const at = (ev: { clientX: number; clientY: number }) => { const r = el.getBoundingClientRect(); apply(ev.clientX - r.left, ev.clientY - r.top); };
    el.setPointerCapture(e.pointerId);
    at(e);
    const move = (ev: PointerEvent) => at(ev);
    const up = () => { el.removeEventListener("pointermove", move); el.removeEventListener("pointerup", up); };
    el.addEventListener("pointermove", move); el.addEventListener("pointerup", up);
  };
  const field = drag((x, y) => { const h = useEditor.getState().colorPicker!.hsb; s.setPickerHsb({ ...h, saturation: clamp01(x / PICKER_FIELD), brightness: 1 - clamp01(y / PICKER_FIELD) }); });
  const strip = drag((_x, y) => { const h = useEditor.getState().colorPicker!.hsb; s.setPickerHsb({ ...h, hue: (1 - clamp01(y / PICKER_FIELD)) * 360 }); });
  const moveTitle = (e: ReactPointerEvent<HTMLDivElement>) => {
    const el = e.currentTarget, start = { x: e.clientX, y: e.clientY }, from = useEditor.getState().pickerAt ?? { x: 0, y: 0 };
    el.setPointerCapture(e.pointerId);
    const move = (ev: PointerEvent) => s.setPickerAt({ x: Math.max(0, from.x + ev.clientX - start.x), y: Math.max(0, from.y + ev.clientY - start.y) });
    const up = () => { el.removeEventListener("pointermove", move); el.removeEventListener("pointerup", up); };
    el.addEventListener("pointermove", move); el.addEventListener("pointerup", up);
  };
  const channel = (label: "Red" | "Green" | "Blue", key: "red" | "green" | "blue") => (
    <label className="picker-channel">{label[0]}
      <NumberInput label={label} testId={`picker-${key}`} value={Math.round(color[key] * 255)} min={0} max={255} step={1}
        onChange={(v) => s.setPickerHsb(withRgb(useEditor.getState().colorPicker!.hsb, { ...color, [key]: Math.round(v) / 255 }))} />
    </label>
  );
  const commitHex = () => {
    if (hexDraft !== null) { const parsed = parseHex(hexDraft); if (parsed) s.setPickerHsb(withRgb(hsb, parsed)); }
    setHexDraft(null);
  };
  const at = s.pickerAt ?? { x: 100, y: 100 };
  const hueColor = cssColor(hsbToRgb({ hue: hsb.hue, saturation: 1, brightness: 1 }));
  const stops = [360, 300, 240, 180, 120, 60, 0].map((h) => cssColor(hsbToRgb({ hue: h, saturation: 1, brightness: 1 }))).join(", ");
  return (
    <div ref={panelRef} className="color-picker" data-testid="color-picker" role="dialog" aria-label={pickerTitle(picker.target)} style={{ left: at.x, top: at.y }}>
      <div className="color-picker-title" data-testid="picker-title" onPointerDown={moveTitle}>{pickerTitle(picker.target)}</div>
      <div className="color-picker-body">
        <div className="picker-field" data-testid="picker-field" aria-label="Saturation and brightness" onPointerDown={field}
          style={{ width: PICKER_FIELD, height: PICKER_FIELD, background: `linear-gradient(to bottom, transparent, #000), linear-gradient(to right, #fff, ${hueColor})` }}>
          <div className="picker-marker" style={{ left: hsb.saturation * PICKER_FIELD - 6, top: (1 - hsb.brightness) * PICKER_FIELD - 6 }} />
        </div>
        <div className="picker-hue" data-testid="picker-hue" aria-label="Hue" onPointerDown={strip} style={{ height: PICKER_FIELD }}>
          <div className="picker-hue-strip" style={{ background: `linear-gradient(to bottom, ${stops})` }} />
          <div className="picker-hue-arrows" style={{ top: (1 - hsb.hue / 360) * PICKER_FIELD - 5 }} />
        </div>
        <div className="picker-side">
          <div className="picker-top">
            <div className="picker-new" data-testid="picker-new" aria-label="New color" style={{ background: cssColor(color) }} />
            <div className="picker-buttons">
              <button className="primary" data-testid="picker-ok" onClick={() => s.closeColorPicker(true)}>OK</button>
              <button data-testid="picker-cancel" onClick={() => s.closeColorPicker(false)}>Cancel</button>
            </div>
          </div>
          <div className="picker-fields">
            {channel("Red", "red")}{channel("Green", "green")}{channel("Blue", "blue")}
            <label className="picker-channel">#
              <input aria-label="Hex color" data-testid="picker-hex" value={hexDraft ?? hexOf(color)} spellCheck={false}
                onChange={(e) => setHexDraft(e.target.value)} onBlur={commitHex}
                onKeyDown={(e) => { if (e.key === "Enter") commitHex(); }} />
            </label>
          </div>
          <div className="picker-hint">Click the canvas to sample</div>
        </div>
      </div>
    </div>
  );
}
