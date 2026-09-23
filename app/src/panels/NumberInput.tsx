import { useState } from "react";

/** A number field that never hands on a value outside `min`..`max`: every edit is clamped before
 * it reaches `onChange`, so the preview and OK only ever see settings the engine accepts
 * (`LayerAdjustment::is_valid` refuses the rest, and a refused OK used to lose the edit). The
 * `min`/`max` attributes alone constrain the spinner, not typing.
 *
 * While the field has focus it shows what was typed, so a partial entry such as "0." is not
 * snapped under the caret; Enter or leaving the field shows the clamped value actually in use.
 * The typed text is kept only while the value is still the one it produced, so a value changed
 * from outside (another range selected, Reset) shows at once. */
export function NumberInput(props: { label: string; value: number; min: number; max: number; step?: number; disabled?: boolean; testId?: string; onChange(v: number): void }) {
  const [draft, setDraft] = useState<{ text: string; value: number } | null>(null);
  return (
    <input aria-label={props.label} data-testid={props.testId} type="number" min={props.min} max={props.max} step={props.step} disabled={props.disabled}
      value={draft && draft.value === props.value ? draft.text : props.value}
      onChange={(e) => {
        const text = e.target.value;
        const v = Number(text);
        const value = text !== "" && Number.isFinite(v) ? Math.min(props.max, Math.max(props.min, v)) : props.value;
        setDraft({ text, value });
        if (value !== props.value) props.onChange(value);
      }}
      onBlur={() => setDraft(null)}
      onKeyDown={(e) => { if (e.key === "Enter") setDraft(null); }} />
  );
}
