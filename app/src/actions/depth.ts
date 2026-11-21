import { useEditor } from "../state/store";
import { activeLayer } from "../state/selection";
import type { DepthInput, Reply } from "../vision/depth-worker";
import { resizePlane } from "../vision/planes";

export function lightingSource() {
  const { activeId, engine, documents, maskSelected } = useEditor.getState();
  if (!activeId || !engine || maskSelected)
    throw new Error("Choose a pixel layer to light.");
  const layer = activeLayer(documents[activeId]);
  if (!layer?.hasPixels || layer.isGroup)
    throw new Error("Choose a pixel layer to light.");
  let width = layer.pixelsWidth,
    height = layer.pixelsHeight,
    level = 0;
  while (
    width > 1 &&
    height > 1 &&
    Math.max(width, height) > 1280 &&
    level < 16
  ) {
    width = Math.max(1, Math.floor(width / 2));
    height = Math.max(1, Math.floor(height / 2));
    level++;
  }
  const source = engine.layerPixels(activeId, layer.id, level);
  if (!source || source.length !== width * height * 4)
    throw new Error("Could not read the source layer.");
  const scale = Math.min(1, 640 / Math.max(width, height));
  const w = Math.max(1, Math.round(width * scale)),
    h = Math.max(1, Math.round(height * scale));
  let pixels: Uint8Array;
  if (w === width && h === height) pixels = new Uint8Array(source);
  else {
    pixels = new Uint8Array(w * h * 4);
    for (let c = 0; c < 4; c++) {
      const values = Float32Array.from(
        { length: width * height },
        (_, i) => source[4 * i + c],
      );
      const resized = resizePlane({ width, height, values }, w, h).values;
      for (let i = 0; i < resized.length; i++)
        pixels[4 * i + c] = Math.round(resized[i]);
    }
  }
  return {
    doc: activeId,
    layer,
    engine,
    input: { pixels, width: w, height: h } satisfies DepthInput,
  };
}
export function visionWorker() {
  return new Worker(new URL("../vision/depth-worker.ts", import.meta.url), {
    type: "module",
  });
}
export async function extractDepth(): Promise<void> {
  const state = useEditor.getState();
  if (state.working || state.busy || state.panelOwnsDocument()) return;
  state.commitTransform();
  state.commitGradient();
  let worker: Worker | undefined;
  useEditor.setState({ working: true });
  try {
    const { doc, layer, engine, input } = lightingSource();
    const current = (worker = visionWorker());
    const map = await new Promise<Extract<Reply, { kind: "map" }>>(
      (resolve, reject) => {
        current.onerror = (e) =>
          reject(new Error(e.message || "Depth estimation could not start."));
        current.onmessage = (e: MessageEvent<Reply>) => {
          if (e.data.kind === "error") reject(new Error(e.data.error));
          if (e.data.kind === "map") resolve(e.data);
        };
        current.postMessage({ kind: "depth", input }, [input.pixels.buffer]);
      },
    );
    if (useEditor.getState().activeId !== doc) return;
    engine.insertDepthMap(
      doc,
      layer.id,
      layer.pixelsRevision,
      map.width,
      map.height,
      map.values,
    );
    state.refresh(doc);
    state.revealActiveLayer();
  } catch (error) {
    state.setError(error instanceof Error ? error.message : String(error));
  } finally {
    worker?.terminate();
    useEditor.setState({ working: false });
  }
}
