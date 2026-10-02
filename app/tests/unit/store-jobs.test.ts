import { beforeEach, describe, expect, it, vi } from "vitest";
import { BUSY_MESSAGE, JOB_PIXELS, useEditor } from "../../src/state/store";
import { defaultAdjustment } from "../../src/state/adjust-edit";
import { closeProject, saveProject } from "../../src/actions/files";
import type { Command, DocumentState, LayerAdjustment, LayerState, PreviewRequest } from "../../src/engine/types";
import type { EngineClient } from "../../src/engine/client";
import type { JobClient, JobRequest, JobResult } from "../../src/engine/jobs";
import type { ShellBridge } from "../../src/shell/bridge";
import type { Viewport } from "../../src/canvas/viewport";

function layer(id: string, width: number, height: number): LayerState {
  return { id, name: id, visible: true, isGroup: false, parentId: null, opacity: 1, blendMode: "Normal",
    transform: { origin: [0, 0], size: [width, height], rotation: 0, flipX: false, flipY: false, sampling: "High quality" },
    pixelsWidth: width, pixelsHeight: height, pixelsRevision: 1, hasPixels: true, hasMask: false, maskWidth: 0, maskHeight: 0,
    maskRevision: 0, maskEnabled: true, maskLinked: true, maskSourceId: null, maskPlacement: null, maskBackground: 255 };
}
function document(l: LayerState): DocumentState {
  return { id: "D", documentId: "D", width: l.pixelsWidth, height: l.pixelsHeight, resolution: 72, activeLayerId: l.id, canUndo: false,
    canRedo: false, isModified: false, undoDepth: 0, undoEntryId: null, path: null, guides: [], undrawn: [], selection: null, layers: [l] };
}
const bins = () => [0, 1, 2, 3].map((c) => new Array(256).fill(c));

/** A store over one layer of `width` x `height` with a stub engine and a stub job worker whose jobs
 * finish when `finish` is called. */
function install(width: number, height: number, onInstall?: () => void) {
  const log: string[] = [];
  const previews: (PreviewRequest | null)[] = [];
  const requests: JobRequest[] = [];
  const displays: (ArrayBuffer | null)[] = [];
  let finish: (r: JobResult | null) => void = () => {};
  const engine = {
    state: () => document(layer("A", width, height)),
    execute: (_id: string, cmd: Command) => { log.push(`execute ${cmd.type}`); return { structure: true, canvas: false, layers: [] }; },
    setPreview: (_id: string, request: PreviewRequest | null) => { previews.push(request); return { structure: true, canvas: false, layers: [] }; },
    histogram: () => { log.push("histogram here"); return bins(); },
    storedPixels: () => useEditor.getState().documents.D.layers[0].pixelsWidth * useEditor.getState().documents.D.layers[0].pixelsHeight,
    // Task 5's jobs API: a job's input carries its selection's points as a separate buffer (null
    // here -- these fixtures have no selection), beside the JSON and the pixel/mask buffers.
    jobInput: () => { log.push("job input"); return { input: '{"stamp":{"pixelsRevision":1}}', pixels: new ArrayBuffer(4), mask: null, points: null }; },
    jobInputAsync: () => { log.push("job input"); return { input: '{"stamp":{"pixelsRevision":1}}', pixels: new ArrayBuffer(4), mask: null, points: null }; },
    installJobAsync: (_doc: string, layerId: string, input: string, output: string, _pixels: ArrayBuffer | null, _mask: ArrayBuffer | null, display: ArrayBuffer | null) => { log.push(`install ${layerId} ${output}`); displays.push(display); onInstall?.(); expect(input).toContain("stamp"); return { structure: true, canvas: false, layers: [] }; },
    undo: () => { log.push("undo"); return { structure: true, canvas: false, layers: [] }; },
    adjustmentIsIdentity: (a: LayerAdjustment) => JSON.stringify(a) === JSON.stringify(defaultAdjustment(a.kind)),
  } as unknown as EngineClient;
  const jobs = { run: (_channel: string, request: JobRequest) => { requests.push(request); return new Promise<JobResult | null>((resolve) => { finish = resolve; }); } } as unknown as JobClient;
  useEditor.setState({ engine, jobs, jobPixels: JOB_PIXELS, activeId: "D", documents: { D: document(layer("A", width, height)) }, order: ["D"], selectedLayerIds: ["A"],
    maskSelected: false, transformEdit: null, adjustEdit: null, error: null, tool: "move", cropRect: null, sheet: null, working: false, viewports: {} });
  return { log, previews, requests, displays, finish: (r: JobResult | null) => finish(r) };
}
/** Opens Levels on layer A and moves a slider, so OK has something to apply. */
function levelsChanged() {
  useEditor.getState().beginAdjust({ kind: "Levels" });
  const next = defaultAdjustment("Levels");
  next.levels.ranges[0] = { ...next.levels.ranges[0], outputWhite: 200 };
  useEditor.getState().updateAdjust({ adjustment: next });
}
const flush = () => new Promise((r) => setTimeout(r, 0));

beforeEach(() => useEditor.setState({ jobs: null, working: false, error: null }));

describe("destructive commits on large layers go to the job worker", () => {
  it("OK hands a large layer's edit to the worker, keeps the preview until the result is back, and puts it back once", async () => {
    // Just over the threshold: 2001 x 2000.
    const { log, previews, requests, finish } = install(2001, 2000);
    levelsChanged();
    const shown = previews.length;
    useEditor.getState().commitAdjust();
    expect(useEditor.getState().adjustEdit, "the panel closes at once").toBeNull();
    expect(useEditor.getState().working).toBe(true);
    expect(previews.length, "the preview stays on the canvas meanwhile").toBe(shown);
    expect(log.filter((l) => l.startsWith("execute"))).toEqual([]);
    const command = JSON.parse((requests.at(-1) as { command: string }).command) as Command;
    expect(command.type).toBe("ApplyAdjustment");
    finish({ header: "OUT", pixels: new ArrayBuffer(4), mask: null });
    await flush();
    expect(log.at(-1)).toBe("install A OUT");
    expect(useEditor.getState().working).toBe(false);
    expect(previews.length, "the engine cleared its own preview as it put the result back").toBe(shown);
  });

  it("an edit job carries the scale the canvas draws at, and the result's halving goes back with it (F1)", async () => {
    const { requests, displays, finish } = install(2001, 2000);
    // An eighth of a CSS pixel per document pixel; the node test has no devicePixelRatio, so 1.
    useEditor.setState({ viewports: { D: { pointsPerPixel: 0.125 } as unknown as Viewport } });
    levelsChanged();
    useEditor.getState().commitAdjust();
    const request = requests.at(-1)!;
    expect(request.kind === "edit" && request.outPerDoc).toBe(0.125);
    const display = new ArrayBuffer(8);
    finish({ header: "OUT", pixels: new ArrayBuffer(4), mask: null, display });
    await flush();
    expect(displays).toEqual([display]);
    expect(displays[0], "the very buffer the worker sent").toBe(display);
  });

  it("a layer at the threshold is edited on the UI thread as before", () => {
    const { log, requests } = install(2000, 2000);
    levelsChanged();
    useEditor.getState().commitAdjust();
    expect(requests.filter((r) => r.kind === "edit")).toEqual([]);
    expect(log.filter((l) => l.startsWith("execute"))).toEqual(["execute ApplyAdjustment"]);
  });

  it("while a job's result is to come, commands and undo wait", async () => {
    const { log, finish } = install(2001, 2000);
    levelsChanged();
    useEditor.getState().commitAdjust();
    expect(useEditor.getState().run({ type: "AddBlankLayer" })).toBe(false);
    expect(useEditor.getState().error).toBe(BUSY_MESSAGE);
    useEditor.getState().undo();
    expect(log).not.toContain("undo");
    expect(log).not.toContain("execute AddBlankLayer");
    finish(null);
    await flush();
    expect(useEditor.getState().run({ type: "AddBlankLayer" })).toBe(true);
  });

  it("a result the engine will not put back takes the preview away and says why", async () => {
    const { previews, finish } = install(2001, 2000, () => { throw new Error("The layer changed while the edit was being made, so it was not applied."); });
    levelsChanged();
    useEditor.getState().commitAdjust();
    finish({ header: "OUT", pixels: null, mask: null });
    await flush();
    expect(useEditor.getState().error).toContain("The layer changed");
    expect(previews.at(-1)).toBeNull();
    expect(useEditor.getState().working).toBe(false);
  });

  it("a large layer's Levels histogram arrives from the worker, into the panel it was read for only", async () => {
    const { log, requests, finish } = install(2001, 2000);
    useEditor.getState().beginAdjust({ kind: "Levels" });
    expect(useEditor.getState().adjustEdit!.histogram, "the panel opens without waiting").toBeNull();
    expect(log).not.toContain("histogram here");
    expect(requests.at(-1)?.kind).toBe("histogram");
    finish({ header: JSON.stringify(bins()), pixels: null, mask: null });
    await flush();
    expect(useEditor.getState().adjustEdit!.histogram).toEqual(bins());
    // A histogram that lands after its panel closed goes nowhere.
    useEditor.getState().cancelAdjust();
    useEditor.getState().beginAdjust({ kind: "Curves" });
    const late = finish;
    useEditor.getState().cancelAdjust();
    late({ header: JSON.stringify(bins()), pixels: null, mask: null });
    await flush();
    expect(useEditor.getState().adjustEdit).toBeNull();
  });

  // Fix round 1: `working` must gate every panel/transform entry point, not just `run`/`undo`/`redo`
  // -- a shortcut (Ctrl+L etc.) calls `beginAdjust` directly, without going through `canAdjust`
  // first, and `commitAdjust` used to close the panel (`adjustEdit: null`) before `runEditJob`'s own
  // `working` refusal ran, losing the user's settings behind a banner.
  it("beginAdjust refused while working: no panel opens, and canAdjust says no too", () => {
    const { log, requests } = install(2001, 2000);
    useEditor.setState({ working: true });
    expect(useEditor.getState().canAdjust()).toBe(false);
    expect(useEditor.getState().beginAdjust({ kind: "Levels" })).toBe(false);
    expect(useEditor.getState().adjustEdit).toBeNull();
    expect(log).not.toContain("histogram here");
    expect(requests).toEqual([]);
  });

  it("re-editing an adjustment layer is refused while working, though it bypasses canAdjust (Task 6 deferred minor)", () => {
    const { log, requests } = install(2001, 2000);
    const levels = defaultAdjustment("Levels");
    const adjusted: LayerState = { ...layer("A", 2001, 2000), hasPixels: false, adjustment: levels };
    useEditor.setState({ documents: { D: document(adjusted) }, working: true });
    expect(useEditor.getState().beginAdjust({ kind: "Levels", layerId: "A", target: "adjustmentLayer" })).toBe(false);
    expect(useEditor.getState().adjustEdit).toBeNull();
    expect([log, requests]).toEqual([[], []]);
    // The same call opens the panel once the result is in.
    useEditor.setState({ working: false });
    expect(useEditor.getState().beginAdjust({ kind: "Levels", layerId: "A", target: "adjustmentLayer" })).toBe(true);
  });

  it("a histogram whose copy out fails closes the panel and says why, rather than reading forever (final review minor 5)", () => {
    const { previews, requests } = install(2001, 2000);
    useEditor.setState({ engine: { ...useEditor.getState().engine!, jobInput: () => { throw new RangeError("Array buffer allocation failed"); } } as never });
    let opened: boolean | undefined;
    expect(() => { opened = useEditor.getState().beginAdjust({ kind: "Levels" }); }, "nothing thrown out of the key handler").not.toThrow();
    expect(opened).toBe(false);
    expect([useEditor.getState().adjustEdit, useEditor.getState().error, requests]).toEqual([null, "Array buffer allocation failed", []]);
    expect(previews.at(-1), "its preview taken away").toBeNull();
  });

  it("Save and the close prompt's OK say why they wait while working, and the document stays open (final review minor 4)", async () => {
    install(2001, 2000);
    const writes: string[] = [];
    const bridge = { baseName: (p: string) => p, writePackage: async (p: string) => { writes.push(p); }, addRecentPackage: async () => {}, pickSavePackage: async () => "C:/x.comp" } as unknown as ShellBridge;
    const modified = { ...document(layer("A", 2001, 2000)), isModified: true, path: "C:/a.comp" };
    useEditor.setState({ bridge, busy: false, documents: { D: modified }, working: true });
    vi.stubGlobal("window", { confirm: () => true });
    try {
      await saveProject();
      expect([writes, useEditor.getState().error]).toEqual([[], BUSY_MESSAGE]);
      useEditor.setState({ error: null });
      expect(await closeProject("D")).toBe(false);
      expect([writes, useEditor.getState().error, Object.keys(useEditor.getState().documents)]).toEqual([[], BUSY_MESSAGE, ["D"]]);
    } finally { vi.unstubAllGlobals(); }
  });

  it("OK while working keeps the panel open and shows BUSY_MESSAGE, rather than losing the edit", async () => {
    // A layer at the threshold, so its own OK would run on the UI thread -- `working` here stands
    // in for some other job (an adjustment layer's, or another document's) still in flight.
    const { log, requests } = install(2000, 2000);
    levelsChanged();
    const edit = useEditor.getState().adjustEdit;
    useEditor.setState({ working: true });
    useEditor.getState().commitAdjust();
    expect(useEditor.getState().adjustEdit, "the panel stays open").toEqual(edit);
    expect(useEditor.getState().error).toBe(BUSY_MESSAGE);
    expect(log.filter((l) => l.startsWith("execute"))).toEqual([]);
    expect(requests.filter((r) => r.kind === "edit")).toEqual([]);
  });
});
