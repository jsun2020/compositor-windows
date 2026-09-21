import { useEditor, type Tool } from "../state/store";
const TOOLS: { id: Tool; label: string; key: string }[] = [
  { id: "move", label: "Move", key: "V" }, { id: "hand", label: "Hand", key: "H" }, { id: "zoom", label: "Zoom", key: "Z" }, { id: "crop", label: "Crop", key: "C" },
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
          onClick={(e) => { setTool(t.id); e.currentTarget.blur(); }}>{t.label[0]}</button>
      ))}
    </div>
  );
}
