import { useState } from "react";
import { useEditor } from "../state/store";

/** Joins phrases as prose: "a", "a and b", "a, b and c". */
function joinPhrases(items: string[]): string {
  if (items.length <= 1) return items.join("");
  if (items.length === 2) return `${items[0]} and ${items[1]}`;
  return `${items.slice(0, -1).join(", ")} and ${items[items.length - 1]}`;
}

/** Tells the user what the active document contains that this build does not draw yet
 * (`Document::undrawn`, surfaced as `DocumentState.undrawn`). Dismissing it only hides it for
 * that document handle for the rest of this session -- reopening or switching back to a
 * different document with its own undrawn features shows it again. */
export function UndrawnNotice() {
  const activeId = useEditor((s) => s.activeId);
  const undrawn = useEditor((s) => (s.activeId ? s.documents[s.activeId]?.undrawn : undefined));
  const [dismissed, setDismissed] = useState<Set<string>>(new Set());
  if (!activeId || !undrawn || undrawn.length === 0 || dismissed.has(activeId)) return null;
  return (
    <div className="undrawn-notice" data-testid="undrawn-notice" role="status">
      This project uses {joinPhrases(undrawn)}. Compositor for Windows does not draw these yet, so the canvas
      and exports show the project without them. They are kept exactly as they are when you save.
      <button onClick={() => setDismissed((prev) => new Set(prev).add(activeId))}>Dismiss</button>
    </div>
  );
}
