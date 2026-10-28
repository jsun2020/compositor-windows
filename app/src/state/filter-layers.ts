import type { FilterLayerSettings, FilterParams, LayerAdjustment, LensEffectSettings } from "../engine/types";
import { DEFAULT_SHARPEN, DEFAULT_TONAL, DEFAULT_VIGNETTE } from "../engine/types";

export function defaultFilterLayerSettings(): FilterLayerSettings {
  return {
    progressiveFrom: 0, progressiveTo: 20, progressiveRadial: false, progressiveInverted: false,
    bloomAmount: 40, bloomRadius: 24,
    rgbSplitDistance: 10, rgbSplitBlur: 0, rgbSplitAngle: 0, rgbSplitRadial: false, rgbSplitReach: 1000,
  };
}

export function defaultLensSettings(): LensEffectSettings {
  return {
    effect: "Anamorphic Flare", threshold: 90,
    flareAmount: 50, flareLength: 60, flareBlue: 80,
    starAmount: 50, starLength: 12, starPoints: 6, starAngle: 15,
    mistStrength: 50, mistSize: 30,
    prismAmount: 80, prismCopies: 5, prismOffset: 6, prismAngle: 180,
    prismBothWays: false, prismBlur: 0, prismFringe: 30,
    bokehRadius: 2, bokehBrightness: 50, bokehRing: 100, bokehShape: "Round",
  };
}

/** Effective prismFringe: missing in old files means 30 (project-format v20). */
export function effectivePrismFringe(s: LensEffectSettings): number {
  return s.prismFringe ?? 30;
}

export function filterLayerParams(a: LayerAdjustment | null | undefined): FilterParams | null {
  if (!a) return null;
  const s = { ...defaultFilterLayerSettings(), ...a.filterLayerSettings };
  if (a.kind === "Progressive Blur") return { filter: "ProgressiveBlur", settings: { from: s.progressiveFrom, to: s.progressiveTo, start: s.progressiveStart ?? null, end: s.progressiveEnd ?? null, radial: s.progressiveRadial, inverted: s.progressiveInverted } };
  if (a.kind === "Bloom / Glow") return { filter: "BloomGlow", amount: s.bloomAmount, radius: s.bloomRadius };
  if (a.kind === "RGB Split") return { filter: "RgbSplit", settings: { distance: s.rgbSplitDistance, blur: s.rgbSplitBlur, angle: s.rgbSplitAngle, radial: s.rgbSplitRadial, center: s.rgbSplitCenter ?? null, reach: s.rgbSplitReach } };
  if (a.kind === "Vignette") return { filter: "Vignette", settings: { ...DEFAULT_VIGNETTE, ...s.vignette } };
  if (a.kind === "Tonal Contrast") return { filter: "TonalContrast", settings: { ...DEFAULT_TONAL, ...s.tonal } };
  if (a.kind === "Lens Effects") return { filter: "LensEffects", settings: { ...defaultLensSettings(), ...s.lens } };
  if (a.kind === "Sharpen") { const sh = { ...DEFAULT_SHARPEN, ...s.sharpen }; return { filter: "Sharpen", amount: sh.amount, radius: sh.radius }; }
  return null;
}

export function withFilterLayerParams(a: LayerAdjustment, p: FilterParams): LayerAdjustment {
  const s = { ...defaultFilterLayerSettings(), ...a.filterLayerSettings };
  if (p.filter === "ProgressiveBlur") {
    s.progressiveFrom = p.settings.from; s.progressiveTo = p.settings.to; s.progressiveRadial = p.settings.radial; s.progressiveInverted = p.settings.inverted;
    s.progressiveStart = p.settings.start ?? undefined; s.progressiveEnd = p.settings.end ?? undefined;
  } else if (p.filter === "BloomGlow") {
    s.bloomAmount = p.amount; s.bloomRadius = p.radius;
  } else if (p.filter === "RgbSplit") {
    s.rgbSplitDistance = p.settings.distance; s.rgbSplitBlur = p.settings.blur; s.rgbSplitAngle = p.settings.angle; s.rgbSplitRadial = p.settings.radial;
    s.rgbSplitCenter = p.settings.center ?? undefined; s.rgbSplitReach = p.settings.reach;
  } else if (p.filter === "Vignette") {
    s.vignette = { ...p.settings };
  } else if (p.filter === "TonalContrast") {
    s.tonal = { ...p.settings };
  } else if (p.filter === "LensEffects") {
    s.lens = { ...p.settings };
  } else if (p.filter === "Sharpen") {
    s.sharpen = { amount: p.amount, radius: p.radius };
  } else return a;
  return { ...a, filterLayerSettings: s };
}
