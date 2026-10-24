// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, StrictMode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { CanvasView } from "../../src/canvas/CanvasView";
import { createRenderer } from "../../src/canvas/renderer";
import { useEditor } from "../../src/state/store";
import type { EngineClient } from "../../src/engine/client";

vi.mock("../../src/canvas/renderer", () => ({ createRenderer: vi.fn() }));
(globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;
let root: Root | null = null;
let host: HTMLDivElement;
const resources: { dispose: ReturnType<typeof vi.fn>; clear: ReturnType<typeof vi.fn> }[] = [];
const originalState = useEditor.getState();
const engine = () => ({} as EngineClient);

beforeEach(() => {
  resources.length = 0;
  vi.mocked(createRenderer).mockImplementation(() => {
    const resource = { kind: "gl" as const, render: vi.fn(), clear: vi.fn(), readPixels: vi.fn(), dispose: vi.fn() };
    resources.push(resource);
    return resource;
  });
  vi.stubGlobal("ResizeObserver", class { observe() {} disconnect() {} });
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue({
    save() {}, setTransform() {}, clearRect() {}, restore() {},
  } as unknown as CanvasRenderingContext2D);
  useEditor.setState({ ...originalState, engine: engine(), activeId: null, documents: {}, order: [] }, true);
  host = document.createElement("div"); document.body.appendChild(host);
  root = createRoot(host);
});
afterEach(async () => {
  await act(async () => { root?.unmount(); });
  root = null; host.remove();
  useEditor.setState(originalState, true);
  vi.restoreAllMocks(); vi.unstubAllGlobals(); vi.mocked(createRenderer).mockReset();
});

describe("CanvasView renderer ownership", () => {
  it("reuses its renderer through StrictMode effect replay and releases it once on real unmount", async () => {
    await act(async () => { root!.render(<StrictMode><CanvasView /></StrictMode>); });
    expect(createRenderer).toHaveBeenCalledTimes(1);
    expect(resources[0].dispose).not.toHaveBeenCalled();
    expect(resources[0].clear).toHaveBeenCalled();
    await act(async () => { root!.unmount(); root = null; });
    expect(resources[0].dispose).toHaveBeenCalledTimes(1);
  });

  it("releases an old engine's renderer before constructing its replacement and ignores stale cleanup", async () => {
    await act(async () => { root!.render(<StrictMode><CanvasView /></StrictMode>); });
    const previous = resources[0];
    vi.mocked(createRenderer).mockImplementationOnce(() => {
      expect(previous.dispose).toHaveBeenCalledTimes(1);
      const replacement = { kind: "gl" as const, render: vi.fn(), clear: vi.fn(), readPixels: vi.fn(), dispose: vi.fn() };
      resources.push(replacement); return replacement;
    });
    await act(async () => { useEditor.setState({ engine: engine() }); });
    expect(createRenderer).toHaveBeenCalledTimes(2);
    expect(resources[1].dispose).not.toHaveBeenCalled();
    await act(async () => { root!.unmount(); root = null; });
    expect(previous.dispose).toHaveBeenCalledTimes(1);
    expect(resources[1].dispose).toHaveBeenCalledTimes(1);
  });
});
