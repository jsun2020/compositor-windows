import { useEditor } from "../state/store";
import { ALL_RANGES, DEFAULT_BANDS, colorizeStart, defaultHsv, setHandle } from "../tools/hue-band";
import type { ColorRange, HueBand, HueSaturationSettings, RangeAdjustment } from "../engine/types";
import { NumberInput } from "./NumberInput";

const IDENTITY_ADJUSTMENT: RangeAdjustment = { hue: 0, saturation: 0, lightness: 0 };
const HANDLE_LABELS = ["Falloff start", "Range start", "Range end", "Falloff end"];

export function HueSaturationPanel() {
  const s = useEditor();
  const edit = s.adjustEdit!;
  const settings = edit.adjustment!.hsvSettings ?? defaultHsv();
  const range = settings.range;
  const adjustment = settings.adjustments[range] ?? IDENTITY_ADJUSTMENT;
  const band = settings.bands[range] ?? DEFAULT_BANDS[range];
  // Colour ranges other than Master act through their band; Master applies everywhere, so its
  // handles have nothing to edit and while Colorize is on the whole notion of a band is moot.
  const bandDisabled = range === "Master" || settings.colorize;

  const write = (next: HueSaturationSettings) => s.updateAdjust({ adjustment: { ...edit.adjustment!, hsvSettings: next } });
  const updateField = (field: keyof RangeAdjustment, value: number) => {
    write({ ...settings, adjustments: { ...settings.adjustments, [range]: { ...adjustment, [field]: value } } });
  };
  const updateHandle = (index: number, degrees: number) => {
    write({ ...settings, bands: { ...settings.bands, [range]: setHandle(band, index, degrees) } });
  };

  const hueRange: [number, number] = settings.colorize ? [0, 360] : [-180, 180];
  const saturationRange: [number, number] = settings.colorize ? [0, 100] : [-100, 100];

  return (
    <>
      <label>Range <select data-testid="hue-range" value={range} disabled={settings.colorize}
        onChange={(e) => write({ ...settings, range: e.target.value as ColorRange })}>
        {ALL_RANGES.map((r) => <option key={r} value={r}>{r}</option>)}
      </select></label>
      <label>Hue <NumberInput label="Hue" min={hueRange[0]} max={hueRange[1]} value={adjustment.hue} onChange={(v) => updateField("hue", v)} /></label>
      <label>Saturation <NumberInput label="Saturation" min={saturationRange[0]} max={saturationRange[1]} value={adjustment.saturation} onChange={(v) => updateField("saturation", v)} /></label>
      <label>Lightness <NumberInput label="Lightness" min={-100} max={100} value={adjustment.lightness} onChange={(v) => updateField("lightness", v)} /></label>
      <label><input type="checkbox" data-testid="hue-colorize" checked={settings.colorize}
        onChange={(e) => write(e.target.checked ? colorizeStart() : defaultHsv())} /> Colorize</label>
      <label><input type="checkbox" data-testid="hue-invert" checked={settings.invertRange} disabled={range === "Master"}
        onChange={(e) => write({ ...settings, invertRange: e.target.checked })} /> Apply outside this range instead</label>
      <div className="eyedroppers">
        {(["replace", "add", "remove"] as const).map((mode) => (
          <button key={mode} data-testid={`hue-sample-${mode}`} aria-pressed={edit.sampleMode === mode} disabled={bandDisabled}
            onClick={() => s.setAdjustSample(edit.sampleMode === mode ? null : mode)}>{mode}</button>
        ))}
      </div>
      <div className="hue-bands">
        {(["falloffStart", "rangeStart", "rangeEnd", "falloffEnd"] as (keyof HueBand)[]).map((key, i) => (
          <label key={key}>{HANDLE_LABELS[i]} <input aria-label={HANDLE_LABELS[i]} data-testid={`hue-band-${i}`} type="number" value={band[key]} disabled={bandDisabled}
            onChange={(e) => { const v = Number(e.target.value); if (Number.isFinite(v)) updateHandle(i, v); }} /></label>
        ))}
      </div>
    </>
  );
}
