import { useEditor, type Tool } from "../state/store";
const TOOLS: { id: Tool; label: string; key: string }[] = [
  { id: "move", label: "Move", key: "V" }, { id: "hand", label: "Hand", key: "H" }, { id: "zoom", label: "Zoom", key: "Z" }, { id: "crop", label: "Crop", key: "C" },
];
export function ToolRail() {
  const tool = useEditor((s) => s.tool); const setTool = useEditor((s) => s.setTool);
  return (
    <div className="tool-rail">
      {TOOLS.map((t) => <button key={t.id} data-testid={`tool-${t.id}`} title={`${t.label} (${t.key})`} className={tool === t.id ? "active" : ""} onClick={() => setTool(t.id)}>{t.label[0]}</button>)}
    </div>
  );
}
