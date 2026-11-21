import { boxMean, resizePlane } from "./planes";

export interface RimLamp {
  angle: number;
  color: string;
  strength: number;
  width: number;
  choke: number;
}
export interface RimSettings {
  kind: "rim";
  lights: RimLamp[];
  glow: number;
}
export interface VolumeSettings {
  kind: "volume";
  center: [number, number] | null;
  color: string;
  intensity: number;
  length: number;
  haze: number;
  size: number;
  shafts: number;
  distance: number;
  depthContrast: number;
  keepsSubjectClear: boolean;
}
export type LightSettings = RimSettings | VolumeSettings;
export const firstLamp: RimLamp = {
  angle: 135,
  color: "#ffdbb3",
  strength: 50,
  width: 30,
  choke: 0,
};
export const defaultVolume: VolumeSettings = {
  kind: "volume",
  center: null,
  color: "#ffdbad",
  intensity: 30,
  length: 0,
  haze: 60,
  size: 20,
  shafts: 0,
  distance: 30,
  depthContrast: 0,
  keepsSubjectClear: true,
};
export interface Scene {
  width: number;
  height: number;
  pixels: Uint8Array;
  channels: Float32Array[];
  gray: Float32Array;
  mask: Float32Array;
  depth: Float32Array;
  center: [number, number];
}
const clamp = (v: number, lo = 0, hi = 1) => Math.min(hi, Math.max(lo, v));
const mix = (a: number, b: number, t: number) => a + (b - a) * t;
function smooth(a: number, b: number, x: number) {
  const t = clamp((x - a) / (b - a));
  return t * t * (3 - 2 * t);
}
function linear(v: number) {
  return v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
}
function tint(hex: string) {
  return [1, 3, 5].map((i) => linear(parseInt(hex.slice(i, i + 2), 16) / 255));
}

// Three box passes matching the variance of the requested Gaussian, bounded at the image edges.
function blur(
  p: Float32Array,
  w: number,
  h: number,
  sigma: number,
): Float32Array {
  if (sigma < 0.2) return p;
  const ideal = Math.sqrt(4 * sigma * sigma + 1),
    low = Math.max(1, Math.floor((ideal - 1) / 2) * 2 + 1);
  const small = Math.round(
    (3 * low * low + 12 * low + 9 - 12 * sigma * sigma) / (4 * low + 4),
  );
  let result = p;
  for (let i = 0; i < 3; i++) {
    const radius = (low - 1) / 2 + (i >= small ? 1 : 0);
    if (radius > 0) result = boxMean(result, w, h, radius);
  }
  return result;
}
export function brightest(
  gray: Float32Array,
  width: number,
  height: number,
): [number, number] {
  const scale = Math.min(1, 64 / Math.max(width, height)),
    w = Math.max(1, Math.round(width * scale)),
    h = Math.max(1, Math.round(height * scale));
  const p = boxMean(
    resizePlane({ width, height, values: gray }, w, h).values,
    w,
    h,
    1,
  );
  const peak = Math.max(...p);
  if (peak <= 0) return [0.5, 0.5];
  const seen = new Uint8Array(p.length);
  let best = -1,
    at = 0;
  for (let i = 0; i < p.length; i++) {
    if (seen[i] || p[i] < peak * 0.75) continue;
    let score = 0,
      hot = i;
    const stack = [i];
    seen[i] = 1;
    while (stack.length) {
      const j = stack.pop()!;
      score += p[j];
      if (p[j] > p[hot]) hot = j;
      const x = j % w,
        y = Math.floor(j / w);
      for (const n of [
        x > 0 ? j - 1 : -1,
        x + 1 < w ? j + 1 : -1,
        y > 0 ? j - w : -1,
        y + 1 < h ? j + w : -1,
      ]) {
        if (n >= 0 && !seen[n] && p[n] >= peak * 0.75) {
          seen[n] = 1;
          stack.push(n);
        }
      }
    }
    if (score > best) {
      best = score;
      at = hot;
    }
  }
  return [((at % w) + 0.5) / w, (Math.floor(at / w) + 0.5) / h];
}
export function makeScene(
  pixels: Uint8Array,
  width: number,
  height: number,
  mask: Float32Array,
  depth: Float32Array,
): Scene {
  const channels = [0, 1, 2].map((c) =>
    Float32Array.from(mask, (_, i) =>
      pixels[i * 4 + 3] ? clamp(pixels[i * 4 + c] / pixels[i * 4 + 3]) : 0,
    ),
  );
  const gray = mask.map(
    (_, i) =>
      (0.2126 * pixels[4 * i] +
        0.7152 * pixels[4 * i + 1] +
        0.0722 * pixels[4 * i + 2]) /
      255,
  );
  return {
    width,
    height,
    pixels,
    channels,
    gray,
    mask,
    depth,
    center: brightest(gray, width, height),
  };
}

function spreadDepth(p: Float32Array, amount: number) {
  if (amount === 0 || p.length < 2) return p;
  const order = Uint32Array.from(p, (_, i) => i).sort((a, b) => p[a] - p[b]);
  const adjusted = new Float32Array(p.length);
  order.forEach((index, rank) => {
    adjusted[index] = mix(p[index], rank / (p.length - 1), amount / 100);
  });
  return adjusted;
}
function compose(s: Scene, p: Float32Array, keep: boolean) {
  const { width: w, height: h, mask } = s,
    side = Math.max(w, h),
    reach = side * 0.02;
  const soft = (a: Float32Array, r: number) => blur(a, w, h, r);
  const depth = soft(p, side * 0.0015);
  const beyond = soft(mask, reach).map((v) => clamp(1 - 6 * v));
  const local = (weights: Float32Array, r: number) => {
    const total = soft(weights, r),
      weighted = soft(
        depth.map((v, i) => v * weights[i]),
        r,
      );
    return weighted.map((v, i) => clamp(v / (total[i] + 0.001)));
  };
  const behind = local(beyond, reach * 2.5),
    near = soft(mask, reach * 1.5);
  const inside = keep
    ? depth.map((v) => Math.max(0.9, v))
    : local(mask, side * 0.08);
  return depth.map((v, i) =>
    mix(mix(v, behind[i], clamp(4 * near[i])), inside[i], mask[i]),
  );
}
export function composedDepth(
  s: Scene,
  contrast: number,
  keepsSubjectClear: boolean,
): Float32Array {
  return compose(s, spreadDepth(s.depth, contrast), keepsSubjectClear);
}
function inverseError(x: number) {
  // Winitzki approximation on the bounded interval used for the blurred outline.
  const l = Math.log(1 - x * x),
    a = 2 / (Math.PI * 0.147) + l / 2;
  return (
    Math.sign(x) * Math.sqrt(Math.max(0, Math.sqrt(a * a - l / 0.147) - a))
  );
}
function rimField(s: Scene, settings: RimSettings, field: Float32Array) {
  const { width: w, height: h, mask, gray, channels } = s,
    side = Math.max(w, h);
  const soft = (p: Float32Array, r: number) => blur(p, w, h, r);
  const outline = mask.map((v) => clamp(1.6 * v - 0.3)),
    near = soft(outline, side * 0.015);
  const fine = soft(gray, 1),
    coarse = soft(gray, Math.max(2, side / 400)),
    detail = fine.map((v, i) => v - coarse[i]);
  const open = outline.map((v) => 1 - v),
    weight = soft(open, side * 0.01);
  const differs = new Float32Array(mask.length),
    varies = new Float32Array(mask.length);
  for (const channel of channels) {
    const mean = soft(
      channel.map((v, i) => v * open[i]),
      side * 0.01,
    );
    const squares = soft(
      channel.map((v, i) => v * v * open[i]),
      side * 0.01,
    );
    for (let i = 0; i < mask.length; i++) {
      const background = mean[i] / (weight[i] + 0.001);
      differs[i] = Math.max(differs[i], Math.abs(channel[i] - background));
      varies[i] = Math.max(
        varies[i],
        Math.sqrt(
          Math.abs(squares[i] / (weight[i] + 0.001) - background * background),
        ),
      );
    }
  }
  const rims = channels.map(() => new Float32Array(gray.length));
  for (const lamp of settings.lights) {
    if (!lamp.strength) continue;
    const width = side * (0.002 + 0.0003 * lamp.width),
      reach = Math.max(1, width / 1.5),
      edge = soft(outline, reach);
    const rad = (lamp.angle * Math.PI) / 180,
      dx = Math.cos(rad),
      dy = Math.sin(rad),
      choke = lamp.choke / 100;
    const color = tint(lamp.color).map((v) => v * lamp.strength * 0.1);
    for (let y = 0; y < h; y++)
      for (let x = 0; x < w; x++) {
        const i = y * w + x;
        if (mask[i] <= 0.002 && near[i] <= 0.002) continue;
        const left = y * w + Math.max(0, x - 1),
          right = y * w + Math.min(w - 1, x + 1),
          top = Math.max(0, y - 1) * w + x,
          bottom = Math.min(h - 1, y + 1) * w + x;
        const faces = (p: Float32Array, bumps: boolean) => {
          const gx = p[right] - p[left],
            gy = p[top] - p[bottom],
            size = Math.hypot(gx, gy);
          const nx =
            (size > 1e-6 ? (-2.75 * gx) / size : 0) -
            (bumps ? ((detail[right] - detail[left]) * side) / 500 : 0);
          const ny =
            (size > 1e-6 ? (-2.75 * gy) / size : 0) -
            (bumps ? ((detail[top] - detail[bottom]) * side) / 500 : 0);
          return (
            Math.max(
              0,
              (nx * dx + ny * dy - 0.2) /
                (Math.hypot(nx, ny, 1) * Math.sqrt(1.04)),
            ) ** 1.6
          );
        };
        const depth =
            reach *
            Math.SQRT2 *
            inverseError(2 * clamp(edge[i], 0.001, 0.999) - 1),
          t = Math.max(0, depth) / width;
        const held = 1 - smooth(Math.max(0, (choke - 0.3) / 0.7) * 0.95, 1, t);
        const falloff =
          mix(Math.exp(-2.2 * t), held, Math.min(1, choke / 0.3)) *
          (1 - smooth(1.5, 1.9, t));
        const inside = smooth(-1.5, 1.5, depth);
        const strands =
          clamp((differs[i] - 0.03 - 2.5 * varies[i]) / 0.1) *
          smooth(0, 0.1, near[i]) *
          (1 - inside);
        const lit =
          faces(edge, true) *
            falloff *
            clamp(1 + detail[i] * 6, 0.3, 1.8) *
            mask[i] *
            inside +
          (strands ? faces(near, false) * strands : 0);
        for (let c = 0; c < 3; c++)
          rims[c][i] += color[c] * (channels[c][i] * 0.7 + 0.45) * lit;
      }
  }
  const cover = soft(outline, side * 0.008),
    glow = settings.glow * 0.04;
  rims.forEach((rim, c) => {
    const halo = glow ? soft(rim, side * 0.012) : rim,
      haze = glow ? soft(rim, side * 0.035) : rim;
    for (let i = 0; i < rim.length; i++)
      field[7 * i + 4 + c] = Math.exp(
        -rim[i] - glow * (0.6 * halo[i] + haze[i]) * clamp(2 - 2 * cover[i]),
      );
  });
}
function hash(n: number) {
  const v = Math.sin(n * 12.9898) * 43758.5453;
  return v - Math.floor(v);
}
function around(angle: number, cells: number, seed: number) {
  const u = (angle / (2 * Math.PI) + 0.5) * cells,
    i = Math.floor(u);
  return mix(
    hash((i % cells) + seed),
    hash(((i + 1) % cells) + seed),
    smooth(0, 1, u - i),
  );
}
function volumeField(s: Scene, v: VolumeSettings, field: Float32Array) {
  const { width: w, height: h, channels, mask } = s,
    side = Math.max(w, h);
  const position = v.center ?? s.center,
    lx = position[0] * w - 0.5,
    ly = position[1] * h - 0.5;
  const color = tint(v.color),
    size = Math.max(1, side * (0.02 + 0.004 * v.size)),
    haze = v.haze / 100;
  const depths = spreadDepth(s.depth, v.depthContrast),
    far = compose(s, depths, v.keepsSubjectClear);
  const ordered = depths.map((x) => 1 - x).sort();
  const quantile = (share: number) =>
    ordered[Math.floor(clamp(share) * (ordered.length - 1))];
  const start = Math.min(0.9, quantile(v.distance / 100)),
    end = Math.max(start + 0.45, quantile(v.distance / 100 + 0.45));
  const near = blur(mask, w, h, Math.max(1, side * 0.003));
  const bright = mask.map(
    (_, i) =>
      0.2126 * channels[0][i] +
      0.7152 * channels[1][i] +
      0.0722 * channels[2][i],
  );
  let peak = 0.05;
  for (const value of bright) peak = Math.max(peak, value);
  const emission = bright.map((value, i) => {
    const x = (i % w) - lx,
      y = Math.floor(i / w) - ly,
      dist = Math.hypot(x, y),
      l = value / peak,
      t = Math.atan2(y, x);
    const broken = v.shafts
      ? mix(
          1,
          (0.6 * around(t, 97, 3.1) + 0.4 * around(t, 251, 7.7)) ** 2 * 1.8,
          v.shafts / 100,
        )
      : 1;
    return (
      (Math.exp(-2 * (dist / size) ** 2) *
        (0.25 + 0.75 * clamp((l - 0.08) / 0.3)) +
        clamp((l - 0.45) / 0.45) * Math.exp(-dist / (side * 0.5))) *
      (1 - mask[i]) *
      broken
    );
  });
  const sample = (x: number, y: number) => {
    x = clamp(x, 0, w - 1);
    y = clamp(y, 0, h - 1);
    const left = Math.floor(x),
      top = Math.floor(y),
      right = Math.min(w - 1, left + 1),
      bottom = Math.min(h - 1, top + 1);
    return mix(
      mix(emission[top * w + left], emission[top * w + right], x - left),
      mix(emission[bottom * w + left], emission[bottom * w + right], x - left),
      y - top,
    );
  };
  const air = channels.map((c) => blur(c, w, h, side * 0.12));
  const bloom = channels.map((c) => {
    const local = blur(c, w, h, side * 0.04),
      high = c.map((a, i) => Math.max(0, a - Math.max(0.5, 1.5 * local[i])));
    const halo = blur(high, w, h, side * 0.012),
      wide = blur(high, w, h, side * 0.05);
    return halo.map((a, i) => a + 2 * wide[i]);
  });
  for (let y = 0; y < h; y++)
    for (let x = 0; x < w; x++) {
      const i = y * w + x,
        ramp = smooth(start, end, 1 - far[i]),
        fog = 1 - Math.exp(-3 * haze * (0.12 + 0.88 * ramp));
      const glow = Math.exp(-Math.hypot(x - lx, y - ly) / (side * 0.35));
      let beams = 0;
      if (v.intensity > 0) {
        if (v.length === 0)
          beams = (emission[i] * (1 - 0.985 ** 96)) / (1 - 0.985);
        else {
          const dx = ((lx - x) * v.length) / 9600,
            dy = ((ly - y) * v.length) / 9600,
            j = hash(x * 0.618 + y * 1.414);
          let weight = 1;
          for (let k = 0; k < 96; k++) {
            beams += sample(x + (k + j) * dx, y + (k + j) * dy) * weight;
            weight *= 0.985;
          }
        }
      }
      field[i * 7] = fog;
      for (let c = 0; c < 3; c++) {
        field[i * 7 + 1 + c] =
          fog *
            (air[c][i] * mix(1, color[c], 0.6) * 0.9 + color[c] * glow * 0.45) +
          bloom[c][i] * haze * 1.6 * (0.3 + 0.7 * ramp);
        field[i * 7 + 4 + c] = Math.exp(
          ((-color[c] * v.intensity * 0.05 * beams) / 48) *
            (1 - 0.92 * near[i]),
        );
      }
    }
}
export function renderLight(s: Scene, settings: LightSettings) {
  const field = new Float32Array(s.width * s.height * 7);
  if (settings.kind === "rim") rimField(s, settings, field);
  else volumeField(s, settings, field);
  const preview = new Uint8ClampedArray(s.pixels.length);
  for (let i = 0; i < s.mask.length; i++) {
    preview[i * 4 + 3] = s.pixels[i * 4 + 3];
    for (let c = 0; c < 3; c++) {
      const veiled =
        s.channels[c][i] * (1 - field[7 * i]) + field[7 * i + 1 + c];
      preview[4 * i + c] =
        255 *
        (veiled >= 1 ? veiled : 1 - (1 - veiled) * field[7 * i + 4 + c]);
    }
  }
  return { field, preview };
}
