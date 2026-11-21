import * as ort from "onnxruntime-web/wasm";
import wasm from "../../../node_modules/onnxruntime-web/dist/ort-wasm-simd-threaded.wasm?url";
import mjs from "../../../node_modules/onnxruntime-web/dist/ort-wasm-simd-threaded.mjs?url";
import depthModel from "./depthmini.onnx?url";
import subjectModel from "./u2netp.onnx?url";
import {
  guided,
  nearestDepthShape,
  normalize,
  resizePlane,
  type Plane,
} from "./planes";
import {
  composedDepth,
  makeScene,
  renderLight,
  type Scene,
  type LightSettings,
} from "./lighting";

ort.env.wasm.numThreads = 1;
ort.env.wasm.proxy = false;
ort.env.wasm.wasmPaths = {
  wasm: new URL(wasm, self.location.href).href,
  mjs: new URL(mjs, self.location.href).href,
};

export interface DepthInput {
  pixels: Uint8Array;
  width: number;
  height: number;
}
export interface DepthOutput {
  values: Uint8Array;
  width: number;
  height: number;
}
export type Request =
  | { kind: "depth"; input: DepthInput }
  | { kind: "prepare"; input: DepthInput; depth: boolean }
  | { kind: "render"; id: number; settings: LightSettings }
  | { kind: "map"; contrast: number; keep: boolean };
export type Reply =
  | { kind: "ready"; center: [number, number] }
  | ({ kind: "map" } & DepthOutput)
  | {
      kind: "render";
      id: number;
      width: number;
      height: number;
      field: Float32Array;
      preview: Uint8ClampedArray;
    }
  | { kind: "error"; error: string };

async function predict(
  url: string,
  data: Float32Array,
  w: number,
  h: number,
  firstOutput?: string,
) {
  const session = await ort.InferenceSession.create(
    new URL(url, self.location.href).href,
    { executionProviders: ["wasm"] },
  );
  const input = new ort.Tensor("float32", data, [1, 3, h, w]);
  try {
    const name = firstOutput ?? session.outputNames[0];
    const result = await session.run({ [session.inputNames[0]]: input }, [
      name,
    ]);
    const tensor = result[name];
    try {
      const output = tensor.data;
      if (!(output instanceof Float32Array) || output.length !== w * h)
        throw new Error("Unexpected model output size.");
      return output.slice();
    } finally {
      tensor.dispose();
    }
  } finally {
    input.dispose();
    await session.release();
  }
}
function stack(planes: Plane[], w: number, h: number) {
  const input = new Float32Array(3 * w * h);
  planes.forEach((p, i) => input.set(resizePlane(p, w, h).values, i * w * h));
  return input;
}
async function subject(planes: Plane[], input: DepthInput, gray: Float32Array) {
  const { pixels, width, height } = input,
    alpha = gray.map((_, i) => pixels[4 * i + 3] / 255);
  // A cut-out already has a matte at source resolution.
  if (alpha.some((v) => v < 0.995)) return alpha;
  const data = stack(planes, 320, 320),
    stride = 320 * 320;
  let scale = 1;
  for (const v of data) scale = Math.max(scale, v);
  const means = [0.485, 0.456, 0.406],
    std = [0.229, 0.224, 0.225];
  for (let c = 0; c < 3; c++)
    for (let i = 0; i < stride; i++)
      data[c * stride + i] = (data[c * stride + i] / scale - means[c]) / std[c];
  const output = await predict(subjectModel, data, 320, 320);
  if (!output.some((v) => v > 0.5)) return new Float32Array(width * height);
  const mask = resizePlane(
    { width: 320, height: 320, values: normalize(output) },
    width,
    height,
  );
  return guided(
    mask,
    gray,
    Math.max(2, Math.round(Math.max(width, height) * 0.008)),
  ).values;
}

async function prepare(input: DepthInput, needsDepth: boolean): Promise<Scene> {
  const { width, height, pixels } = input;
  const planes = [0, 1, 2].map((c) => ({
    width,
    height,
    values: Float32Array.from({ length: width * height }, (_, i) =>
      pixels[i * 4 + 3]
        ? Math.min(255, (255 * pixels[i * 4 + c]) / pixels[i * 4 + 3])
        : 0,
    ),
  }));
  const gray = planes[0].values.map(
    (v, i) =>
      (0.2126 * v +
        0.7152 * planes[1].values[i] +
        0.0722 * planes[2].values[i]) /
      255,
  );
  const mask = await subject(planes, input, gray);
  let depth = new Float32Array(width * height);
  if (needsDepth) {
    const [w, h] = nearestDepthShape(width, height);
    const values = await predict(
      depthModel,
      stack(planes, w, h),
      w,
      h,
      "depth",
    );
    const expanded = resizePlane(
      { width: w, height: h, values: normalize(values) },
      width,
      height,
    );
    depth.set(
      guided(
        expanded,
        gray,
        Math.max(1, Math.round((13 * Math.max(width, height)) / 640)),
      ).values,
    );
  }
  return makeScene(pixels, width, height, mask, depth);
}
let scene: Scene | undefined;
self.onmessage = async (event: MessageEvent<Request>) => {
  try {
    const msg = event.data;
    if (msg.kind === "depth" || msg.kind === "prepare") {
      scene = await prepare(msg.input, msg.kind === "depth" || msg.depth);
      if (msg.kind === "prepare") {
        self.postMessage({
          kind: "ready",
          center: scene.center,
        } satisfies Reply);
        return;
      }
    }
    if (!scene) throw new Error("The lighting source is missing.");
    const { width, height } = scene;
    if (msg.kind === "render") {
      const { preview, field } = renderLight(scene, msg.settings);
      self.postMessage(
        {
          kind: "render",
          id: msg.id,
          width,
          height,
          preview,
          field,
        } satisfies Reply,
        { transfer: [preview.buffer, field.buffer] },
      );
    } else {
      const map = composedDepth(
        scene,
        msg.kind === "map" ? msg.contrast : 0,
        msg.kind === "map" ? msg.keep : true,
      );
      const values = Uint8Array.from(map, (v) => Math.round(255 * v));
      self.postMessage({ kind: "map", width, height, values } satisfies Reply, {
        transfer: [values.buffer],
      });
    }
  } catch (error) {
    self.postMessage({
      kind: "error",
      error: error instanceof Error ? error.message : String(error),
    } satisfies Reply);
  }
};
