import { useState } from "react";
import { Sheet } from "./Sheet";
import { SELECTION_AMOUNT_MAX, useEditor, type SelectionAmountOperation } from "../state/store";

/** The whole number of pixels typed, when it lies in 1..max; null otherwise (`SelectionAmountSheet.amount`). */
export function parseAmount(text: string, max: number): number | null {
  const trimmed = text.trim();
  if (!/^\d+$/.test(trimmed)) return null;
  const value = Number(trimmed);
  return value >= 1 && value <= max ? value : null;
}

/** Select > Expand / Contract / Feather: a slider and a field for the amount, OK only with a whole
 * number in range (LassoControls.swift:180-236). */
export function SelectionAmountSheet(props: { operation: SelectionAmountOperation }) {
  const s = useEditor();
  const max = SELECTION_AMOUNT_MAX[props.operation];
  const key = props.operation === "Expand" ? "expand" : props.operation === "Contract" ? "contract" : "feather";
  const [input, setInput] = useState(String(s.selectionOptions[key]));
  const amount = parseAmount(input, max);
  return (
    <Sheet title={`${props.operation} Selection`} primary="OK" canConfirm={amount !== null} onCancel={s.closeSheet}
      onConfirm={() => { if (amount !== null) { s.closeSheet(); s.modifySelection(props.operation, amount); } }}>
      <label>Amount <input aria-label="Amount slider" type="range" min={1} max={max} step={1} value={amount ?? 1} onChange={(e) => setInput(e.target.value)} />
        <input aria-label="Amount" data-testid="selection-amount" autoFocus value={input} onChange={(e) => setInput(e.target.value)} /> px</label>
      {amount === null && <p className="hint">Enter a whole number from 1 to {max} px.</p>}
    </Sheet>
  );
}
