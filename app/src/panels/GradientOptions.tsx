import { useEditor } from "../state/store";
import { GRADIENT_STYLES, gradientStops, type GradientStyle } from "../state/gradient-edit";
import { NumberInput } from "./NumberInput";

/** The Gradient tool's bar (GradientControls.swift): Linear or Radial, the colours over a checkerboard,
 * the style, Reverse, Opacity (1-100 %, also the digit keys), "Mask" while a mask is the target, and
 * Cancel / Apply while a gradient is pending. Every control gives up the focus once used, as the
 * selection bar's do, so Return, Escape and the digits reach the canvas. */
export function GradientOptions() {
  const s = useEditor();
  const doc = s.activeId ? s.documents[s.activeId] : null;
  if (s.tool !== "gradient" || !doc || s.sheet !== null) return null;
  const o = s.gradientOptions;
  // The palette's black or white while a mask is the target, as the swatch on the Mac shows
  // (GradientControls.swift:43, ColorPalette.swift:26-28; fix round 1, I-2).
  const [from, to] = gradientStops(o, s.paletteColor(false), s.paletteColor(true));
  const css = (c: number[]) => `rgba(${Math.round(c[0] * 255)}, ${Math.round(c[1] * 255)}, ${Math.round(c[2] * 255)}, ${c[3]})`;
  const disabled = s.working;
  return (
    <div className="tool-options" data-testid="gradient-options">
      <strong>Gradient</strong>
      <span className="segmented" title="Linear runs along the line; Radial spreads out from the start point">
        {(["Linear", "Radial"] as const).map((k) => (
          <button key={k} data-testid={`gradient-${k.toLowerCase()}`} aria-pressed={o.shape === k} disabled={disabled}
            onClick={(e) => { s.setGradientOptions({ shape: k }); e.currentTarget.blur(); }}>{k}</button>
        ))}
      </span>
      <span className="gradient-swatch" data-testid="gradient-swatch" aria-hidden="true" style={{ backgroundImage: `linear-gradient(to right, ${css(from)}, ${css(to)}), repeating-conic-gradient(rgba(128,128,128,0.45) 0% 25%, #fff 0% 50%)`, backgroundSize: "auto, 8px 8px" }} />
      <select aria-label="Colors" data-testid="gradient-style" value={o.style} disabled={disabled}
        onChange={(e) => { s.setGradientOptions({ style: e.target.value as GradientStyle }); e.currentTarget.blur(); }}>
        {GRADIENT_STYLES.map((style) => <option key={style}>{style}</option>)}
      </select>
      <label><input type="checkbox" data-testid="gradient-reverse" checked={o.reversed} disabled={disabled}
        onChange={(e) => { s.setGradientOptions({ reversed: e.target.checked }); e.currentTarget.blur(); }} /> Reverse</label>
      <label title="Press 1-9 for 10-90%, 0 for 100%">Opacity{" "}
        {/* A reduced preview on every tick, settled once the pointer lets go (fix round 1, M-1: the
          * settled 2048 px preview cost 80-95 ms on every input event). */}
        <input type="range" aria-hidden tabIndex={-1} min={1} max={100} step={1} value={Math.round(o.opacity * 100)} disabled={disabled}
          onChange={(e) => s.setGradientOptions({ opacity: Number(e.target.value) / 100 }, true)}
          onPointerUp={(e) => { s.refreshGradient(); e.currentTarget.blur(); }} />
        <NumberInput label="Opacity" testId="gradient-opacity" value={Math.round(o.opacity * 100)} min={1} max={100} step={1} blurOnEnter disabled={disabled}
          onChange={(v) => s.setGradientOptions({ opacity: Math.round(v) / 100 })} />%
      </label>
      <span style={{ flex: 1 }} />
      {s.maskTargeted() && <span className="muted">Mask</span>}
      {s.gradientEdit && (
        <>
          <button data-testid="gradient-cancel" onClick={(e) => { e.currentTarget.blur(); s.cancelGradient(); }}>Cancel</button>
          <button data-testid="gradient-apply" className="primary" onClick={(e) => { e.currentTarget.blur(); s.commitGradient(); }}>Apply</button>
        </>
      )}
    </div>
  );
}
