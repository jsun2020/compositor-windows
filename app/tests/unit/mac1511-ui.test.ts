import { beforeEach, describe, expect, it } from "vitest";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { ADJUSTMENT_KINDS, DEFAULT_SHARPEN, DEFAULT_TONAL, DEFAULT_VIGNETTE, isSpatialKind, type Command, type DocumentState, type LayerState, type LayerTransform } from "../../src/engine/types";
import { defaultAdjustment, defaultFilterParams, FILTER_TITLES, isAdjustIdentity, isFilterKind } from "../../src/state/adjust-edit";
import { defaultFilterLayerSettings, defaultLensSettings, effectivePrismFringe, filterLayerParams, withFilterLayerParams } from "../../src/state/filter-layers";
import { normalizeCustomRatioText, parseCustomRatio, ratioValue } from "../../src/tools/crop-tool";
import { pickerTitle, useEditor, type PickerTarget } from "../../src/state/store";
import { shouldIgnoreSpaceForHand } from "../../src/canvas/space-hand";
import { canAlignSelection, canDistributeSelection } from "../../src/panels/MoveOptions";
import type { EngineClient } from "../../src/engine/client";
import { cornersOf, cornersToTuples } from "../../src/tools/transform-geometry";

const here = path.dirname(fileURLToPath(import.meta.url));

describe("Mac 1.5.11 UI contract", () => {
  it("exposes new adjustment kinds as spatial", () => {
    for (const kind of ["Vignette", "Lens Effects", "Tonal Contrast", "Sharpen"] as const) {
      expect(ADJUSTMENT_KINDS).toContain(kind);
      expect(isSpatialKind(kind)).toBe(true);
    }
  });

  it("wires filter params and prism / tonal identity rules", () => {
    expect(isFilterKind("Vignette")).toBe(true);
    expect(FILTER_TITLES.Sharpen).toBe("Sharpen");
    const vignette = defaultFilterParams("Vignette");
    expect(vignette).toEqual({ filter: "Vignette", settings: DEFAULT_VIGNETTE });
    expect(isAdjustIdentity({ params: { filter: "Vignette", settings: { ...DEFAULT_VIGNETTE, amount: 0 } }, adjustment: null, original: null }, () => false)).toBe(true);
    // Either zero amount or zero contribution from every tone is identity.
    expect(isAdjustIdentity({ params: { filter: "TonalContrast", settings: { ...DEFAULT_TONAL, amount: 0 } }, adjustment: null, original: null }, () => false)).toBe(true);
    expect(isAdjustIdentity({ params: { filter: "TonalContrast", settings: { amount: 75, radius: 16, shadows: 0, midtones: 0, highlights: 0 } }, adjustment: null, original: null }, () => false)).toBe(true);
    const lens = defaultFilterParams("LensEffects");
    if (lens.filter !== "LensEffects") throw new Error("kind");
    expect(lens.settings.prismAmount).toBe(80);
    expect(isAdjustIdentity({ params: { ...lens, settings: { ...lens.settings, effect: "Prism", prismAmount: 10, prismOffset: 0 } }, adjustment: null, original: null }, () => false)).toBe(false);
    expect(isAdjustIdentity({ params: { ...lens, settings: { ...lens.settings, effect: "Prism", prismAmount: 0 } }, adjustment: null, original: null }, () => false)).toBe(true);
    expect(defaultFilterParams("Sharpen")).toEqual({ filter: "Sharpen", amount: DEFAULT_SHARPEN.amount, radius: DEFAULT_SHARPEN.radius });
  });

  it("round-trips vignette/tonal/lens/sharpen through filterLayerSettings", () => {
    for (const kind of ["Vignette", "Tonal Contrast", "Lens Effects", "Sharpen"] as const) {
      const a = defaultAdjustment(kind);
      const params = filterLayerParams(a);
      expect(params).not.toBeNull();
      const changed = withFilterLayerParams(a, params!);
      expect(filterLayerParams(changed)?.filter).toBe(params!.filter);
    }
    expect(effectivePrismFringe({ ...defaultLensSettings(), prismFringe: undefined })).toBe(30);
    expect(defaultFilterLayerSettings().vignette).toBeUndefined();
  });

  it("parses and normalizes crop ratios with upstream bounds", () => {
    const doc = { width: 200, height: 100 } as any;
    expect(ratioValue("Free", doc)).toBeNull();
    expect(ratioValue("Canvas", doc)).toBe(2);
    expect(ratioValue("2.39:1", doc)).toBeCloseTo(2.39);
    expect(parseCustomRatio("4:5")).toBeCloseTo(0.8);
    expect(normalizeCustomRatioText(" 4 : 5 ")).toBe("4:5");
    expect(parseCustomRatio("bad")).toBeNull();
    expect(parseCustomRatio("0:1")).toBeNull();
    expect(parseCustomRatio("1:30001")).toBeNull();
  });

  it("routes new spatial kinds through gl-renderer CPU-reference detection", () => {
    const src = fs.readFileSync(path.resolve(here, "../../src/canvas/gl-renderer.ts"), "utf8");
    for (const kind of ["Vignette", "Lens Effects", "Tonal Contrast", "Sharpen"]) {
      expect(src.includes(`"${kind}": ADJUST_KIND.identity`)).toBe(true);
    }
    expect(src.includes('"Vignette","Lens Effects","Tonal Contrast","Sharpen"')).toBe(true);
  });
});

describe("vignette color picker (destructive + adjustment layer)", () => {
  function layer(): LayerState {
    return { id: "A", name: "A", visible: true, isGroup: false, parentId: null, opacity: 1, blendMode: "Normal",
      transform: { origin: [0, 0], size: [4, 4], rotation: 0, flipX: false, flipY: false, sampling: "High quality" },
      pixelsWidth: 4, pixelsHeight: 4, pixelsRevision: 1, hasPixels: true, hasMask: false, maskWidth: 0, maskHeight: 0,
      maskRevision: 1, maskEnabled: true, maskLinked: true, maskSourceId: null, maskPlacement: null, maskBackground: 255 };
  }
  function doc(): DocumentState {
    return { id: "D", documentId: "D", width: 4, height: 4, resolution: 72, activeLayerId: "A", canUndo: false, canRedo: false, isModified: false,
      undoDepth: 0, undoEntryId: null, path: null, guides: [], undrawn: [], selection: null, layers: [layer()] };
  }
  beforeEach(() => {
    const engine = {
      state: () => doc(),
      setPreview: () => ({ structure: true, canvas: false, layers: [] }),
      sampleColor: () => null,
      adjustmentIsIdentity: () => false,
      histogram: () => [],
    } as unknown as EngineClient;
    useEditor.setState({
      engine, jobs: null, activeId: "D", documents: { D: doc() }, order: ["D"], selectedLayerIds: ["A"], maskSelected: false,
      working: false, colorPicker: null, adjustEdit: null, transformEdit: null, gradientEdit: null, sheet: null, error: null, tool: "move",
    });
  });

  it.each(["Vignette", "Sharpen"] as const)("opens existing %s adjustment without replacing stored settings", (kind) => {
    const adjustment = defaultAdjustment(kind);
    adjustment.filterLayerSettings = { ...adjustment.filterLayerSettings!,
      ...(kind === "Vignette" ? { vignette: { ...DEFAULT_VIGNETTE, amount: 73 } } : { sharpen: { amount: 123, radius: 3.5 } }) };
    const d = doc(); d.layers[0] = { ...d.layers[0], hasPixels: false, adjustment };
    useEditor.setState({ documents: { D: d } });
    expect(useEditor.getState().beginAdjust({ kind, target: "adjustmentLayer", layerId: "A" })).toBe(true);
    expect(useEditor.getState().adjustEdit).toMatchObject({ params: null, adjustment, original: adjustment, target: "adjustmentLayer" });
  });

  it("allows only Vignette on blank pixel layers and respects selection/mask/visibility guards", () => {
    const d = doc(); d.layers[0].hasPixels = false;
    useEditor.setState({ documents: { D: d } });
    const s = () => useEditor.getState();
    expect(s().canAdjust()).toBe(false);
    expect(s().canAdjust("Sharpen")).toBe(false);
    expect(s().canAdjust("Vignette")).toBe(true);
    useEditor.setState({ maskSelected: true }); expect(s().canAdjust("Vignette")).toBe(false);
    useEditor.setState({ maskSelected: false });
    d.layers[0].visible = false; expect(s().canAdjust("Vignette")).toBe(false);
    d.layers[0].visible = true;
    expect(s().beginAdjust({ kind: "Vignette" })).toBe(true);
    expect(s().adjustEdit?.params?.filter).toBe("Vignette");
    expect(s().openColorPicker({ kind: "vignette" })).toBe(true);
    s().cancelAdjust();
    expect(s().adjustEdit).toBeNull(); expect(s().colorPicker).toBeNull();
  });

  it("sizes blank Vignette jobs from the canvas instead of an absent pixel asset", () => {
    const d = doc(); d.width = 6000; d.height = 4000; d.layers[0].hasPixels = false;
    const engine = { ...useEditor.getState().engine!, storedPixels: () => 0 } as unknown as EngineClient;
    useEditor.setState({ engine, jobs: {} as any, documents: { D: d }, jobPixels: 1_000_000 });
    expect(useEditor.getState().usesJob("A")).toBe(false);
    expect(useEditor.getState().usesJob("A", true)).toBe(true);
  });

  it("titles vignette target", () => {
    const t: PickerTarget = { kind: "vignette" };
    expect(pickerTitle(t)).toBe("Color Picker (Vignette Color)");
  });

  it("opens on destructive Vignette params, previews on drag, Cancel restores", () => {
    const s = () => useEditor.getState();
    const original = { ...DEFAULT_VIGNETTE, color: { red: 0.2, green: 0.1, blue: 0.1 } };
    useEditor.setState({
      adjustEdit: {
        kind: "Vignette", target: "layer", layerId: "A", adjustment: null,
        params: { filter: "Vignette", settings: original }, original: null, preview: true, sampleMode: null, histogram: null,
      },
    });
    expect(s().openColorPicker({ kind: "vignette" })).toBe(true);
    expect(s().colorPicker!.original).toEqual(original.color);
    s().setPickerHsb({ hue: 0, saturation: 1, brightness: 1 });
    expect(s().adjustEdit!.params).toMatchObject({ filter: "Vignette", settings: { color: { red: 1, green: 0, blue: 0 } } });
    s().closeColorPicker(false);
    expect(s().adjustEdit!.params).toMatchObject({ filter: "Vignette", settings: { color: original.color } });
    expect(s().colorPicker).toBeNull();
  });

  it("opens on adjustment-layer Vignette filterLayerSettings and OK commits", () => {
    const s = () => useEditor.getState();
    const adjustment = {
      ...defaultAdjustment("Vignette"),
      filterLayerSettings: { ...defaultFilterLayerSettings(), vignette: { ...DEFAULT_VIGNETTE, color: { red: 0, green: 0, blue: 0 } } },
    };
    useEditor.setState({
      adjustEdit: {
        kind: "Vignette", target: "adjustmentLayer", layerId: "A", adjustment, params: null,
        original: adjustment, preview: true, sampleMode: null, histogram: null,
      },
    });
    expect(s().openColorPicker({ kind: "vignette" })).toBe(true);
    s().setPickerHsb({ hue: 120, saturation: 1, brightness: 1 });
    s().closeColorPicker(true);
    const color = s().adjustEdit!.adjustment!.filterLayerSettings!.vignette!.color;
    expect(color.green).toBeGreaterThan(0.5);
    expect(s().colorPicker).toBeNull();
  });
});

describe("Space hand target guards", () => {
  it("ignores editable and dialog targets; allows canvas", () => {
    const input = { closest: (sel: string) => (sel.includes("input") ? {} : null) } as unknown as HTMLElement;
    const canvas = { closest: () => null } as unknown as HTMLElement;
    const dialog = { closest: (sel: string) => (sel.includes("dialog") || sel.includes("color-picker") ? {} : null) } as unknown as HTMLElement;
    expect(shouldIgnoreSpaceForHand(input)).toBe(true);
    expect(shouldIgnoreSpaceForHand(dialog)).toBe(true);
    expect(shouldIgnoreSpaceForHand(canvas)).toBe(false);
    expect(shouldIgnoreSpaceForHand(null)).toBe(false);
  });
});

describe("Align / Distribute enablement and dispatch", () => {
  const original: LayerTransform = { origin: [0, 0], size: [10, 10], rotation: 0, flipX: false, flipY: false, sampling: "High quality" };
  function layer(id: string): LayerState {
    return {
      id, name: id, visible: true, isGroup: false, parentId: null, opacity: 1, blendMode: "Normal", transform: original,
      pixelsWidth: 10, pixelsHeight: 10, pixelsRevision: 1, hasPixels: true, hasMask: false, maskWidth: 0, maskHeight: 0, maskRevision: 0,
      maskEnabled: true, maskLinked: true, maskSourceId: null, maskPlacement: null, maskBackground: 255,
    };
  }
  function document(ids: string[]): DocumentState {
    return { id: "D", documentId: "D", width: 100, height: 100, resolution: 72, activeLayerId: ids[0], canUndo: false, canRedo: false, isModified: false, undoDepth: 0, undoEntryId: null, path: null, guides: [], undrawn: [], selection: null, layers: ids.map(layer) };
  }

  it("disables align under perspective distortion; distribute needs 3+", () => {
    expect(canAlignSelection({ working: false, panelOwns: false, distorting: true, ids: ["A", "B"] })).toBe(false);
    expect(canAlignSelection({ working: false, panelOwns: false, distorting: false, ids: ["A"] })).toBe(true);
    expect(canDistributeSelection({ working: false, panelOwns: false, distorting: false, ids: ["A", "B"] })).toBe(false);
    expect(canDistributeSelection({ working: false, panelOwns: false, distorting: false, ids: ["A", "B", "C"] })).toBe(true);
  });

  it("commits pending transform before AlignLayers and records the command", () => {
    const doc = document(["A", "B"]);
    const calls: Command[] = [];
    const engine = {
      state: () => doc,
      execute: (_id: string, cmd: Command) => { calls.push(cmd); return { structure: true, canvas: false, layers: [] }; },
      undo: () => ({ structure: true, canvas: false, layers: [] }),
      redo: () => ({ structure: true, canvas: false, layers: [] }),
      closeDocument: () => {},
      setPreview: () => ({ structure: false, canvas: false, layers: [] }),
    } as unknown as EngineClient;
    const draft = { ...original, origin: [5, 0] as [number, number] };
    useEditor.setState({
      engine, jobs: null, activeId: "D", documents: { D: doc }, selectedLayerIds: ["A", "B"], maskSelected: false, working: false,
      adjustEdit: null, gradientEdit: null, sheet: null, error: null,
      transformEdit: { kind: "group", id: "A", ids: ["A", "B"], box: original, original, draft, corners: null, persistent: true, duplicated: false, duplicateEntry: null },
    });
    // run() itself commits a pending transform before recording the next command (store.run).
    const ok = useEditor.getState().run({ type: "AlignLayers", ids: ["A", "B"], alignment: "Left Edges" });
    expect(ok).toBe(true);
    expect(useEditor.getState().error).toBeNull();
    expect(useEditor.getState().transformEdit).toBeNull();
    expect(calls.map((c) => c.type)).toEqual(["TransformLayers", "AlignLayers"]);
    expect(calls[1]).toMatchObject({ type: "AlignLayers", ids: ["A", "B"], alignment: "Left Edges" });
  });

  it("does not treat distorted corners as alignable", () => {
    const corners = cornersToTuples(cornersOf(original));
    expect(canAlignSelection({ working: false, panelOwns: false, distorting: !!corners, ids: ["A", "B"] })).toBe(false);
  });
});
