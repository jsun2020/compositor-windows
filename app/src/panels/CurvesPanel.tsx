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
  useEffect(() => {
    const element = canvas.current; if (!element) return;
    const ctx = element.getContext("2d")!;
    ctx.clearRect(0, 0, SIZE, SIZE);
    ctx.strokeStyle = "#3a3a3a"; ctx.beginPath(); ctx.moveTo(0, SIZE); ctx.lineTo(SIZE, 0); ctx.stroke();
    ctx.strokeStyle = "#e0e0e0"; ctx.beginPath();
    curveSamples(points, SIZE).forEach((y, x) => (x === 0 ? ctx.moveTo(x, SIZE - y) : ctx.lineTo(x, SIZE - y)));
    ctx.stroke();
    points.forEach((p, i) => { ctx.fillStyle = i === selected ? "#4da3ff" : "#e0e0e0"; ctx.fillRect(p.x - 3, SIZE - p.y - 3, 6, 6); });
  }, [points, selected]);
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
          if (hit === null) { const next = insertPoint(points, at); setPoints(next); setSelected(next.findIndex((p) => p.x === Math.round(at.x))); }
          else setSelected(hit);
          setDragging(true); (e.target as HTMLElement).setPointerCapture(e.pointerId);
        }}
        onPointerMove={(e) => { if (dragging) setPoints(movePoint(points, selected, pointAt(e))); }}
        onPointerUp={() => setDragging(false)}
        onContextMenu={(e) => { e.preventDefault(); const hit = nearestPoint(points, pointAt(e), 8); if (hit !== null) { setPoints(removePoint(points, hit)); setSelected(0); } }} />
      <label>Input <input aria-label="Input" data-testid="curves-point-x" type="number" value={Math.round(points[selected]?.x ?? 0)}
        onChange={(e) => setPoints(movePoint(points, selected, { x: Number(e.target.value), y: points[selected].y }))} /></label>
      <label>Output <input aria-label="Output" data-testid="curves-point-y" type="number" value={Math.round(points[selected]?.y ?? 0)}
        onChange={(e) => setPoints(movePoint(points, selected, { x: points[selected].x, y: Number(e.target.value) }))} /></label>
    </>
  );
}
