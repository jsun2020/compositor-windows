import { useEditor, type Tool } from "../state/store";
const TOOLS: { id: Tool; label: string; key: string; glyph: string }[] = [
  { id: "move", label: "Move", key: "V", glyph: "M" }, { id: "marquee", label: "Marquee", key: "M", glyph: "[]" },
  { id: "lasso", label: "Lasso", key: "L", glyph: "L" }, { id: "wand", label: "Magic Wand", key: "W", glyph: "W" },
  { id: "crop", label: "Crop", key: "C", glyph: "C" }, { id: "hand", label: "Hand", key: "H", glyph: "H" }, { id: "zoom", label: "Zoom", key: "Z", glyph: "Z" },
];
export function ToolRail() {
  const tool = useEditor((s) => s.tool); const setTool = useEditor((s) => s.setTool);
  return (
    <div className="tool-rail">
      {TOOLS.map((t) => (
        // Blur immediately: a tool button that keeps keyboard focus after a mouse click
        // would make isEditableTarget treat every subsequent key (including the tool's
        // own single-letter shortcut and, for Move, the opacity digit keys) as typed
        // into a form control and swallow it.
        <button key={t.id} data-testid={`tool-${t.id}`} title={`${t.label} (${t.key})`} className={tool === t.id ? "active" : ""}
          onClick={(e) => { setTool(t.id); e.currentTarget.blur(); }}>{t.glyph}</button>
      ))}
    </div>
  );
}
