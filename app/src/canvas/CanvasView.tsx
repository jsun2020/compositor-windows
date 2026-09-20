import { useEffect, useRef } from "react";
import { useEditor } from "../state/store";
import { createRenderer, type Renderer } from "./renderer";
import { drawOverlay } from "./overlay";
import { installTestApi } from "../test-api";

export const HIT_HANDLE_PX = 6;

export function CanvasView() {
  const glRef = useRef<HTMLCanvasElement>(null);
  const overlayRef = useRef<HTMLCanvasElement>(null);
  const rendererRef = useRef<Renderer | null>(null);
  const checkerboardRef = useRef(true);
  const guidesRef = useRef<{ xs: number[]; ys: number[] }>({ xs: [], ys: [] });
  const engine = useEditor((s) => s.engine);
  const activeId = useEditor((s) => s.activeId);
  const state = useEditor((s) => (s.activeId ? s.documents[s.activeId] : null));
  const viewport = useEditor((s) => (s.activeId ? s.viewports[s.activeId] : null));
  const cropRect = useEditor((s) => s.cropRect);
  const tool = useEditor((s) => s.tool);
  const renderTick = useEditor((s) => s.renderTick);

  // Renderer lifetime follows the canvas element.
  useEffect(() => {
    if (!engine || !glRef.current) return;
    const renderer = createRenderer(glRef.current, engine);
    rendererRef.current = renderer;
    useEditor.getState().setRendererKind(renderer.kind);
    installTestApi({
      renderer,
      setCheckerboard: (on: boolean) => { checkerboardRef.current = on; useEditor.getState().invalidate(); },
      setZoom: async (z: number) => { const s = useEditor.getState(); const vp = s.viewports[s.activeId!]; const d = s.documents[s.activeId!];
        vp.setZoom(z * (window.devicePixelRatio || 1), vp.center, { width: d.width, height: d.height }); s.invalidate();
        await new Promise((r) => requestAnimationFrame(r)); },
      readDocumentPixels: () => {
        const s = useEditor.getState(); const vp = s.viewports[s.activeId!]; const d = s.documents[s.activeId!];
        const dpr = window.devicePixelRatio || 1; const all = renderer.readPixels();
        const rect = vp.documentRect({ width: d.width, height: d.height });
        const W = glRef.current!.width;
        // Round each edge independently and derive width/height from the difference, matching
        // the renderer's scissor rounding exactly so this extracts the same device pixels the
        // GL renderer actually painted the document into (see GlRenderer.render).
        const x0 = Math.round(rect.x * dpr), x1 = Math.round((rect.x + rect.width) * dpr);
        const y0 = Math.round(rect.y * dpr), y1 = Math.round((rect.y + rect.height) * dpr);
        const w = x1 - x0, h = y1 - y0;
        const out = new Uint8Array(w * h * 4);
        for (let y = 0; y < h; y++) out.set(all.subarray(((y0 + y) * W + x0) * 4, ((y0 + y) * W + x0 + w) * 4), y * w * 4);
        return out;
      },
    });
    return () => { renderer.dispose(); rendererRef.current = null; };
  }, [engine]);

  // Size the viewport to the element.
  useEffect(() => {
    const el = glRef.current?.parentElement; if (!el) return;
    const observer = new ResizeObserver(() => {
      const s = useEditor.getState();
      for (const id of s.order) { const d = s.documents[id]; s.viewports[id].resize({ width: el.clientWidth, height: el.clientHeight }, window.devicePixelRatio || 1, { width: d.width, height: d.height }); }
      s.invalidate();
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, [activeId]);

  // Draw on every store change that affects the picture.
  useEffect(() => {
    const renderer = rendererRef.current; const gl = glRef.current; const overlay = overlayRef.current;
    if (!renderer || !gl || !overlay || !state || !viewport || !engine) return;
    const dpr = window.devicePixelRatio || 1;
    renderer.sync(engine, state);
    renderer.render(state, viewport, dpr, { checkerboard: checkerboardRef.current });
    overlay.width = gl.width; overlay.height = gl.height;
    drawOverlay(overlay.getContext("2d")!, viewport, dpr, { docWidth: state.width, docHeight: state.height, cropRect: tool === "crop" ? (cropRect ?? { x: 0, y: 0, width: state.width, height: state.height }) : null, guides: guidesRef.current });
  }, [state, viewport, cropRect, tool, renderTick, engine]);

  // Wheel: zoom with Ctrl, otherwise pan.
  useEffect(() => {
    const el = glRef.current?.parentElement; if (!el) return;
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      const s = useEditor.getState(); if (!s.activeId) return;
      const vp = s.viewports[s.activeId]; const d = s.documents[s.activeId];
      const r = el.getBoundingClientRect(); const anchor = { x: e.clientX - r.left, y: e.clientY - r.top };
      if (e.ctrlKey || e.metaKey) vp.setZoom(vp.zoom * Math.exp(-e.deltaY * 0.002), anchor, { width: d.width, height: d.height });
      else vp.translate({ width: -e.deltaX, height: -e.deltaY });
      s.invalidate();
    };
    el.addEventListener("wheel", onWheel, { passive: false });
    return () => el.removeEventListener("wheel", onWheel);
  }, []);

  // Drag to pan with the hand tool or the space bar.
  useEffect(() => {
    const el = glRef.current?.parentElement; if (!el) return;
    let last: { x: number; y: number } | null = null; let space = false;
    const down = (e: PointerEvent) => { const s = useEditor.getState(); if (s.tool === "hand" || space || e.button === 1) { last = { x: e.clientX, y: e.clientY }; el.setPointerCapture(e.pointerId); } };
    const move = (e: PointerEvent) => { if (!last) return; const s = useEditor.getState(); if (!s.activeId) return; s.viewports[s.activeId].translate({ width: e.clientX - last.x, height: e.clientY - last.y }); last = { x: e.clientX, y: e.clientY }; s.invalidate(); };
    const up = () => { last = null; };
    const key = (e: KeyboardEvent) => { if (e.code === "Space") space = e.type === "keydown"; };
    el.addEventListener("pointerdown", down); el.addEventListener("pointermove", move); el.addEventListener("pointerup", up);
    window.addEventListener("keydown", key); window.addEventListener("keyup", key);
    return () => { el.removeEventListener("pointerdown", down); el.removeEventListener("pointermove", move); el.removeEventListener("pointerup", up); window.removeEventListener("keydown", key); window.removeEventListener("keyup", key); };
  }, []);

  return (
    <div data-testid="canvas-view" style={{ position: "relative", flex: 1, minWidth: 0, minHeight: 0, overflow: "hidden", background: "#292929" }}>
      <canvas ref={glRef} style={{ position: "absolute", inset: 0, width: "100%", height: "100%" }} />
      <canvas ref={overlayRef} data-testid="overlay" style={{ position: "absolute", inset: 0, width: "100%", height: "100%", pointerEvents: "none" }} />
      {!state && <div style={{ position: "absolute", inset: 0, display: "grid", placeItems: "center", color: "#999" }}>Open an image or create a new canvas</div>}
    </div>
  );
}
