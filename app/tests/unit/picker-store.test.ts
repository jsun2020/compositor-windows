import { beforeEach, describe, expect, it } from "vitest";
import { DEFAULT_PALETTE, pickerTitle, useEditor } from "../../src/state/store";
import { BLACK, WHITE, hexOf, hsbOf, parseHex, type PaletteColor } from "../../src/tools/color";
import { defaultAdjustment } from "../../src/state/adjust-edit";
import type { DocumentState, LayerAdjustment, LayerState, PreviewRequest } from "../../src/engine/types";
import type { EngineClient } from "../../src/engine/client";

// The colour picker (ColorPalette.swift:60-178; ColorPickerTests.canvasSamplingReadsCompositeAndCommitsOnlyOnOK).
const RED: PaletteColor = { red: 1, green: 0, blue: 0 };
// On the 8-bit grid, so the picker (which snaps to it) gives it back exactly.
const TEAL: PaletteColor = { red: 0, green: 128 / 255, blue: 128 / 255 };

function layer(hasMask: boolean): LayerState {
  return { id: "A", name: "A", visible: true, isGroup: false, parentId: null, opacity: 1, blendMode: "Normal",
    transform: { origin: [0, 0], size: [4, 4], rotation: 0, flipX: false, flipY: false, sampling: "High quality" },
    pixelsWidth: 4, pixelsHeight: 4, pixelsRevision: 1, hasPixels: true, hasMask, maskWidth: hasMask ? 4 : 0, maskHeight: hasMask ? 4 : 0,
    maskRevision: 1, maskEnabled: true, maskLinked: true, maskSourceId: null, maskPlacement: null, maskBackground: 255 };
}
function doc(hasMask = false): DocumentState {
  return { id: "D", documentId: "D", width: 4, height: 4, resolution: 72, activeLayerId: "A", canUndo: false, canRedo: false, isModified: false,
    undoDepth: 0, undoEntryId: null, path: null, guides: [], undrawn: [], selection: null, layers: [layer(hasMask)] };
}
let previews: (PreviewRequest | null)[] = [];
/** The Mac test's 4 x 4 image: red on the top two rows, blue below. */
const sampled = (at: { x: number; y: number }): [number, number, number] | null =>
  at.x < 0 || at.y < 0 || at.x >= 4 || at.y >= 4 ? null : at.y < 2 ? [1, 0, 0] : [0, 0, 1];
function install(hasMask = false) {
  previews = [];
  const engine = {
    state: () => doc(hasMask),
    setPreview: (_id: string, r: PreviewRequest | null) => { previews.push(r); return { structure: true, canvas: false, layers: [] }; },
    sampleColor: (_id: string, at: { x: number; y: number }) => sampled(at),
    adjustmentIsIdentity: (a: LayerAdjustment) => JSON.stringify(a) === JSON.stringify(defaultAdjustment(a.kind)),
    histogram: () => [],
  } as unknown as EngineClient;
  useEditor.setState({ engine, jobs: null, activeId: "D", documents: { D: doc(hasMask) }, order: ["D"], selectedLayerIds: ["A"], maskSelected: false,
    working: false, palette: DEFAULT_PALETTE, colorPicker: null, adjustEdit: null, transformEdit: null, error: null, tool: "move" });
}
const s = () => useEditor.getState();

describe("the colour picker", () => {
  beforeEach(() => install());
  it("samples the canvas into itself and commits only on OK, to the swatch it opened on", () => {
    expect(s().openColorPicker({ kind: "palette", background: false })).toBe(true);
    expect(pickerTitle(s().colorPicker!.target)).toBe("Color Picker (Foreground Color)");
    s().sampleIntoPicker({ x: 2, y: 3 });
    expect(hexOf(s().pickerColor()!)).toBe("0000FF");
    expect(s().palette.foreground).toEqual(BLACK);
    s().closeColorPicker(false);
    expect([s().palette.foreground, s().colorPicker]).toEqual([BLACK, null]);
    s().openColorPicker({ kind: "palette", background: true });
    s().sampleIntoPicker({ x: 2, y: 0 });
    s().closeColorPicker(true);
    expect([hexOf(s().palette.background), s().palette.foreground]).toEqual(["FF0000", BLACK]);
  });
  it("keeps its colour where the canvas has none, and switches swatches while open", () => {
    s().setPaletteColor(TEAL, false);
    s().openColorPicker({ kind: "palette", background: false });
    s().sampleIntoPicker({ x: -1, y: 1 });
    expect(s().pickerColor()).toEqual(TEAL);
    s().openColorPicker({ kind: "palette", background: true });
    expect(s().colorPicker!.original).toEqual(WHITE);
    expect(s().colorPicker!.target).toEqual({ kind: "palette", background: true });
  });
  it("snaps its working colour to 8 bits", () => {
    s().openColorPicker({ kind: "palette", background: false });
    s().setPickerHsb({ hue: 200, saturation: 0.333, brightness: 0.777 });
    const c = s().pickerColor()!;
    for (const v of [c.red, c.green, c.blue]) expect(Number.isInteger(Math.round(v * 255 * 1e9) / 1e9)).toBe(true);
    s().closeColorPicker(true);
    expect(s().palette.foreground).toEqual(c);
  });
  it("does not open on a swatch while a mask is the target, and targeting one closes it uncommitted", () => {
    install(true);
    s().openColorPicker({ kind: "palette", background: false });
    s().setPickerHsb(hsbOf(RED));
    s().setMaskSelected(true);
    expect(s().colorPicker).toBeNull();
    expect(s().palette.foreground).toEqual(BLACK);
    expect(s().openColorPicker({ kind: "palette", background: false })).toBe(false);
  });
  it("does not open while a job's result is to come", () => {
    useEditor.setState({ working: true });
    expect(s().openColorPicker({ kind: "palette", background: false })).toBe(false);
  });
});

describe("Gradient Map ends through the picker", () => {
  beforeEach(() => install());
  it("starts a destructive Gradient Map at the palette, previews an end as it changes, and Cancel puts it back", () => {
    s().setPaletteColor(RED, false);
    s().setPaletteColor(TEAL, true);
    expect(s().beginAdjust({ kind: "Gradient Map" })).toBe(true);
    const settings = () => s().adjustEdit!.adjustment!.gradientMapSettings!;
    expect([settings().shadows, settings().highlights]).toEqual([RED, TEAL]);
    expect(s().openColorPicker({ kind: "gradientMap", highlights: true })).toBe(true);
    expect(pickerTitle(s().colorPicker!.target)).toBe("Color Picker (Gradient Map Highlights)");
    expect(s().colorPicker!.original).toEqual(TEAL);
    // A second picker does not open over it.
    expect(s().openColorPicker({ kind: "gradientMap", highlights: false })).toBe(false);
    const before = previews.length;
    s().setPickerHsb(hsbOf(parseHex("FF8000")!));
    expect(hexOf(settings().highlights)).toBe("FF8000");
    expect(previews.length).toBeGreaterThan(before);
    expect((previews.at(-1) as { adjustment: LayerAdjustment }).adjustment.gradientMapSettings!.highlights).toEqual(settings().highlights);
    s().closeColorPicker(false);
    expect(settings().highlights).toEqual(TEAL);
    expect(s().palette.background).toEqual(TEAL);
  });
  it("keeps the end on OK, and goes with its panel", () => {
    s().beginAdjust({ kind: "Gradient Map" });
    s().openColorPicker({ kind: "gradientMap", highlights: false });
    s().setPickerHsb(hsbOf(RED));
    s().closeColorPicker(true);
    expect(s().adjustEdit!.adjustment!.gradientMapSettings!.shadows).toEqual(RED);
    s().openColorPicker({ kind: "gradientMap", highlights: false });
    s().cancelAdjust();
    expect(s().colorPicker).toBeNull();
    expect(s().openColorPicker({ kind: "gradientMap", highlights: false })).toBe(false);
  });
});
