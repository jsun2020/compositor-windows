// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { ToolRail } from "../../src/panels/ToolRail";
import { DEFAULT_SELECTION_OPTIONS, useEditor } from "../../src/state/store";

// React 18 reads this to keep act() from warning.
(globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;

let resized: ResizeObserverCallback;
beforeEach(()=>vi.stubGlobal("ResizeObserver",class { constructor(callback:ResizeObserverCallback){resized=callback;} observe(){} disconnect(){} }));
let root: Root | null = null;
let host: HTMLDivElement | null = null;

function mount() {
  host = document.createElement("div");
  document.body.appendChild(host);
  root = createRoot(host);
  act(() => { root!.render(<ToolRail />); });
}
afterEach(() => {
  act(() => { root?.unmount(); });
  host?.remove();
  root = null; host = null; vi.unstubAllGlobals();
  useEditor.setState({ selectionOptions: DEFAULT_SELECTION_OPTIONS });
});

const button = (id: string) => host!.querySelector(`[data-testid="tool-${id}"]`) as HTMLButtonElement;
const icon = (id: string) => button(id).querySelector("svg")!.getAttribute("data-icon");

describe("ToolRail", () => {
  it("shows an icon, not a letter, on every tool, named for assistive technology", () => {
    mount();
    const expected: Record<string, string> = { move: "Move", marquee: "Marquee", lasso: "Lasso", wand: "Magic Wand",
      crop: "Crop", gradient: "Gradient", shape: "Shape", eyedropper: "Eyedropper", hand: "Hand", zoom: "Zoom" };
    for (const [id, label] of Object.entries(expected)) {
      const b = button(id);
      expect(b.querySelectorAll("svg").length, id).toBe(1);
      expect(b.textContent, id).toBe("");
      expect(b.getAttribute("aria-label"), id).toBe(label);
    }
    // Each tool draws its own picture.
    const names = Object.keys(expected).map(icon);
    expect(new Set(names).size).toBe(names.length);
  });

  it("follows the Marquee's and the Lasso's mode, as the Mac's rail does", () => {
    mount();
    expect(icon("marquee")).toBe("marquee-rectangle");
    expect(icon("lasso")).toBe("lasso-freehand");
    act(() => { useEditor.setState({ selectionOptions: { ...DEFAULT_SELECTION_OPTIONS, marquee: "Ellipse", lasso: "Polygonal" } }); });
    expect(icon("marquee")).toBe("marquee-ellipse");
    expect(icon("lasso")).toBe("lasso-polygonal");
  });
});

it("uses two columns only while a single column does not fit",()=>{
  mount();
  act(()=>resized([{contentRect:{height:400}} as ResizeObserverEntry],{} as ResizeObserver));
  expect(host!.querySelector(".tool-rail")!.classList.contains("two-columns")).toBe(true);
  act(()=>resized([{contentRect:{height:900}} as ResizeObserverEntry],{} as ResizeObserver));
  expect(host!.querySelector(".tool-rail")!.classList.contains("two-columns")).toBe(false);
});
