import { useEditor } from "../state/store";
import { closeProject } from "../actions/files";

const MODIFIED_DOT = " " + String.fromCharCode(0x2022);

export function ProjectTabs() {
  const s = useEditor();
  return (
    <div className="tabs">
      {s.order.map((id) => {
        const d = s.documents[id];
        const title = d.path ? s.bridge!.baseName(d.path) : "Untitled";
        return (
          <div key={id} data-testid="project-tab" className={"tab" + (id === s.activeId ? " active" : "")} onClick={() => s.setActive(id)}>
            <span>{title}{d.isModified ? MODIFIED_DOT : ""}</span>
            <button aria-label={`Close ${title}`} onClick={(e) => { e.stopPropagation(); void closeProject(id); }}>x</button>
          </div>
        );
      })}
    </div>
  );
}
