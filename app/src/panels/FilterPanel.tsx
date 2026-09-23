import { useEditor } from "../state/store";
import type { AdjustmentColor, ExposureSettings, FilterParams, GrainSettings } from "../engine/types";
import { NumberInput } from "./NumberInput";

// A plain <span> caption, not a <label>, wraps the pair: a <label> would give the range
// input the same accessible name as the number input (Chromium keeps a focusable element
// in the accessibility tree even under aria-hidden, per the ARIA spec's rule against hiding
// focusable content, so aria-hidden alone does not stop it inheriting the wrapping label's
// name). getByLabel("Radius") then resolved two elements instead of one. Kept out of any
// label and given no name of its own, the slider is invisible to getByLabel either way.
function NumberField(props: { label: string; value: number; min: number; max: number; step: number; onChange(v: number): void }) {
  return (
    <span className="number-field">
      <span className="number-field-label">{props.label}</span>
      <input type="range" aria-hidden tabIndex={-1} min={props.min} max={props.max} step={props.step} value={props.value}
        onChange={(e) => props.onChange(Number(e.target.value))} />
      <NumberInput label={props.label} value={props.value} min={props.min} max={props.max} step={props.step} onChange={props.onChange} />
    </span>
  );
}
const hex = (c: AdjustmentColor) => "#" + [c.red, c.green, c.blue].map((v) => Math.round(v * 255).toString(16).padStart(2, "0")).join("");
const fromHex = (text: string): AdjustmentColor => ({ red: parseInt(text.slice(1, 3), 16) / 255, green: parseInt(text.slice(3, 5), 16) / 255, blue: parseInt(text.slice(5, 7), 16) / 255 });

export function FilterPanel() {
  const s = useEditor();
  const edit = s.adjustEdit!;
  if (edit.params) {
    const p = edit.params;
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
    return (<>
      <label>Shadows <input aria-label="Shadows" type="color" value={hex(g.shadows)} onChange={(e) => set({ shadows: fromHex(e.target.value) })} /></label>
      <label>Highlights <input aria-label="Highlights" type="color" value={hex(g.highlights)} onChange={(e) => set({ highlights: fromHex(e.target.value) })} /></label>
      <label><input type="checkbox" aria-label="Reverse" checked={g.reversed} onChange={(e) => set({ reversed: e.target.checked })} /> Reverse</label>
    </>);
  }
  const grain: GrainSettings = a.grainSettings ?? { amount: 25, size: 1.5, roughness: 50, seed: 0 };
  const set = (patch: Partial<GrainSettings>) => setAdjustment({ grainSettings: { ...grain, ...patch } });
  return (<>
    <NumberField label="Amount" value={grain.amount} min={0} max={100} step={1} onChange={(amount) => set({ amount })} />
    <NumberField label="Size" value={grain.size} min={0.5} max={20} step={0.1} onChange={(size) => set({ size })} />
    <NumberField label="Roughness" value={grain.roughness} min={0} max={100} step={1} onChange={(roughness) => set({ roughness })} />
  </>);
}
