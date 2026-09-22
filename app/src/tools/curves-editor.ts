import type { CurvePoint } from "../engine/types";

export const MAX_POINTS = 32;

/** The engine's `CurvesSettings::value` (engine/src/adjust/settings.rs), for drawing the curve
 * and its handles. The applied result always comes from the engine; this only draws. */
export function curveValue(points: CurvePoint[], x: number): number {
  let i = 0;
  for (let j = 0; j < points.length; j++) if (points[j].x <= x) i = j;
  i = Math.min(points.length - 2, Math.max(0, i));
  const d = points.slice(0, -1).map((p, j) => (points[j + 1].y - p.y) / (points[j + 1].x - p.x));
  const slope = (j: number): number => {
    if (j === 0) return d[0];
    if (j === points.length - 1) return d[d.length - 1];
    if (d[j - 1] * d[j] <= 0) return 0;
    return 2 / (1 / d[j - 1] + 1 / d[j]);
  };
  const h = points[i + 1].x - points[i].x;
  const t = Math.min(1, Math.max(0, (x - points[i].x) / h));
  const y = (2 * t ** 3 - 3 * t ** 2 + 1) * points[i].y + (t ** 3 - 2 * t ** 2 + t) * h * slope(i)
    + (-2 * t ** 3 + 3 * t ** 2) * points[i + 1].y + (t ** 3 - t ** 2) * h * slope(i + 1);
  return Math.min(255, Math.max(0, y));
}
export function curveSamples(points: CurvePoint[], count = 256): number[] {
  return Array.from({ length: count }, (_, i) => curveValue(points, (i * 255) / (count - 1)));
}
/** The index of a handle within `tolerance` of a point in curve space, or null. */
export function nearestPoint(points: CurvePoint[], at: CurvePoint, tolerance: number): number | null {
  let best: number | null = null; let bestDistance = tolerance;
  points.forEach((p, i) => { const distance = Math.hypot(p.x - at.x, p.y - at.y); if (distance <= bestDistance) { best = i; bestDistance = distance; } });
  return best;
}
export function insertPoint(points: CurvePoint[], at: CurvePoint): CurvePoint[] {
  const x = Math.round(Math.min(255, Math.max(0, at.x)));
  const y = Math.min(255, Math.max(0, at.y));
  if (points.length >= MAX_POINTS || points.some((p) => p.x === x)) return points;
  return [...points, { x, y }].sort((a, b) => a.x - b.x);
}
/** Moves one handle, keeping the inputs strictly increasing; the two endpoints keep their input. */
export function movePoint(points: CurvePoint[], index: number, to: CurvePoint): CurvePoint[] {
  const next = points.map((p) => ({ ...p }));
  const y = Math.min(255, Math.max(0, to.y));
  if (index === 0 || index === points.length - 1) { next[index].y = y; return next; }
  const low = points[index - 1].x + 1;
  const high = points[index + 1].x - 1;
  next[index] = { x: Math.round(Math.min(high, Math.max(low, to.x))), y };
  return next;
}
export function removePoint(points: CurvePoint[], index: number): CurvePoint[] {
  if (index <= 0 || index >= points.length - 1 || points.length <= 2) return points;
  return points.filter((_, i) => i !== index);
}
