import { useEffect, useRef } from "react";
import { useEditor } from "../state/store";
import { LEVELS_CHANNELS, clampRange, histogramScale } from "../tools/levels-tools";
import type { LevelRange, LevelsAuto, LevelsChannel } from "../engine/types";

const FIELDS: [keyof LevelRange, string][] = [["black", "Black point"], ["gamma", "Gamma"], ["white", "White point"], ["outputBlack", "Output black"], ["outputWhite", "Output white"]];

export function LevelsPanel() {
  const s = useEditor();
  const edit = s.adjustEdit!;
  const settings = edit.adjustment!.levels;
  const channel = settings.channel;
  const index = LEVELS_CHANNELS.indexOf(channel);
  const range = settings.ranges[index];
  const canvas = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const element = canvas.current; const bins = edit.histogram?.[index]; if (!element || !bins) return;
    const ctx = element.getContext("2d")!;
    const scale = histogramScale(bins);
    ctx.clearRect(0, 0, element.width, element.height);
    ctx.fillStyle = "#8a8a8a";
    for (let i = 0; i < 256; i++) {
      const h = scale > 0 ? Math.min(1, bins[i] / scale) * element.height : 0;
      ctx.fillRect((i * element.width) / 256, element.height - h, Math.ceil(element.width / 256), h);
    }
  }, [edit.histogram, index]);
  const update = (patch: Partial<LevelRange>) => {
    const ranges = settings.ranges.map((r, i) => (i === index ? clampRange({ ...r, ...patch }) : r));
    s.updateAdjust({ adjustment: { ...edit.adjustment!, levels: { ...settings, ranges } } });
  };
  return (
    <>
      <label>Channel <select data-testid="levels-channel" value={channel} onChange={(e) => s.updateAdjust({ adjustment: { ...edit.adjustment!, levels: { ...settings, channel: e.target.value as LevelsChannel } } })}>
        {LEVELS_CHANNELS.map((c) => <option key={c} value={c}>{c}</option>)}
      </select></label>
      <canvas data-testid="levels-histogram" ref={canvas} width={256} height={100} className="histogram" />
      {FIELDS.map(([key, label]) => (
        <label key={key}>{label} <input aria-label={label} type="number" step={key === "gamma" ? 0.01 : 1} value={range[key]}
          onChange={(e) => { const v = Number(e.target.value); if (Number.isFinite(v)) update({ [key]: v } as Partial<LevelRange>); }} /></label>
      ))}
      {!edit.histogram && <span className="hint" data-testid="histogram-pending">Reading the histogram...</span>}
      <label>Auto <select data-testid="levels-auto" value="" disabled={!edit.histogram} onChange={(e) => { if (e.target.value) s.autoLevels(e.target.value as LevelsAuto); }}>
        <option value="">Choose...</option>
        <option value="Contrast">Contrast</option>
        <option value="Color">Color</option>
        <option value="Neutral">Color + neutral midtones</option>
      </select></label>
      <div className="eyedroppers">
        {(["Black", "Gray", "White"] as const).map((mode) => (
          <button key={mode} data-testid={`levels-sample-${mode.toLowerCase()}`} aria-pressed={edit.sampleMode === mode}
            onClick={() => s.setAdjustSample(edit.sampleMode === mode ? null : mode)}>{mode}</button>
        ))}
      </div>
    </>
  );
}
