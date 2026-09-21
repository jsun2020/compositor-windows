// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { ContextMenu } from "../../src/panels/ContextMenu";

// React 18 reads this to keep act() from warning.
(globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;

let root: Root | null = null;
let host: HTMLDivElement | null = null;

function mount(node: React.ReactElement) {
  host = document.createElement("div");
  document.body.appendChild(host);
  root = createRoot(host);
  act(() => { root!.render(node); });
}
afterEach(() => {
  act(() => { root?.unmount(); });
  host?.remove();
  root = null; host = null;
  vi.restoreAllMocks();
});

const items = [{ id: "x", label: "X", run: () => {} }];

describe("ContextMenu window listeners", () => {
  it("registers once and is not torn down when the parent passes a new inline onClose", () => {
    const add = vi.spyOn(window, "addEventListener");
    const remove = vi.spyOn(window, "removeEventListener");
    mount(<ContextMenu at={{ x: 0, y: 0 }} items={items} onClose={() => {}} />);
    const first = add.mock.calls.filter((c) => c[0] === "keydown" || c[0] === "pointerdown").length;
    expect(first).toBe(2);
    // A new arrow identity on every render is exactly what LayersList passes.
    act(() => { root!.render(<ContextMenu at={{ x: 0, y: 0 }} items={items} onClose={() => {}} />); });
    act(() => { root!.render(<ContextMenu at={{ x: 0, y: 0 }} items={items} onClose={() => {}} />); });
    const after = add.mock.calls.filter((c) => c[0] === "keydown" || c[0] === "pointerdown").length;
    expect(after).toBe(first);
    expect(remove.mock.calls.filter((c) => c[0] === "keydown" || c[0] === "pointerdown").length).toBe(0);
  });

  it("still calls the latest onClose, not the one captured at mount", () => {
    const firstClose = vi.fn();
    const latestClose = vi.fn();
    mount(<ContextMenu at={{ x: 0, y: 0 }} items={items} onClose={firstClose} />);
    act(() => { root!.render(<ContextMenu at={{ x: 0, y: 0 }} items={items} onClose={latestClose} />); });
    act(() => { window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })); });
    expect(latestClose).toHaveBeenCalledTimes(1);
    expect(firstClose).not.toHaveBeenCalled();
  });

  it("stops Escape from reaching a listener bound later on window", () => {
    const shortcut = vi.fn();
    window.addEventListener("keydown", shortcut);
    try {
      mount(<ContextMenu at={{ x: 0, y: 0 }} items={items} onClose={() => {}} />);
      act(() => { window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true })); });
      expect(shortcut).not.toHaveBeenCalled();
      // Any other key still reaches it: only Escape belongs to the open menu.
      act(() => { window.dispatchEvent(new KeyboardEvent("keydown", { key: "z", bubbles: true, cancelable: true })); });
      expect(shortcut).toHaveBeenCalledTimes(1);
    } finally { window.removeEventListener("keydown", shortcut); }
  });
});
