import { useEditor } from "../state/store";
import type { AdjustmentColor, BlackWhiteSettings, ColorBalanceSettings, ExposureSettings, FilterParams, GrainSettings } from "../engine/types";
import { DEFAULT_BLACK_WHITE, DEFAULT_COLOR_BALANCE } from "../engine/types";
import { NumberInput } from "./NumberInput";
import { CameraRawPanel } from "./CameraRawPanel";

// A plain <span> caption, not a <label>, wraps the pair: a <label> would give the range
// input the same accessible name as the number input (Chromium keeps a focusable element
// in the accessibility tree even under aria-hidden, per the ARIA spec's rule against hiding
// focusable content, so aria-hidden alone does not stop it inheriting the wrapping label's
// name). getByLabel("Radius") then resolved two elements instead of one. Kept out of any
// label and given no name of its own, the slider is invisible to getByLabel either way.
function NumberField(props: { label: string; name?: string; value: number; min: number; max: number; step: number; onChange(v: number): void }) {
  return (
    <span className="number-field">
      <span className="number-field-label">{props.label}</span>
      <input type="range" aria-hidden tabIndex={-1} min={props.min} max={props.max} step={props.step} value={props.value}
        onChange={(e) => props.onChange(Number(e.target.value))} />
      <NumberInput label={props.name ?? props.label} value={props.value} min={props.min} max={props.max} step={props.step} onChange={props.onChange} />
    </span>
  );
}
const hex = (c: AdjustmentColor) => "#" + [c.red, c.green, c.blue].map((v) => Math.round(v * 255).toString(16).padStart(2, "0")).join("");

export function FilterPanel() {
  const s = useEditor();
  const edit = s.adjustEdit!;
  if (edit.params) {
    const p = edit.params;
    if (p.filter === "CameraRaw") return <CameraRawPanel />;
    const set = (patch: Partial<FilterParams>) => s.updateAdjust({ params: { ...p, ...patch } as FilterParams });
    if (p.filter === "GaussianBlur") return <NumberField label="Radius" value={p.radius} min={0.1} max={250} step={0.1} onChange={(radius) => set({ radius } as never)} />;
    if (p.filter === "MotionBlur") return (<>
      <NumberField label="Angle" value={p.angle} min={-90} max={90} step={1} onChange={(angle) => set({ angle } as never)} />
      <NumberField label="Distance" value={p.distance} min={1} max={2000} step={1} onChange={(distance) => set({ distance } as never)} />
    </>);
    if (p.filter === "AddNoise") return (<>
      <NumberField label="Amount" value={p.amount} min={0.1} max={400} step={0.1} onChange={(amount) => set({ amount } as never)} />
      <label>Distribution <select aria-label="Distribution" value={p.gaussian ? "Gaussian" : "Uniform"} onChange={(e) => set({ gaussian: e.target.value === "Gaussian" } as never)}>
        <option>Uniform</option><option>Gaussian</option></select></label>
      <label><input type="checkbox" aria-label="Monochromatic" checked={p.monochromatic} onChange={(e) => set({ monochromatic: e.target.checked } as never)} /> Monochromatic</label>
    </>);
    return <NumberField label="Remove distortion" value={p.distortion} min={-100} max={100} step={1} onChange={(distortion) => set({ distortion } as never)} />;
  }
  const a = edit.adjustment!;
  const setAdjustment = (patch: Partial<typeof a>) => s.updateAdjust({ adjustment: { ...a, ...patch } });
  if (a.kind === "Exposure") {
    const e: ExposureSettings = a.exposureSettings ?? { exposure: 0, offset: 0, gamma: 1 };
    const set = (patch: Partial<ExposureSettings>) => setAdjustment({ exposureSettings: { ...e, ...patch } });
    return (<>
      <NumberField label="Exposure" value={e.exposure} min={-20} max={20} step={0.01} onChange={(exposure) => set({ exposure })} />
      <NumberField label="Offset" value={e.offset} min={-0.5} max={0.5} step={0.001} onChange={(offset) => set({ offset })} />
      <NumberField label="Gamma" value={e.gamma} min={0.01} max={9.99} step={0.01} onChange={(gamma) => set({ gamma })} />
    </>);
  }
  if (a.kind === "Gradient Map") {
    const g = a.gradientMapSettings ?? { shadows: { red: 0, green: 0, blue: 0 }, highlights: { red: 1, green: 1, blue: 1 }, reversed: false };
    const set = (patch: Partial<typeof g>) => setAdjustment({ gradientMapSettings: { ...g, ...patch } });
    // Each end opens the app's colour picker, which previews as it changes (`openGradientMapColorPicker`).
    const end = (label: "Shadows" | "Highlights", c: AdjustmentColor) => (
      <label>{label} <button aria-label={label} data-testid={`gradient-map-${label.toLowerCase()}`} className="color-swatch-button" style={{ background: hex(c) }}
        disabled={!!s.colorPicker} onClick={(e) => { e.currentTarget.blur(); s.openColorPicker({ kind: "gradientMap", highlights: label === "Highlights" }); }} /></label>
    );
    return (<>
      {end("Shadows", g.shadows)}
      {end("Highlights", g.highlights)}
      <label><input type="checkbox" aria-label="Reverse" checked={g.reversed} onChange={(e) => set({ reversed: e.target.checked })} /> Reverse</label>
    </>);
  }
  // FilterSheet.swift:28-41: six weights (BlackWhiteSettings.range, whole percents), Tint, and the
  // tint's Hue and Saturation only while Tint is on.
  if (a.kind === "Black & White") {
    const bw: BlackWhiteSettings = a.blackWhiteSettings ?? DEFAULT_BLACK_WHITE;
    const set = (patch: Partial<BlackWhiteSettings>) => setAdjustment({ blackWhiteSettings: { ...bw, ...patch } });
    const weights = [["Reds", "reds"], ["Yellows", "yellows"], ["Greens", "greens"], ["Cyans", "cyans"], ["Blues", "blues"], ["Magentas", "magentas"]] as const;
    return (<>
      {weights.map(([label, key]) => <NumberField key={key} label={label} value={bw[key]} min={-200} max={300} step={1} onChange={(v) => set({ [key]: v } as never)} />)}
      <label><input type="checkbox" aria-label="Tint" checked={bw.tint} onChange={(e) => set({ tint: e.target.checked })} /> Tint</label>
      {bw.tint && <>
        <NumberField label="Hue" value={bw.tintHue} min={0} max={360} step={1} onChange={(tintHue) => set({ tintHue })} />
        <NumberField label="Saturation" value={bw.tintSaturation} min={0} max={100} step={1} onChange={(tintSaturation) => set({ tintSaturation })} />
      </>}
    </>);
  }
  // FilterSheet.swift:45-59: three rows per tone. The captions repeat across tones, so each field's
  // accessible name carries its tone.
  if (a.kind === "Color Balance") {
    const cb: ColorBalanceSettings = a.colorBalanceSettings ?? DEFAULT_COLOR_BALANCE;
    const set = (patch: Partial<ColorBalanceSettings>) => setAdjustment({ colorBalanceSettings: { ...cb, ...patch } });
    const tone = (title: string, prefix: "shadow" | "mid" | "highlight") => (
      <div key={prefix}>
        <div>{title}</div>
        {([["Cyan / Red", "CyanRed"], ["Magenta / Green", "MagentaGreen"], ["Yellow / Blue", "YellowBlue"]] as const).map(([label, suffix]) => {
          const key = `${prefix}${suffix}` as const;
          return <NumberField key={key} label={label} name={`${title} ${label}`} value={cb[key]} min={-100} max={100} step={1} onChange={(v) => set({ [key]: v } as never)} />;
        })}
      </div>
    );
    return (<>
      {tone("Shadows", "shadow")}{tone("Midtones", "mid")}{tone("Highlights", "highlight")}
      <label><input type="checkbox" aria-label="Preserve Luminosity" checked={cb.preserveLuminosity} onChange={(e) => set({ preserveLuminosity: e.target.checked })} /> Preserve Luminosity</label>
    </>);
  }
  // The filters' own controls (FilterSheet.swift:84-96) on the layer's flat keys; absent keys show
  // the Mac's resolved defaults (LayerAdjustment.swift:92-119).
  if (a.kind === "Gaussian Blur") return <NumberField label="Radius" value={a.blurRadius ?? 10} min={0.1} max={250} step={0.1} onChange={(blurRadius) => setAdjustment({ blurRadius })} />;
  if (a.kind === "Motion Blur") return (<>
    <NumberField label="Angle" value={a.motionAngle ?? 0} min={-90} max={90} step={1} onChange={(motionAngle) => setAdjustment({ motionAngle })} />
    <NumberField label="Distance" value={a.motionDistance ?? 10} min={1} max={2000} step={1} onChange={(motionDistance) => setAdjustment({ motionDistance })} />
  </>);
  if (a.kind === "Add Noise") return (<>
    <NumberField label="Amount" value={a.noiseAmount ?? 10} min={0.1} max={400} step={0.1} onChange={(noiseAmount) => setAdjustment({ noiseAmount })} />
    <label>Distribution <select aria-label="Distribution" value={a.noiseGaussian ? "Gaussian" : "Uniform"} onChange={(e) => setAdjustment({ noiseGaussian: e.target.value === "Gaussian" })}>
      <option>Uniform</option><option>Gaussian</option></select></label>
    <label><input type="checkbox" aria-label="Monochromatic" checked={a.noiseMonochromatic ?? false} onChange={(e) => setAdjustment({ noiseMonochromatic: e.target.checked })} /> Monochromatic</label>
  </>);
  const grain: GrainSettings = a.grainSettings ?? { amount: 25, size: 1.5, roughness: 50, seed: 0 };
  const set = (patch: Partial<GrainSettings>) => setAdjustment({ grainSettings: { ...grain, ...patch } });
  return (<>
    <NumberField label="Amount" value={grain.amount} min={0} max={100} step={1} onChange={(amount) => set({ amount })} />
    <NumberField label="Size" value={grain.size} min={0.5} max={20} step={0.1} onChange={(size) => set({ size })} />
    <NumberField label="Roughness" value={grain.roughness} min={0} max={100} step={1} onChange={(roughness) => set({ roughness })} />
  </>);
}
