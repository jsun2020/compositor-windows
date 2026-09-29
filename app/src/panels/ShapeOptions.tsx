import { useEditor } from "../state/store";
import { SHAPE_FIELD_MAX, SHAPE_KINDS } from "../tools/shape-draft";
import { cssColor } from "../tools/color";
import { NumberInput } from "./NumberInput";

/** The Shape tool's bar (ShapeControls.swift): Rectangle, Ellipse or Line; a rectangle's Radius or a
 * line's Width (sliders to 200 and 100, fields to 5000 px); and the Fill swatch, the foreground colour,
 * which opens the picker. Controls give up the focus once used, as the other bars' do. */
export function ShapeOptions() {
  const s = useEditor();
  const doc = s.activeId ? s.documents[s.activeId] : null;
  if (s.tool !== "shape" || !doc || s.sheet !== null) return null;
  const o = s.shapeOptions;
  const disabled = s.working;
  const amount = (label: "Radius" | "Width", key: "cornerRadius" | "lineWidth", min: number, sliderMax: number, help: string) => (
    <label title={help}>{label}{" "}
      <input type="range" aria-hidden tabIndex={-1} min={min} max={sliderMax} step={1} value={Math.min(sliderMax, o[key])} disabled={disabled}
        onChange={(e) => s.setShapeOptions({ [key]: Math.round(Number(e.target.value)) })} onPointerUp={(e) => e.currentTarget.blur()} />
      <NumberInput label={label} testId={`shape-${key === "cornerRadius" ? "radius" : "width"}`} value={o[key]} min={min} max={SHAPE_FIELD_MAX} step={1} blurOnEnter disabled={disabled}
        onChange={(v) => s.setShapeOptions({ [key]: v })} /> px
    </label>
  );
  return (
    <div className="tool-options" data-testid="shape-options">
      <strong>Shape</strong>
      <span className="segmented" title="Shift+U (or Tab) steps through Rectangle, Ellipse and Line">
        {SHAPE_KINDS.map((k) => (
          <button key={k} data-testid={`shape-${k.toLowerCase()}`} aria-pressed={o.kind === k} disabled={disabled}
            onClick={(e) => { s.setShapeOptions({ kind: k }); e.currentTarget.blur(); }}>{k}</button>
        ))}
      </span>
      {o.kind === "Line" && amount("Width", "lineWidth", 1, 100, "The line's thickness")}
      {o.kind === "Rectangle" && amount("Radius", "cornerRadius", 0, 200, "Round the rectangle's corners by this many pixels; 0 keeps them square")}
      <label title="Shapes fill with the foreground color; click to change it">Fill{" "}
        <button className="color-swatch-button" data-testid="shape-fill" aria-label="Fill color" style={{ background: cssColor(s.palette.foreground) }} disabled={disabled}
          onClick={(e) => { e.currentTarget.blur(); s.openColorPicker({ kind: "palette", background: false }); }} />
      </label>
    </div>
  );
}
