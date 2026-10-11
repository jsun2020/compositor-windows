import { beforeEach, describe, expect, it } from "vitest";
import { DEFAULT_PALETTE, useEditor } from "../../src/state/store";
import { BLACK, WHITE, type PaletteColor } from "../../src/tools/color";
import type { DocumentState, LayerState } from "../../src/engine/types";

// The palette (ColorPalette.swift:20-58): the image's foreground and background, and black or white
// while a mask is the target.
const RED: PaletteColor = { red: 1, green: 0, blue: 0 };
const TEAL: PaletteColor = { red: 0, green: 0.5, blue: 0.5 };

function layer(hasMask: boolean): LayerState {
  return { id: "A", name: "A", visible: true, isGroup: false, parentId: null, opacity: 1, blendMode: "Normal",
    transform: { origin: [0, 0], size: [4, 4], rotation: 0, flipX: false, flipY: false, sampling: "High quality" },
    pixelsWidth: 4, pixelsHeight: 4, pixelsRevision: 1, hasPixels: true, hasMask, maskWidth: hasMask ? 4 : 0, maskHeight: hasMask ? 4 : 0,
    maskRevision: 1, maskEnabled: true, maskLinked: true, maskSourceId: null, maskPlacement: null, maskBackground: 255 };
}
function doc(hasMask: boolean): DocumentState {
  return { id: "D", documentId: "D", width: 4, height: 4, resolution: 72, activeLayerId: "A", canUndo: false, canRedo: false, isModified: false,
    undoDepth: 0, undoEntryId: null, path: null, guides: [], undrawn: [], selection: null, layers: [layer(hasMask)] };
}
function install(hasMask: boolean, maskSelected: boolean) {
  useEditor.setState({ activeId: "D", documents: { D: doc(hasMask) }, maskSelected, working: false, palette: DEFAULT_PALETTE });
}
const s = () => useEditor.getState();

describe("the palette", () => {
  beforeEach(() => install(false, false));
  it("starts black over white and sets, swaps and resets the image's colours", () => {
    expect([s().paletteColor(false), s().paletteColor(true)]).toEqual([BLACK, WHITE]);
    s().setPaletteColor(RED, false);
    s().setPaletteColor(TEAL, true);
    expect([s().paletteColor(false), s().paletteColor(true)]).toEqual([RED, TEAL]);
    s().swapPalette();
    expect([s().paletteColor(false), s().paletteColor(true)]).toEqual([TEAL, RED]);
    s().resetPalette();
    expect([s().paletteColor(false), s().paletteColor(true)]).toEqual([BLACK, WHITE]);
  });
  it("on a targeted mask shows black and white, which swap and reset without touching the image's colours", () => {
    install(true, true);
    useEditor.setState({ palette: { foreground: RED, background: TEAL, maskPaintWhite: false } });
    expect([s().paletteColor(false), s().paletteColor(true)]).toEqual([BLACK, WHITE]);
    s().swapPalette();
    expect([s().paletteColor(false), s().paletteColor(true)]).toEqual([WHITE, BLACK]);
    s().resetPalette();
    expect(s().palette.maskPaintWhite).toBe(false);
    // Setting the background to white makes black the foreground; setting the foreground to any
    // colour other than white makes it black.
    s().setPaletteColor(WHITE, true);
    expect(s().palette.maskPaintWhite).toBe(false);
    s().setPaletteColor(WHITE, false);
    expect(s().palette.maskPaintWhite).toBe(true);
    s().setPaletteColor(RED, false);
    expect(s().palette.maskPaintWhite).toBe(false);
    expect([s().palette.foreground, s().palette.background]).toEqual([RED, TEAL]);
  });
  it("is the image's while the mask chip is chosen on a layer without a mask, or a layer with one is chosen", () => {
    install(false, true);
    useEditor.setState({ palette: { foreground: RED, background: TEAL, maskPaintWhite: true } });
    expect(s().paletteColor(false)).toEqual(RED);
    install(true, false);
    useEditor.setState({ palette: { foreground: RED, background: TEAL, maskPaintWhite: true } });
    expect(s().paletteColor(false)).toEqual(RED);
  });
  it("takes the Eyedropper's sample as the image's foreground, even with a mask targeted, and keeps it off the canvas", () => {
    install(true, true);
    const at: { x: number; y: number }[] = [];
    useEditor.setState({ engine: { sampleColor: (_d: string, p: { x: number; y: number }) => { at.push(p); return p.x < 4 ? [1, 128 / 255, 0] as [number, number, number] : null; } } as never });
    s().sampleForeground({ x: 1.5, y: 2.5 });
    expect(at).toEqual([{ x: 1.5, y: 2.5 }]);
    expect(s().palette.foreground).toEqual({ red: 1, green: 128 / 255, blue: 0 });
    expect(s().paletteColor(false), "the mask's own palette still shows").toEqual(BLACK);
    s().sampleForeground({ x: 9, y: 2 });
    expect(s().palette.foreground).toEqual({ red: 1, green: 128 / 255, blue: 0 });
    useEditor.setState({ working: true });
    s().setPaletteColor(RED, false);
    s().sampleForeground({ x: 0, y: 0 });
    expect(at.length, "no sample while a job's result is to come").toBe(2);
  });
  // Ruling I3: the brief's version of this test ends in resetPalette(), which restores black over
  // white, so `toEqual(DEFAULT_PALETTE)` would hold even with every `working` guard deleted. Instead
  // start from a non-default palette set while `working` is still false, then turn `working` on and
  // check after EACH call that the palette is still exactly {RED, TEAL}.
  it("does not change while a job's result is to come", () => {
    s().setPaletteColor(RED, false);
    s().setPaletteColor(TEAL, true);
    const unchanged = { foreground: RED, background: TEAL, maskPaintWhite: false };
    useEditor.setState({ working: true });
    s().setPaletteColor(WHITE, false);
    expect(s().palette).toEqual(unchanged);
    s().swapPalette();
    expect(s().palette).toEqual(unchanged);
    s().resetPalette();
    expect(s().palette).toEqual(unchanged);
  });
  it("reads every sample but publishes only a changed foreground", () => {
    let rgb: [number, number, number] = [1, 0, 0], reads = 0;
    useEditor.setState({ gradientEdit: null, engine: { sampleColor: () => { reads++; return rgb; } } as never });
    s().sampleForeground({ x: 1, y: 1 });
    const palette = s().palette, tick = s().overlayTick;
    s().sampleForeground({ x: 2, y: 1 });
    expect(reads).toBe(2);
    expect(s().palette).toBe(palette);
    expect(s().overlayTick).toBe(tick);
    // The same location can change after an edit; sampling is never cached.
    rgb = [0, 0.5, 0.5];
    s().sampleForeground({ x: 2, y: 1 });
    expect(reads).toBe(3);
    expect(s().palette.foreground).toEqual(TEAL);
    expect(s().overlayTick).toBeGreaterThan(tick);
  });
  it("repaints the sample ring only when its position or either colour changes", () => {
    useEditor.setState({ sampleRing: null });
    const ring = { at: { x: 10, y: 20 }, sampled: RED, original: BLACK };
    s().setSampleRing(ring);
    const tick = s().overlayTick;
    s().setSampleRing({ at: { ...ring.at }, sampled: { ...RED }, original: { ...BLACK } });
    expect(s().sampleRing).toBe(ring);
    expect(s().overlayTick).toBe(tick);
    s().setSampleRing({ ...ring, at: { x: 11, y: 20 } });
    s().setSampleRing({ ...ring, sampled: TEAL });
    s().setSampleRing({ ...ring, original: WHITE });
    expect(s().overlayTick).toBe(tick + 3);
    s().setSampleRing(null);
    expect(s().sampleRing).toBeNull();
    expect(s().overlayTick).toBe(tick + 4);
    s().setSampleRing(null);
    expect(s().overlayTick).toBe(tick + 4);
  });
});
