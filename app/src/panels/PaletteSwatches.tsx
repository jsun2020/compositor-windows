import { useEffect, useState } from "react";
import { useEditor } from "../state/store";
import { BLACK, WHITE, cssColor, type PaletteColor } from "../tools/color";

/** The palette at the foot of the tool rail (ColorPaletteControls.swift): the foreground swatch over
 * the background one, swap (X) at the top right, default colours (D) at the bottom left. A click on a
 * swatch opens the colour picker; while a mask is the target it asks black or white instead. Every
 * button gives up the focus once used, as the rail's tools do. */
export function PaletteSwatches() {
  const s = useEditor();
  const [choosing, setChoosing] = useState<"foreground" | "background" | null>(null);
  const masked = s.maskTargeted();
  // A change of target closes the black-or-white choice (ColorPaletteControls.swift:53-54).
  useEffect(() => setChoosing(null), [masked]);
  const enabled = !s.working;
  const swatch = (background: boolean) => {
    const label = background ? "Background color" : "Foreground color";
    return (
      <button className={`palette-swatch ${background ? "background" : "foreground"}`} data-testid={`palette-${background ? "background" : "foreground"}`}
        aria-label={label} title={label} disabled={!enabled} style={{ background: cssColor(s.paletteColor(background)) }}
        onClick={(e) => {
          e.currentTarget.blur();
          if (masked) setChoosing(background ? "background" : "foreground");
          else s.openColorPicker({ kind: "palette", background });
        }} />
    );
  };
  const choose = (color: PaletteColor) => { if (choosing) s.setPaletteColor(color, choosing === "background"); setChoosing(null); };
  return (
    <div className="palette" data-testid="palette">
      {swatch(true)}
      {swatch(false)}
      <button className="palette-swap" data-testid="palette-swap" aria-label="Swap colors" title="Swap foreground and background (X)" disabled={!enabled}
        onClick={(e) => { e.currentTarget.blur(); s.swapPalette(); }}>
        <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          <path d="M3 9.5 9.5 3" /><path d="M6.5 3h3v3" /><path d="M3 6v3.5h3.5" />
        </svg>
      </button>
      <button className="palette-reset" data-testid="palette-reset" aria-label="Default colors" title="Default colors (D)" disabled={!enabled}
        onClick={(e) => { e.currentTarget.blur(); s.resetPalette(); }}>
        <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
          <rect x="1" y="1" width="6" height="6" fill="#000" stroke="#ccc" strokeWidth="0.8" /><rect x="5" y="5" width="6" height="6" fill="#fff" stroke="#ccc" strokeWidth="0.8" />
        </svg>
      </button>
      {choosing && masked && (
        <div className="palette-mask-popover" data-testid="palette-mask-popover" role="dialog" aria-label={choosing === "background" ? "Mask background" : "Mask foreground"}>
          <strong>{choosing === "background" ? "Mask background" : "Mask foreground"}</strong>
          <div>
            <button data-testid="mask-black" onClick={() => choose(BLACK)}>Black - Hide</button>
            <button data-testid="mask-white" onClick={() => choose(WHITE)}>White - Reveal</button>
          </div>
        </div>
      )}
    </div>
  );
}
