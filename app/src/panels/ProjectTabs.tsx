import { useRef, useState } from "react";
import { useEditor } from "../state/store";
import { closeProject } from "../actions/files";
import { TAB_DRAG_THRESHOLD, commitTarget, dragged, makeReorderState, renderX, type TabReorderState, type TabSlot } from "../state/tab-reorder";

const MODIFIED_DOT = " " + String.fromCharCode(0x2022);

/** A press on a tab, until it is let go: where it started and the strip as it was laid out then. */
interface Press { id: string; pointerId: number; startX: number; layout: TabSlot[]; spacing: number; contentWidth: number; }

/** The Mac's `workspace.canSwitch` as far as a reorder needs it: no job's result to come and no project
 * operation running (ProjectWorkspace.swift:27-31 at v1.4.5). */
function canReorder(): boolean { const s = useEditor.getState(); return !s.working && !s.busy; }

export function ProjectTabs() {
  const s = useEditor();
  const strip = useRef<HTMLDivElement>(null);
  const press = useRef<Press | null>(null);
  // Mirrored in a ref so the pointer handlers, which run between renders, read the latest state.
  const reorderRef = useRef<TabReorderState | null>(null);
  const [reorder, setReorderState] = useState<TabReorderState | null>(null);
  const setReorder = (r: TabReorderState | null) => { reorderRef.current = r; setReorderState(r); };

  const layout = (): { slots: TabSlot[]; spacing: number; contentWidth: number } => {
    const tabs = Array.from(strip.current?.querySelectorAll<HTMLElement>('[data-testid="project-tab"]') ?? []);
    const slots = tabs.map((el) => ({ id: el.dataset.docId!, x: el.offsetLeft, width: el.offsetWidth }));
    const spacing = slots.length > 1 ? Math.max(0, slots[1].x - (slots[0].x + slots[0].width)) : 0;
    const last = slots[slots.length - 1];
    return { slots, spacing, contentWidth: last ? last.x + last.width : 0 };
  };
  const down = (e: React.PointerEvent, id: string) => {
    if (e.button !== 0 || (e.target as HTMLElement).closest("button")) return;
    const { slots, spacing, contentWidth } = layout();
    press.current = { id, pointerId: e.pointerId, startX: e.clientX, layout: slots, spacing, contentWidth };
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  };
  const move = (e: React.PointerEvent) => {
    const p = press.current;
    if (!p || e.pointerId !== p.pointerId) return;
    const translation = e.clientX - p.startX;
    let r = reorderRef.current;
    if (!r) {
      // ProjectTabs.swift:160-164: a press that moves 3 px becomes a drag, and the drag selects its tab.
      if (!canReorder() || Math.abs(translation) < TAB_DRAG_THRESHOLD) return;
      useEditor.getState().setActive(p.id);
      r = makeReorderState(p.id, p.layout, p.spacing);
    }
    if (r.id !== p.id) return;
    setReorder(dragged(r, translation));
  };
  const up = (e: React.PointerEvent) => {
    const p = press.current;
    if (!p || e.pointerId !== p.pointerId) return;
    const r = reorderRef.current;
    // Busy by the time the drag ends: the tabs go back where they were (:169-171).
    if (r && r.id === p.id && canReorder()) {
      const st = useEditor.getState();
      const target = commitTarget(st.order, r);
      if (target !== null) st.moveTab(r.id, target);
    }
    press.current = null;
    setReorder(null);
  };
  const cancel = () => { press.current = null; setReorder(null); };

  const contentWidth = press.current?.contentWidth ?? 0;
  return (
    <div className="tabs" ref={strip}>
      {s.order.map((id) => {
        const d = s.documents[id];
        const title = d.path ? s.bridge!.baseName(d.path) : "Untitled";
        const slot = press.current?.layout.find((l) => l.id === id);
        const offset = reorder && slot ? renderX(reorder, id, slot.x, contentWidth) - slot.x : 0;
        const isDragged = reorder?.id === id;
        return (
          <div key={id} data-testid="project-tab" data-doc-id={id} className={"tab" + (id === s.activeId ? " active" : "") + (isDragged ? " dragging" : "")}
            style={reorder ? { transform: `translateX(${offset}px)`, transition: isDragged ? "none" : "transform 0.15s ease-out", zIndex: isDragged ? 1 : 0, position: "relative" } : undefined}
            onClick={() => s.setActive(id)}
            onPointerDown={(e) => down(e, id)} onPointerMove={move} onPointerUp={up} onPointerCancel={cancel} onLostPointerCapture={cancel}>
            <span>{title}{d.isModified ? MODIFIED_DOT : ""}</span>
            <button aria-label={`Close ${title}`} onClick={(e) => { e.stopPropagation(); void closeProject(id); }}>x</button>
          </div>
        );
      })}
    </div>
  );
}
