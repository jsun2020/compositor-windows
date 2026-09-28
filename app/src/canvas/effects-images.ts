// The canvas's effects images for large styled layers (Phase 4b-1; spec section 3, the user's choice
// of 2026-09-28): a reduced image first, at most 1536 px as the Mac's canvas preview is
// (EffectsPreviewCache.swift:51, :114-133), then the full-size one while the padded image is at most
// about 24 MP; above that only the reduced one. Both are made by the job worker; until one lands the
// last image made for the layer stays on screen, and with none the layer is drawn plainly, as the Mac
// draws it until its first preview is ready. Small images are made at once on the UI thread, as before.
import type { Corners, LayerDraw, LayerTransform, PreviewEdit } from "../engine/types";
import type { EngineClient } from "../engine/client";
import { EFFECTS_JOB_DISPLACED, type JobClient } from "../engine/jobs";
import { sizeAtLevel } from "./layer-textures";
import { fromTuple, homographyUnitTo, mat3Apply, toTuple } from "../tools/transform-geometry";

/** The sizes that choose the path (tests lower them): a padded image of at most `sync` pixels is made
 * on the UI thread; the reduced image's longer side is at most `reduced`; the full-size image is made
 * for a layer of at most `full` pixels of its own (a 6000 x 4000 photo qualifies, effects and all). */
export const EFFECTS_LIMITS = { sync: 65_536, reduced: 1536, full: 24_000_000 };

/** An effects image the worker made at a reduction (the full-size ones go into the engine's cache). */
export interface ReducedImage { key: string; width: number; height: number; inset: number; bytes: Uint8Array; }

/** The fewest halvings that bring the layer's longer side to at most `EFFECTS_LIMITS.reduced`. */
export function reducedLevel(width: number, height: number): number {
  let level = 0;
  for (let s = sizeAtLevel(width, height, 0); Math.max(s.width, s.height) > EFFECTS_LIMITS.reduced && s.width > 1 && s.height > 1; s = sizeAtLevel(width, height, ++level)) { /* halve */ }
  return level;
}

/** Where an effects image with its own `inset` around `width` x `height` layer pixels is drawn, given
 * the plan's draw of the full padded image (`draw.effects.inset` around `draw.pixelsWidth` padded):
 * the layer's own box, grown in proportion to this image (engine `grown_transform`, `grown_corners`).
 * An inset of 0 is the layer drawn plainly. */
export function placedLike(draw: LayerDraw, width: number, height: number, inset: number): { transform: LayerTransform; corners: Corners | null } {
  const i0 = draw.effects!.inset;
  const pw = draw.pixelsWidth - 2 * i0, ph = draw.pixelsHeight - 2 * i0;
  if (draw.corners) {
    // The plan's corners map the unit square grown by (i0 / pw, i0 / ph); reach this image's.
    const grown = homographyUnitTo(draw.corners.map(fromTuple));
    const [a, b, a1, b1] = [i0 / pw, i0 / ph, inset / width, inset / height];
    const at = (u: number, v: number) => mat3Apply(grown, { x: (u + a) / (1 + 2 * a), y: (v + b) / (1 + 2 * b) });
    const corners = [at(-a1, -b1), at(1 + a1, -b1), at(1 + a1, 1 + b1), at(-a1, 1 + b1)].map(toTuple) as Corners;
    return { transform: draw.transform, corners };
  }
  const t = draw.transform;
  const own = { width: t.size[0] * pw / (pw + 2 * i0), height: t.size[1] * ph / (ph + 2 * i0) };
  const size: [number, number] = [own.width * (width + 2 * inset) / width, own.height * (height + 2 * inset) / height];
  const cx = t.origin[0] + t.size[0] / 2, cy = t.origin[1] + t.size[1] / 2;
  return { transform: { ...t, origin: [cx - size[0] / 2, cy - size[1] / 2], size }, corners: null };
}

/** The effects images of one renderer's large styled layers, and the jobs that make them. */
export class EffectsImages {
  /** The newest reduced image landed for each `doc:layer`, whatever pixels it was made from. */
  private reduced = new Map<string, ReducedImage>();
  /** What was last asked for each `doc:layer`, so a frame asks once. */
  private asked = new Map<string, string>();

  constructor(private readonly jobs: () => JobClient | null, private readonly landed: () => void) {}

  /** For a large styled layer this frame: "full" when the engine has the full-size image (draw it the
   * usual way), else the reduced image to draw (null: none yet, draw the layer plainly). Asks the
   * worker for what the image named `key` still lacks. */
  choose(engine: EngineClient, doc: string, layer: string, key: string, draw: LayerDraw, pixelsWidth: number, pixelsHeight: number, edit: PreviewEdit | null): "full" | ReducedImage | null {
    const k = `${doc}:${layer}`;
    const wantFull = pixelsWidth * pixelsHeight <= EFFECTS_LIMITS.full;
    if (wantFull && engine.hasEffectsImage(doc, layer, edit)) return "full";
    const have = this.reduced.get(k) ?? null;
    const level = reducedLevel(pixelsWidth, pixelsHeight);
    // A layer small enough needs no reduced image: its full-size one is asked for straight away.
    const next = have?.key === key || level === 0 ? (wantFull ? "full" : null) : "reduced";
    if (next && this.asked.get(k) !== `${key}:${next}`) {
      this.asked.set(k, `${key}:${next}`);
      // `key` is this class's own compound identity for the image (`doc:layer`'s caller-supplied
      // `bytesKey`, e.g. gl-renderer.ts's "fx:pixelsRev:maskRev:inset:engineKey"); `draw.effects.key`
      // is the engine's own `EffectsDraw.key` alone, in the engine's own format -- `keep_effects_image`
      // (fix round 1, issue 2) compares against exactly that, freshly recomputed, so it must be told
      // that one, not this class's compound string, which the engine has never heard of.
      this.ask(engine, doc, layer, key, draw.effects!.key, next === "full" ? 0 : level, pixelsWidth, edit);
    }
    return have;
  }

  private ask(engine: EngineClient, doc: string, layer: string, key: string, engineKey: string, level: number, pixelsWidth: number, edit: PreviewEdit | null): void {
    const jobs = this.jobs(); if (!jobs) return;
    const copy = engine.displayJobInput(doc, layer, level);
    const width = (JSON.parse(copy.input) as { pixels: [number, number] }).pixels[0];
    const request = { kind: "effects" as const, input: copy.input, pixels: copy.pixels!, mask: copy.mask, factor: width / pixelsWidth, edit: edit ? JSON.stringify(edit) : null };
    const k = `${doc}:${layer}`;
    const asked = `${key}:${level === 0 ? "full" : "reduced"}`;
    // However the job ends, the next frame asks for what is still missing (again, if the engine let
    // the image go or the job came to nothing), unless a newer ask has taken its place.
    const settled = () => { if (this.asked.get(k) === asked) this.asked.delete(k); };
    jobs.run(`fx:${k}`, request).then((result) => {
      settled();
      // Superseded by a newer image's job, or nothing to draw.
      if (!result?.header || !result.pixels) return;
      const image = JSON.parse(result.header) as { width: number; height: number; inset: number };
      if (level === 0) engine.keepEffectsImage(doc, layer, copy.input, engineKey, edit, image.width, image.height, result.pixels);
      else this.reduced.set(k, { key, ...image, bytes: new Uint8Array(result.pixels) });
      this.landed();
    }).catch((e) => {
      settled();
      // An edit or histogram job displaced this one (fix round 1, issue 3): it still needs making,
      // so the next frame must notice and ask again -- unlike an ordinary refusal or a dead worker,
      // which leave the picture exactly as it was, nothing new to redraw for.
      if (e instanceof Error && e.message === EFFECTS_JOB_DISPLACED) this.landed();
    });
  }

  /** Forgets the layers of `doc` not in `ids`, and every other document's. */
  retainOnly(doc: string, ids: Set<string>): void {
    for (const map of [this.reduced, this.asked] as Map<string, unknown>[]) {
      for (const k of [...map.keys()]) { const [d, l] = k.split(":"); if (d !== doc || !ids.has(l)) map.delete(k); }
    }
  }
}
