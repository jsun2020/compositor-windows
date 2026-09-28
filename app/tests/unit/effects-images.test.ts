import { afterEach, describe, expect, it } from "vitest";
import { EFFECTS_LIMITS, EffectsImages, placedLike, reducedLevel } from "../../src/canvas/effects-images";
import type { Corners, LayerDraw, LayerTransform } from "../../src/engine/types";
import type { EngineClient } from "../../src/engine/client";
import type { JobClient, JobRequest, JobResult } from "../../src/engine/jobs";
import { homographyUnitTo, mat3Apply } from "../../src/tools/transform-geometry";

const defaults = { ...EFFECTS_LIMITS };
afterEach(() => Object.assign(EFFECTS_LIMITS, defaults));

/** The plan's draw of a styled 40 x 24 layer at (28, 24) with an inset of 10: the engine's grown
 * transform (effects/mod.rs `grown_transform`): 60 x 44 about the same centre. */
function drawOf(extra: Partial<LayerDraw> = {}): LayerDraw {
  const transform: LayerTransform = { origin: [18, 14], size: [60, 44], rotation: 0, flipX: false, flipY: false, sampling: "High quality" };
  return { id: "A", transform, corners: null, pixelsWidth: 60, pixelsHeight: 44, pixelsRevision: 3, opacity: 1, blend: "Normal", coverages: [], clip: null,
    keepsAlpha: false, effects: { inset: 10, key: "k" }, ...extra };
}

describe("reducedLevel", () => {
  it("halves until the longer side is at most 1536", () => {
    expect(reducedLevel(1536, 900)).toBe(0);
    expect(reducedLevel(1537, 10)).toBe(1);
    expect(reducedLevel(6000, 4000)).toBe(2); // 6000 -> 3000 -> 1500
    expect(reducedLevel(10000, 10000)).toBe(3); // -> 1250
  });
});

describe("placedLike", () => {
  it("puts the layer's own pixels at its own box, and an image with another inset grown in proportion", () => {
    const d = drawOf();
    expect(placedLike(d, 40, 24, 0).transform).toMatchObject({ origin: [28, 24], size: [40, 24] });
    expect(placedLike(d, 40, 24, 10).transform).toMatchObject({ origin: [18, 14], size: [60, 44] });
    // Halved pixels with an inset of 6 (rounded up from 5): 40 * (20 + 12) / 20 = 64 wide about x 48.
    expect(placedLike(d, 20, 12, 6).transform).toMatchObject({ origin: [16, 12], size: [64, 48] });
  });

  it("undoes and redoes the growth of a distorted layer's corners", () => {
    // A parallelogram for the layer's own corners, and the plan's corners for inset 10 around 40 x 24.
    const own = [{ x: 30, y: 20 }, { x: 70, y: 26 }, { x: 66, y: 50 }, { x: 26, y: 44 }];
    const h = homographyUnitTo(own);
    const [a, b] = [10 / 40, 10 / 24];
    const grown = [mat3Apply(h, { x: -a, y: -b }), mat3Apply(h, { x: 1 + a, y: -b }), mat3Apply(h, { x: 1 + a, y: 1 + b }), mat3Apply(h, { x: -a, y: 1 + b })];
    const d = drawOf({ corners: grown.map((p) => [p.x, p.y]) as Corners });
    const plain = placedLike(d, 40, 24, 0).corners!;
    plain.forEach(([x, y], i) => { expect(x).toBeCloseTo(own[i].x, 9); expect(y).toBeCloseTo(own[i].y, 9); });
  });
});

describe("EffectsImages.choose", () => {
  /** A stub engine that has the full image once `full` is set, and a stub worker whose jobs finish
   * when `finish` is called. */
  function setup() {
    const asked: JobRequest[] = [];
    const state = { full: false, kept: 0, closed: false };
    let finish: (r: JobResult | null) => void = () => {};
    const engine = {
      hasEffectsImage: () => state.full,
      displayJobInput: (_d: string, _l: string, level: number) => {
        if (state.closed) throw new Error("no such document"); // the engine's NoDocument
        return { input: JSON.stringify({ pixels: [40 >> level, 24 >> level], stamp: {} }), pixels: new ArrayBuffer(4), mask: null };
      },
      keepEffectsImage: () => { state.kept++; return true; },
    } as unknown as EngineClient;
    const jobs = { run: (_c: string, r: JobRequest) => { asked.push(r); return new Promise<JobResult | null>((resolve) => { finish = resolve; }); } } as unknown as JobClient;
    let landed = 0;
    const images = new EffectsImages(() => jobs, () => { landed++; });
    return { images, engine, asked, state, finish: (r: JobResult | null) => finish(r), landed: () => landed };
  }
  const flush = () => new Promise((r) => setTimeout(r, 0));

  it("asks for a reduced image, then the full one, once each, and draws what has landed", async () => {
    EFFECTS_LIMITS.reduced = 32;
    const { images, engine, asked, state, finish, landed } = setup();
    expect(images.choose(engine, "D", "A", "fx:1", drawOf(), 40, 24, null)).toBeNull();
    expect(images.choose(engine, "D", "A", "fx:1", drawOf(), 40, 24, null)).toBeNull();
    expect(asked.map((r) => (r as { factor: number }).factor), "one reduced job, halved once").toEqual([0.5]);
    finish({ header: JSON.stringify({ width: 32, height: 24, inset: 6 }), pixels: new ArrayBuffer(32 * 24 * 4), mask: null });
    await flush();
    expect(landed()).toBe(1);
    const reduced = images.choose(engine, "D", "A", "fx:1", drawOf(), 40, 24, null);
    expect(reduced).toMatchObject({ key: "fx:1", width: 32, inset: 6 });
    // The full ask is deferred a task (fix round 3), so it lands after the reduced image's own frame,
    // not inside the same synchronous `choose()` call that just returned it.
    await flush();
    expect(asked.map((r) => (r as { factor: number }).factor), "then the full one").toEqual([0.5, 1]);
    finish({ header: JSON.stringify({ width: 60, height: 44, inset: 10 }), pixels: new ArrayBuffer(60 * 44 * 4), mask: null });
    await flush();
    expect(state.kept, "the full image goes to the engine").toBe(1);
    state.full = true;
    expect(images.choose(engine, "D", "A", "fx:1", drawOf(), 40, 24, null)).toBe("full");
    // New pixels: the last reduced image is drawn meanwhile, and a reduced one for them is asked for.
    state.full = false;
    expect(images.choose(engine, "D", "A", "fx:2", drawOf(), 40, 24, null)).toMatchObject({ key: "fx:1" });
    expect(asked.length).toBe(3);
  });

  it("asks for nothing at full size past the full-size limit, and for the full image at once when no reduction is needed", async () => {
    EFFECTS_LIMITS.reduced = 32; EFFECTS_LIMITS.full = 40 * 24 - 1;
    const big = setup();
    big.images.choose(big.engine, "D", "A", "fx:1", drawOf(), 40, 24, null);
    big.finish({ header: JSON.stringify({ width: 32, height: 24, inset: 6 }), pixels: new ArrayBuffer(4), mask: null });
    await flush();
    big.images.choose(big.engine, "D", "A", "fx:1", drawOf(), 40, 24, null);
    expect(big.asked.length, "reduced only").toBe(1);
    EFFECTS_LIMITS.reduced = 1536; EFFECTS_LIMITS.full = defaults.full;
    const small = setup();
    small.images.choose(small.engine, "D", "A", "fx:1", drawOf(), 40, 24, null);
    // Deferred a task too (fix round 3): no reduction stage at all, but still a full-size ask.
    await flush();
    expect(small.asked.map((r) => (r as { factor: number }).factor)).toEqual([1]);
  });

  it("drops a deferred full-size ask superseded or forgotten before it runs, and one whose document closed is asked again later", async () => {
    // 40 x 24 needs no reduction: every choose() below defers a full-size ask (fix round 3).
    const superseded = setup();
    superseded.images.choose(superseded.engine, "D", "A", "fx:1", drawOf(), 40, 24, null);
    superseded.images.choose(superseded.engine, "D", "A", "fx:2", drawOf(), 40, 24, null);
    await flush();
    expect(superseded.asked.length, "only the newer pixels' image is asked for").toBe(1);
    const forgotten = setup();
    forgotten.images.choose(forgotten.engine, "D", "A", "fx:1", drawOf(), 40, 24, null);
    forgotten.images.retainOnly("D", new Set());
    await flush();
    expect(forgotten.asked.length, "a layer let go asks for nothing").toBe(0);
    // The copy throws (the document closed meanwhile): nothing escapes the task, and the ask is not
    // left pending, so the same image is asked for again once it can be.
    const closed = setup();
    closed.state.closed = true;
    closed.images.choose(closed.engine, "D", "A", "fx:1", drawOf(), 40, 24, null);
    await flush();
    expect(closed.asked.length).toBe(0);
    closed.state.closed = false;
    closed.images.choose(closed.engine, "D", "A", "fx:1", drawOf(), 40, 24, null);
    await flush();
    expect(closed.asked.length, "asked again").toBe(1);
  });
});
