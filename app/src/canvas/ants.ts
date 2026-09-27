import type { PointTuple } from "../engine/types";

/** How often the ants march, in ms (`updateAntsTimer`, EditorCanvas.swift:2004-2020). */
export const ANTS_INTERVAL_MS = 120;
/** One march step; the phase wraps at the dash period (4 on, 4 off). */
export const nextPhase = (phase: number): number => (phase + 1) % 8;

/** The outline's detail for a zoom (device px per document px): 1 at or above 1:1, else the power of
 * two at or below the zoom, so the engine traces a very detailed outline no finer than the screen
 * shows it (`selection_lod`; TransformOverlay.swift:92-177, :284-298). */
export function outlineStep(zoom: number): number {
  if (!(zoom > 0) || zoom >= 1) return 1;
  return Math.pow(2, Math.floor(Math.log2(zoom)));
}

/** The last outline fetched, kept until the document, the selection's revision or the step
 * changes: the ants redraw every tick without asking the engine again. */
export class OutlineCache {
  private key: string | null = null;
  private contours: PointTuple[][] = [];
  get(doc: string, revision: number, step: number, fetch: () => PointTuple[][]): PointTuple[][] {
    const key = `${doc}:${revision}:${step}`;
    if (key !== this.key) { this.contours = fetch(); this.key = key; }
    return this.contours;
  }
}
