import { useEffect, useRef, useState } from "react";
import { useEditor } from "../state/store";
import { LEVELS_CHANNELS } from "../tools/levels-tools";
import { curveSamples, insertPoint, movePoint, nearestPoint, removePoint } from "../tools/curves-editor";
import type { CurvePoint, LevelsChannel } from "../engine/types";

const SIZE = 256;

export function CurvesPanel() {
  const s = useEditor();
  const edit = s.adjustEdit!;
  const settings = edit.adjustment!.curves;
  const index = LEVELS_CHANNELS.indexOf(settings.channel);
  const points = settings.channels[index];
  const canvas = useRef<HTMLCanvasElement>(null);
  const [selected, setSelected] = useState(0);
  const [dragging, setDragging] = useState(false);
  // `selected` goes stale in three ways: switching to a channel with fewer points, a right-click
  // removal, and an insert the editor refused (the 32-point cap, or a click that landed exactly
  // on an existing point's input) -- none of those reset it. Every read below goes through this
  // clamp instead, so no crash depends on remembering to reset `selected` at each of those sites;
  // `points.length` is always at least 2 (the two endpoints), so this is always a valid index.
  const selectedIndex = Math.min(Math.max(selected, 0), points.length - 1);
  useEffect(() => {
    const element = canvas.current; if (!element) return;
    const ctx = element.getContext("2d")!;
    ctx.clearRect(0, 0, SIZE, SIZE);
    ctx.strokeStyle = "#3a3a3a"; ctx.beginPath(); ctx.moveTo(0, SIZE); ctx.lineTo(SIZE, 0); ctx.stroke();
    ctx.strokeStyle = "#e0e0e0"; ctx.beginPath();
    curveSamples(points, SIZE).forEach((y, x) => (x === 0 ? ctx.moveTo(x, SIZE - y) : ctx.lineTo(x, SIZE - y)));
    ctx.stroke();
    points.forEach((p, i) => { ctx.fillStyle = i === selectedIndex ? "#4da3ff" : "#e0e0e0"; ctx.fillRect(p.x - 3, SIZE - p.y - 3, 6, 6); });
  }, [points, selectedIndex]);
  const pointAt = (e: React.PointerEvent | React.MouseEvent): CurvePoint => {
    const r = canvas.current!.getBoundingClientRect();
    return { x: ((e.clientX - r.left) / r.width) * 255, y: 255 - ((e.clientY - r.top) / r.height) * 255 };
  };
  const setPoints = (next: CurvePoint[]) => {
    const channels = settings.channels.map((c, i) => (i === index ? next : c));
    s.updateAdjust({ adjustment: { ...edit.adjustment!, curves: { ...settings, channels } } });
  };
  return (
    <>
      <label>Channel <select data-testid="curves-channel" value={settings.channel} onChange={(e) => s.updateAdjust({ adjustment: { ...edit.adjustment!, curves: { ...settings, channel: e.target.value as LevelsChannel } } })}>
        {LEVELS_CHANNELS.map((c) => <option key={c} value={c}>{c}</option>)}
      </select></label>
      <canvas data-testid="curves-editor" ref={canvas} width={SIZE} height={SIZE} className="curves"
        onPointerDown={(e) => {
          const at = pointAt(e);
          const hit = nearestPoint(points, at, 8);
          if (hit !== null) setSelected(hit);
          else {
            const next = insertPoint(points, at);
            // A no-op `insertPoint` (the 32-point cap, or a point already at that input) returns
            // the same-length array back; searching it for the point that was never added would
            // find none and select nothing (`selected` = -1), so leave the current selection
            // alone instead.
            if (next.length > points.length) { setPoints(next); setSelected(next.findIndex((p) => p.x === Math.round(at.x))); }
          }
          setDragging(true); (e.target as HTMLElement).setPointerCapture(e.pointerId);
        }}
        onPointerMove={(e) => { if (dragging) setPoints(movePoint(points, selectedIndex, pointAt(e))); }}
        onPointerUp={() => setDragging(false)}
        onContextMenu={(e) => { e.preventDefault(); const hit = nearestPoint(points, pointAt(e), 8); if (hit !== null) { setPoints(removePoint(points, hit)); setSelected(0); } }} />
      <label>Input <input aria-label="Input" data-testid="curves-point-x" type="number" value={Math.round(points[selectedIndex].x)}
        onChange={(e) => setPoints(movePoint(points, selectedIndex, { x: Number(e.target.value), y: points[selectedIndex].y }))} /></label>
      <label>Output <input aria-label="Output" data-testid="curves-point-y" type="number" value={Math.round(points[selectedIndex].y)}
        onChange={(e) => setPoints(movePoint(points, selectedIndex, { x: points[selectedIndex].x, y: Number(e.target.value) }))} /></label>
    </>
  );
}
