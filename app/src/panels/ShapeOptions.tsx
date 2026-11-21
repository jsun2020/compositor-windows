import { useEditor } from "../state/store";
import { SHAPE_FIELD_MAX, SHAPE_KINDS } from "../tools/shape-draft";
import { cssColor, colorOf, colorTuple, hexOf, parseHex } from "../tools/color";
import { NumberInput } from "./NumberInput";

/** The Shape tool's bar (ShapeControls.swift): Rectangle, Ellipse or Line; a rectangle's Radius or a
 * line's Width (sliders to 200 and 100, fields to 5000 px); and the Fill swatch, the foreground colour,
 * which opens the picker. Controls give up the focus once used, as the other bars' do. */
export function ShapeOptions() {
  const s = useEditor();
  const doc = s.activeId ? s.documents[s.activeId] : null;
  if (s.tool !== "shape" || !doc || s.sheet !== null) return null;
  const o = s.shapeOptions;
  const layer = doc.layers.find(l => l.id === doc.activeLayerId);
  const record = s.selectedLayerIds.length === 1 && layer?.shape?.kind === o.kind ? layer.shape : undefined;
  const stroke = record ? {
    width: Number(record.strokeWidth ?? 0),
    color: [Number(record.strokeRed ?? o.stroke?.color[0] ?? 0),Number(record.strokeGreen ?? o.stroke?.color[1] ?? 0),Number(record.strokeBlue ?? o.stroke?.color[2] ?? 0)] as [number,number,number],
  } : o.stroke ?? {width:0, color:[0,0,0] as [number,number,number]};
  const changeStroke = (patch: Partial<typeof stroke>) => {
    const next = {...stroke,...patch};
    if (record && layer && (next.width > 0 || stroke.width > 0)) s.run({type:"SetShapeAppearance",id:layer.id,stroke:next});
    s.setShapeOptions({stroke:next});
  };
  const changeAmount = (key: "cornerRadius"|"lineWidth", value: number) => {
    if (record && layer) s.run({type:"SetShapeAppearance",id:layer.id,[key]:value});
    s.setShapeOptions({[key]:value});
  };
  const changeStyle = (patch: Pick<typeof o, "lineStyle" | "lineCap" | "smoothing">) => {
    if (record && layer) s.run({type:"SetShapeAppearance", id:layer.id, ...patch});
    s.setShapeOptions(patch);
  };
  const disabled = s.working;
  const amount = (label: "Radius" | "Width", key: "cornerRadius" | "lineWidth", min: number, sliderMax: number, help: string) => (
    <label title={help}>{label}{" "}
      <input type="range" aria-hidden tabIndex={-1} min={min} max={sliderMax} step={1} value={Math.min(sliderMax, Number(record?.[key] ?? o[key]))} disabled={disabled}
        onChange={(e) => changeAmount(key, Math.round(Number(e.target.value)))} onPointerUp={(e) => e.currentTarget.blur()} />
      <NumberInput label={label} testId={`shape-${key === "cornerRadius" ? "radius" : "width"}`} value={Number(record?.[key] ?? o[key])} min={min} max={SHAPE_FIELD_MAX} step={1} blurOnEnter disabled={disabled}
        onChange={(v) => changeAmount(key, v)} /> px
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
      {o.kind === "Rectangle" && <label><input type="checkbox" checked={record ? record.smoothing === true : o.smoothing ?? false} disabled={disabled} onChange={e=>changeStyle({smoothing:e.target.checked})}/> Smooth Corners</label>}
      {o.kind === "Line" && <>
        <label>Line Style <select aria-label="Line Style" value={String(record ? record.lineStyle ?? "Solid" : o.lineStyle ?? "Solid")} disabled={disabled} onChange={e=>changeStyle({lineStyle:e.target.value as typeof o.lineStyle})}>{["Solid","Dotted","Dashed","Long Dash"].map(v=><option key={v}>{v}</option>)}</select></label>
        <label>Line Caps <select aria-label="Line Caps" value={String(record ? record.lineCap ?? "Round" : o.lineCap ?? "Square")} disabled={disabled} onChange={e=>changeStyle({lineCap:e.target.value as typeof o.lineCap})}><option>Square</option><option>Round</option></select></label>
      </>}
      {o.kind !== "Line" && <>
        <NumberInput label="Inside stroke" value={stroke.width} min={0} max={SHAPE_FIELD_MAX} step={1} blurOnEnter disabled={disabled} onChange={width => changeStroke({width})}/>
        <label>Stroke <input type="color" aria-label="Stroke color" disabled={disabled} value={"#"+hexOf(colorOf(stroke.color))}
          onChange={e=>{const c=parseHex(e.target.value); if(c) changeStroke({color:colorTuple(c)});}} /></label>
      </>}
      {record && layer && <label>Selected fill <input type="color" aria-label="Selected shape fill" disabled={disabled}
        value={"#"+hexOf({red:Number(record.red),green:Number(record.green),blue:Number(record.blue)})}
        onChange={e=>{const c=parseHex(e.target.value); if(c) s.run({type:"SetShapeAppearance",id:layer.id,fill:colorTuple(c)});}} /></label>}
      <label title="Shapes fill with the foreground color; click to change it">Fill{" "}
        <button className="color-swatch-button" data-testid="shape-fill" aria-label="Fill color" style={{ background: cssColor(s.palette.foreground) }} disabled={disabled}
          onClick={(e) => { e.currentTarget.blur(); s.openColorPicker({ kind: "palette", background: false }); }} />
      </label>
    </div>
  );
}
