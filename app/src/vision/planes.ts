export interface Plane {
  width: number;
  height: number;
  values: Float32Array;
}

export function resizePlane(
  source: Plane,
  width: number,
  height: number,
): Plane {
  const values = new Float32Array(width * height);
  for (let y = 0; y < height; y++) {
    const sy = Math.max(
      0,
      Math.min(source.height - 1, ((y + 0.5) * source.height) / height - 0.5),
    );
    const y0 = Math.floor(sy),
      y1 = Math.min(y0 + 1, source.height - 1),
      fy = sy - y0;
    for (let x = 0; x < width; x++) {
      const sx = Math.max(
        0,
        Math.min(source.width - 1, ((x + 0.5) * source.width) / width - 0.5),
      );
      const x0 = Math.floor(sx),
        x1 = Math.min(x0 + 1, source.width - 1),
        fx = sx - x0;
      const top =
        source.values[y0 * source.width + x0] * (1 - fx) +
        source.values[y0 * source.width + x1] * fx;
      const bottom =
        source.values[y1 * source.width + x0] * (1 - fx) +
        source.values[y1 * source.width + x1] * fx;
      values[y * width + x] = top * (1 - fy) + bottom * fy;
    }
  }
  return { width, height, values };
}

/** A box mean on the image domain, including partial windows at its edges. */
export function boxMean(
  data: Float32Array,
  w: number,
  h: number,
  radius: number,
): Float32Array {
  const pitch = w + 1,
    table = new Float64Array(pitch * (h + 1));
  for (let y = 0; y < h; y++) {
    let row = 0;
    for (let x = 0; x < w; x++) {
      row += data[y * w + x];
      table[(y + 1) * pitch + x + 1] = table[y * pitch + x + 1] + row;
    }
  }
  const mean = new Float32Array(w * h);
  for (let y = 0; y < h; y++) {
    const y0 = Math.max(0, y - radius),
      y1 = Math.min(h, y + radius + 1);
    for (let x = 0; x < w; x++) {
      const x0 = Math.max(0, x - radius),
        x1 = Math.min(w, x + radius + 1);
      mean[y * w + x] =
        (table[y1 * pitch + x1] -
          table[y0 * pitch + x1] -
          table[y1 * pitch + x0] +
          table[y0 * pitch + x0]) /
        ((x1 - x0) * (y1 - y0));
    }
  }
  return mean;
}

export function guided(
  mask: Plane,
  guide: Float32Array,
  radius: number,
  epsilon = 1e-3,
): Plane {
  const { width: w, height: h, values: p } = mask;
  const mean = (input: Float32Array) => boxMean(input, w, h, radius);
  const gi = mean(guide),
    pi = mean(p);
  const ii = mean(guide.map((v) => v * v)),
    ip = mean(guide.map((v, i) => v * p[i]));
  const a = ip.map(
    (v, i) =>
      (v - gi[i] * pi[i]) / (Math.max(0, ii[i] - gi[i] * gi[i]) + epsilon),
  );
  const b = mean(pi.map((v, i) => v - a[i] * gi[i])),
    aa = mean(a);
  return {
    width: w,
    height: h,
    values: b.map((v, i) => Math.max(0, Math.min(1, v + aa[i] * guide[i]))),
  };
}

export function nearestDepthShape(w: number, h: number): [number, number] {
  const shapes: [number, number][] = [
    [512, 512],
    [512, 384],
    [512, 288],
    [384, 512],
    [288, 512],
  ];
  const distance = ([x, y]: [number, number]) =>
    Math.abs(Math.log(w / h) - Math.log(x / y));
  return shapes.reduce((a, b) => (distance(a) <= distance(b) ? a : b));
}

export function normalize(values: Float32Array): Float32Array {
  let low = Infinity,
    high = -Infinity;
  for (const v of values) {
    if (!Number.isFinite(v))
      throw new Error("The depth model returned invalid pixels.");
    low = Math.min(low, v);
    high = Math.max(high, v);
  }
  if (high <= low) return new Float32Array(values.length);
  return values.map((v) => (v - low) / (high - low));
}
