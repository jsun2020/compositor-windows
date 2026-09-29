import { useEditor, type Tool } from "../state/store";
import { ToolIcon, toolIconName } from "./tool-icons";
import { PaletteSwatches } from "./PaletteSwatches";

const TOOLS: { id: Tool; label: string; key: string }[] = [
  { id: "move", label: "Move", key: "V" }, { id: "marquee", label: "Marquee", key: "M" },
  { id: "lasso", label: "Lasso", key: "L" }, { id: "wand", label: "Magic Wand", key: "W" },
  { id: "crop", label: "Crop", key: "C" }, { id: "eyedropper", label: "Eyedropper", key: "I" },
  { id: "hand", label: "Hand", key: "H" }, { id: "zoom", label: "Zoom", key: "Z" },
];
export function ToolRail() {
  const tool = useEditor((s) => s.tool); const setTool = useEditor((s) => s.setTool);
  const marquee = useEditor((s) => s.selectionOptions.marquee); const lasso = useEditor((s) => s.selectionOptions.lasso);
  return (
    <div className="tool-rail">
      {TOOLS.map((t) => (
        // Blur immediately: a tool button that keeps keyboard focus after a mouse click
        // would make isEditableTarget treat every subsequent key (including the tool's
        // own single-letter shortcut and, for Move, the opacity digit keys) as typed
        // into a form control and swallow it.
        <button key={t.id} data-testid={`tool-${t.id}`} aria-label={t.label} title={`${t.label} (${t.key})`}
          className={tool === t.id ? "active" : ""} onClick={(e) => { setTool(t.id); e.currentTarget.blur(); }}>
          <ToolIcon name={toolIconName(t.id, marquee, lasso)} />
        </button>
      ))}
      <PaletteSwatches />
    </div>
  );
}
