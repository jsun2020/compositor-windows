import { describe, expect, it } from "vitest";
import { useEditor } from "../../src/state/store";
import type { DocumentState } from "../../src/engine/types";

// The crop tool seeds a full-canvas rectangle on entry (macOS EditorSession.selectTool) and
// clears it on exit. Apply and Cancel clear it too, so the frame and the buttons follow the
// rectangle instead of an implicit full-canvas default that could never disappear.

function document(): DocumentState {
  return { id: "D", documentId: "D", width: 120, height: 80, resolution: 72, activeLayerId: null, canUndo: false, canRedo: false, isModified: false, undoDepth: 0, path: null, guides: [], layers: [] };
}

describe("crop tool rectangle seeding", () => {
  it("seeds the full canvas when the crop tool is chosen and clears it when leaving", () => {
    useEditor.setState({ engine: null, activeId: "D", documents: { D: document() }, tool: "move", cropRect: null, transformEdit: null });
    useEditor.getState().setTool("crop");
    expect(useEditor.getState().cropRect).toEqual({ x: 0, y: 0, width: 120, height: 80 });
    useEditor.getState().setTool("move");
    expect(useEditor.getState().cropRect).toBeNull();
  });

  it("keeps an existing rectangle when the crop tool is re-selected", () => {
    useEditor.setState({ engine: null, activeId: "D", documents: { D: document() }, tool: "crop", cropRect: { x: 10, y: 10, width: 50, height: 40 }, transformEdit: null });
    useEditor.getState().setTool("crop");
    expect(useEditor.getState().cropRect).toEqual({ x: 10, y: 10, width: 50, height: 40 });
  });

  it("seeds nothing without a document", () => {
    useEditor.setState({ engine: null, activeId: null, documents: {}, tool: "move", cropRect: null, transformEdit: null });
    useEditor.getState().setTool("crop");
    expect(useEditor.getState().cropRect).toBeNull();
  });
});
