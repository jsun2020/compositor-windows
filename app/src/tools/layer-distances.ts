import type { RectLike } from "../canvas/viewport";

export interface DistanceLine {
  horizontal: boolean;
  start: number;
  end: number;
  cross: number;
}
const axes = (r: RectLike, h: boolean) =>
  h
    ? [r.x, r.x + r.width, r.y, r.y + r.height]
    : [r.y, r.y + r.height, r.x, r.x + r.width];

export function layerDistances(
  box: RectLike,
  others: RectLike[],
  canvas: RectLike,
  hovered?: RectLike | null,
): DistanceLine[] {
  const gaps: DistanceLine[] = [],
    insets: DistanceLine[] = [];
  for (const horizontal of [true, false]) {
    const [lo, hi, a, b] = axes(box, horizontal);
    const line = (start: number, end: number, cross: number) => ({
      horizontal,
      start,
      end,
      cross,
    });
    const middle = (near: number, far: number) =>
      Math.min(b, far) > Math.max(a, near)
        ? (Math.max(a, near) + Math.min(b, far)) / 2
        : (a + b) / 2;
    if (hovered !== undefined) {
      const [l, r, n, f] = axes(hovered ?? canvas, horizontal),
        cross = middle(n, f);
      if (l >= hi) gaps.push(line(hi, l, cross));
      else if (r <= lo) gaps.push(line(r, lo, cross));
      else
        for (const [x, y] of [
          [lo, l],
          [hi, r],
        ])
          if (Math.abs(x - y) >= 0.5)
            insets.push(line(Math.min(x, y), Math.max(x, y), cross));
    } else {
      let before: DistanceLine | undefined, after: DistanceLine | undefined;
      for (const other of [...others, canvas]) {
        const [l, r, n, f] = axes(other, horizontal);
        if (n >= b || f <= a) continue;
        for (const edge of [l, r]) {
          if (edge <= lo && (!before || edge > before.start))
            before = line(edge, lo, middle(n, f));
          if (edge >= hi && (!after || edge < after.end))
            after = line(hi, edge, middle(n, f));
        }
      }
      for (const candidate of [before, after])
        if (candidate && candidate.end - candidate.start >= 0.5)
          gaps.push(candidate);
    }
  }
  return gaps.length ? gaps : insets;
}
