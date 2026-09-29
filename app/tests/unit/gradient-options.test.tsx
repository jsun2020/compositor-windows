// @vitest-environment jsdom
import { afterEach, describe, expect, it } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { GradientOptions } from "../../src/panels/GradientOptions";
import { DEFAULT_PALETTE, JOB_PIXELS, useEditor } from "../../src/state/store";
import { DEFAULT_GRADIENT } from "../../src/state/gradient-edit";
import type { DocumentState, LayerState } from "../../src/engine/types";

// React 18 reads this to keep act() from warning.
(globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;

const RED = { red: 1, green: 0, blue: 0 }, BLUE = { red: 0, green: 0, blue: 1 };

function layer(patch: Partial<LayerState> = {}): LayerState {
  return { id: "A", name: "A", visible: true, isGroup: false, parentId: null, opacity: 1, blendMode: "Normal",
    transform: { origin: [0, 0], size: [40, 30], rotation: 0, flipX: false, flipY: false, sampling: "High quality" },
    pixelsWidth: 40, pixelsHeight: 30, pixelsRevision: 1, hasPixels: true, hasMask: true, maskWidth: 40, maskHeight: 30,
    maskRevision: 1, maskEnabled: true, maskLinked: true, maskSourceId: null, maskPlacement: null, maskBackground: 255, ...patch };
}
function doc(l: LayerState): DocumentState {
  return { id: "D", documentId: "D", width: 40, height: 30, resolution: 72, activeLayerId: l.id, canUndo: true, canRedo: false, isModified: false,
    undoDepth: 3, undoEntryId: 3, path: null, guides: [], undrawn: [], selection: null, layers: [l] };
}

let root: Root | null = null;
let host: HTMLDivElement | null = null;
function mount() {
  host = document.createElement("div");
  document.body.appendChild(host);
  root = createRoot(host);
  act(() => { root!.render(<GradientOptions />); });
}
afterEach(() => {
  act(() => { root?.unmount(); });
  host?.remove();
  root = null; host = null;
});

describe("GradientOptions", () => {
  it("shows the mask's black or white on the swatch while a mask is the target, fix round 1 (I-2)", () => {
    const l = layer();
    useEditor.setState({ jobPixels: JOB_PIXELS, activeId: "D", documents: { D: doc(l) }, order: ["D"], selectedLayerIds: [l.id],
      maskSelected: true, working: false, palette: { ...DEFAULT_PALETTE, foreground: RED, background: BLUE },
      gradientOptions: DEFAULT_GRADIENT, gradientEdit: null, adjustEdit: null, transformEdit: null, error: null,
      tool: "gradient", cropRect: null, sheet: null, colorPicker: null });
    mount();
    const swatch = host!.querySelector('[data-testid="gradient-swatch"]') as HTMLElement;
    // The palette's black (foreground) while a mask is the target, never the image's foreground (red).
    expect(swatch.style.backgroundImage).toContain("rgba(0, 0, 0, 1)");
    expect(swatch.style.backgroundImage).not.toContain("rgba(255, 0, 0");
  });
});
