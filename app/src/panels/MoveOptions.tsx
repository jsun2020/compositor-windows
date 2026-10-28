import { useEditor } from "../state/store";

const ALIGNMENTS = [
  { id: "Left Edges" as const, label: "Align left edges", glyph: "L" },
  { id: "Horizontal Centers" as const, label: "Align horizontal centers", glyph: "HC" },
  { id: "Right Edges" as const, label: "Align right edges", glyph: "R" },
  { id: "Top Edges" as const, label: "Align top edges", glyph: "T" },
  { id: "Vertical Centers" as const, label: "Align vertical centers", glyph: "VC" },
  { id: "Bottom Edges" as const, label: "Align bottom edges", glyph: "B" },
];

const DISTRIBUTIONS = [
  { id: "Horizontal Centers" as const, label: "Distribute horizontal centers", glyph: "H" },
  { id: "Vertical Centers" as const, label: "Distribute vertical centers", glyph: "V" },
  { id: "Horizontal Spacing" as const, label: "Distribute horizontal spacing", glyph: "HS" },
  { id: "Vertical Spacing" as const, label: "Distribute vertical spacing", glyph: "VS" },
];

/** Whether Align is enabled for the current selection (no perspective corners, editable). */
export function canAlignSelection(opts: { working: boolean; panelOwns: boolean; distorting: boolean; ids: string[] }): boolean {
  return !opts.working && !opts.panelOwns && !opts.distorting && opts.ids.length > 0;
}

export function canDistributeSelection(opts: { working: boolean; panelOwns: boolean; distorting: boolean; ids: string[] }): boolean {
  return canAlignSelection(opts) && opts.ids.length > 2;
}

/** Move tool Align / Distribute controls. */
export function MoveOptions() {
  const s = useEditor();
  const doc = s.activeId ? s.documents[s.activeId] : null;
  if (!doc || s.tool !== "move" || s.sheet !== null) return null;
  const ids = s.selectedLayerIds;
  const distorting = !!s.transformEdit?.corners;
  const flags = { working: s.working, panelOwns: s.panelOwnsDocument(), distorting, ids };
  const canAlign = canAlignSelection(flags);
  const canDistribute = canDistributeSelection(flags);
  return (
    <div className="tool-options" data-testid="move-options">
      <span className="muted">Align</span>
      <span className="segmented" role="group" aria-label="Align layers">
        {ALIGNMENTS.map((a) => (
          <button key={a.id} type="button" data-testid={`align-${a.id}`} aria-label={a.label} title={a.label}
            disabled={!canAlign}
            onClick={() => s.run({ type: "AlignLayers", ids, alignment: a.id })}>{a.glyph}</button>
        ))}
      </span>
      <span className="muted">Distribute</span>
      <span className="segmented" role="group" aria-label="Distribute layers">
        {DISTRIBUTIONS.map((d) => (
          <button key={d.id} type="button" data-testid={`distribute-${d.id}`} aria-label={d.label} title={d.label}
            disabled={!canDistribute}
            onClick={() => s.run({ type: "DistributeLayers", ids, distribution: d.id })}>{d.glyph}</button>
        ))}
      </span>
    </div>
  );
}
