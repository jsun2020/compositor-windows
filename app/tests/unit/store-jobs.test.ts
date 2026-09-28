import { beforeEach, describe, expect, it } from "vitest";
import { BUSY_MESSAGE, JOB_PIXELS, useEditor } from "../../src/state/store";
import { defaultAdjustment } from "../../src/state/adjust-edit";
import type { Command, DocumentState, LayerAdjustment, LayerState, PreviewRequest } from "../../src/engine/types";
import type { EngineClient } from "../../src/engine/client";
import type { JobClient, JobRequest, JobResult } from "../../src/engine/jobs";

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
  let finish: (r: JobResult | null) => void = () => {};
  const engine = {
    state: () => document(layer("A", width, height)),
    execute: (_id: string, cmd: Command) => { log.push(`execute ${cmd.type}`); return { structure: true, canvas: false, layers: [] }; },
    setPreview: (_id: string, request: PreviewRequest | null) => { previews.push(request); return { structure: true, canvas: false, layers: [] }; },
    histogram: () => { log.push("histogram here"); return bins(); },
    // Task 5's jobs API: a job's input carries its selection's points as a separate buffer (null
    // here -- these fixtures have no selection), beside the JSON and the pixel/mask buffers.
    jobInput: () => { log.push("job input"); return { input: '{"stamp":{"pixelsRevision":1}}', pixels: new ArrayBuffer(4), mask: null, points: null }; },
    installJob: (_doc: string, layerId: string, input: string, output: string) => { log.push(`install ${layerId} ${output}`); onInstall?.(); expect(input).toContain("stamp"); return { structure: true, canvas: false, layers: [] }; },
    undo: () => { log.push("undo"); return { structure: true, canvas: false, layers: [] }; },
    adjustmentIsIdentity: (a: LayerAdjustment) => JSON.stringify(a) === JSON.stringify(defaultAdjustment(a.kind)),
  } as unknown as EngineClient;
  const jobs = { run: (_channel: string, request: JobRequest) => { requests.push(request); return new Promise<JobResult | null>((resolve) => { finish = resolve; }); } } as unknown as JobClient;
  useEditor.setState({ engine, jobs, jobPixels: JOB_PIXELS, activeId: "D", documents: { D: document(layer("A", width, height)) }, order: ["D"], selectedLayerIds: ["A"],
    maskSelected: false, transformEdit: null, adjustEdit: null, error: null, tool: "move", cropRect: null, sheet: null, working: false });
  return { log, previews, requests, finish: (r: JobResult | null) => finish(r) };
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
});
