import type { LevelRange, LevelsChannel } from "../engine/types";

export const LEVELS_CHANNELS: LevelsChannel[] = ["RGB", "Red", "Green", "Blue"];

/** Display-only vertical scaling, ported from LevelsHistogramDisplay on macOS: keep linear bin
 * ratios, but cap isolated spikes so a large solid background cannot flatten the graph. */
export function histogramScale(bins: number[]): number {
  const positive = bins.filter((v) => Number.isFinite(v) && v > 0);
  const peak = positive.length ? Math.max(...positive) : 0;
  if (peak <= 0) return 0;
  const interior = bins.slice(1, -1).filter((v) => Number.isFinite(v) && v > 0).sort((a, b) => a - b);
  if (interior.length === 0) return peak;
  const typical = interior[Math.floor((interior.length - 1) * 0.95)];
  return Math.min(peak, typical * 4);
}

const clamp = (v: number, lo: number, hi: number, fallback: number) => (Number.isFinite(v) ? Math.min(hi, Math.max(lo, v)) : fallback);

/** The same normalisation `LevelRange::normalized` applies in the engine, so the fields show
 * what will actually be used. */
export function clampRange(range: LevelRange): LevelRange {
  const black = clamp(range.black, 0, 254, 0);
  return {
    black,
    white: clamp(range.white, black + 1, 255, 255),
    gamma: clamp(range.gamma, 0.1, 9.99, 1),
    outputBlack: clamp(range.outputBlack, 0, 255, 0),
    outputWhite: clamp(range.outputWhite, 0, 255, 255),
  };
}
