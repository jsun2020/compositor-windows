import { useEffect, useRef } from "react";
import { createRoot } from "react-dom/client";
import { useEditor } from "../state/store";

export type CloseChoice = "save" | "discard" | "cancel";

function CloseProjectSheet({ name, done }: { name: string; done(choice: CloseChoice): void }) {
  const dialog = useRef<HTMLDivElement>(null);
  const cancel = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    const previous = document.activeElement;
    cancel.current?.focus();
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault(); e.stopImmediatePropagation(); done("cancel");
      } else if (e.key === "Tab") {
        const buttons = Array.from(dialog.current!.querySelectorAll<HTMLButtonElement>("button"));
        const index = buttons.indexOf(document.activeElement as HTMLButtonElement);
        e.preventDefault();
        const next = index < 0 ? (e.shiftKey ? buttons.length - 1 : 0) : (index + (e.shiftKey ? -1 : 1) + buttons.length) % buttons.length;
        buttons[next].focus();
      }
    };
    // A background tab may close while the active document's panel stays open.
    // This modal owns Escape first, so cancelling close cannot cancel that panel.
    window.addEventListener("keydown", key, true);
    return () => {
      window.removeEventListener("keydown", key, true);
      if (previous instanceof HTMLElement && previous.isConnected) previous.focus();
    };
  }, [done]);
  return <div ref={dialog} className="sheet-backdrop" role="dialog" aria-modal="true" aria-label="Close project">
    <div className="sheet">
      <h2>Save changes to {name} before closing?</h2>
      <div className="sheet-body">Closing without saving will discard this project's unsaved changes.</div>
      <div className="sheet-buttons">
        <button onClick={() => done("discard")}>Don't Save and Close</button>
        <button ref={cancel} onClick={() => done("cancel")}>Cancel Close</button>
        <button className="primary" onClick={() => done("save")}>Save and Close</button>
      </div>
    </div>
  </div>;
}

/** The file operation retains its busy guard until both the choice and any save finish. */
export function chooseProjectClose(name: string): Promise<CloseChoice> {
  return new Promise(resolve => {
    // Deliberately preserve an active panel when closing a different, background project.
    useEditor.setState({ sheet: { kind: "closeProject" } });
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    let settled = false;
    const done = (choice: CloseChoice) => {
      if (settled) return;
      settled = true;
      queueMicrotask(() => {
        root.unmount(); host.remove();
        if (useEditor.getState().sheet?.kind === "closeProject") useEditor.getState().closeSheet();
        resolve(choice);
      });
    };
    root.render(<CloseProjectSheet name={name} done={done} />);
  });
}
