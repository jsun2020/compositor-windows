import { useEffect, useRef } from "react";
import { useEditor } from "../state/store";
import { createRenderer, type Renderer } from "./renderer";
import { drawOverlay } from "./overlay";
import { installTestApi } from "../test-api";
import { CropSession, hitTest, ratioValue, SNAP_SCREEN_PX } from "../tools/crop-tool";
import { TransformSession, startMode } from "../tools/transform-session";
import { containsPoint, cornersToTuples, fromTuple, hitOverlay, overlayGeometry, snapTargets, type OverlayGeometry, type P } from "../tools/transform-geometry";
import { canTransform, editedShape, transformsAsGroup } from "../state/selection";

export const HIT_HANDLE_PX = 6;

export function CanvasView() {
  const glRef = useRef<HTMLCanvasElement>(null);
  const overlayRef = useRef<HTMLCanvasElement>(null);
  const rendererRef = useRef<Renderer | null>(null);
  const checkerboardRef = useRef(true);
  // Whether the space bar is currently held, shared by the pan and crop pointer
  // handlers below (both treat space-held as "temporarily pan" regardless of tool).
  const spaceRef = useRef(false);
  const engine = useEditor((s) => s.engine);
  const activeId = useEditor((s) => s.activeId);
  const state = useEditor((s) => (s.activeId ? s.documents[s.activeId] : null));
  const viewport = useEditor((s) => (s.activeId ? s.viewports[s.activeId] : null));
  const cropRect = useEditor((s) => s.cropRect);
  const tool = useEditor((s) => s.tool);
  const renderTick = useEditor((s) => s.renderTick);
  const transformEdit = useEditor((s) => s.transformEdit);
  const snapGuides = useEditor((s) => s.snapGuides);
  const selectedLayerIds = useEditor((s) => s.selectedLayerIds);
  const maskSelected = useEditor((s) => s.maskSelected);

  // Renderer lifetime follows the canvas element.
  useEffect(() => {
    if (!engine || !glRef.current) return;
    const renderer = createRenderer(glRef.current);
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
      // Exposed for e2e tests to compute where the on-screen transform handles (including the
      // rotation handle, offset above the shape) currently sit, rather than hard-coding an
      // assumed screen offset that would break if the handle geometry ever changes.
      transformGeometry: (): OverlayGeometry | null => {
        const s = useEditor.getState(); if (!s.activeId) return null;
        const d = s.documents[s.activeId]; const vp = s.viewports[s.activeId];
        const shape = editedShape(d, s.transformEdit, s.selectedLayerIds, s.maskSelected);
        if (!shape) return null;
        return overlayGeometry(shape.corners ?? shape.transform, vp, { width: d.width, height: d.height });
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
    renderer.render(engine, state, viewport, dpr, { checkerboard: checkerboardRef.current }, useEditor.getState().previewEdit());
    overlay.width = gl.width; overlay.height = gl.height;
    let transformGeometry: OverlayGeometry | null = null;
    if (tool === "move" && (transformEdit || canTransform(state, selectedLayerIds, maskSelected))) {
      const shape = editedShape(state, transformEdit, selectedLayerIds, maskSelected);
      if (shape) transformGeometry = overlayGeometry(shape.corners ?? shape.transform, viewport, { width: state.width, height: state.height });
    }
    drawOverlay(overlay.getContext("2d")!, viewport, dpr, { docWidth: state.width, docHeight: state.height, cropRect: tool === "crop" ? (cropRect ?? { x: 0, y: 0, width: state.width, height: state.height }) : null, guides: snapGuides, transform: transformGeometry });
  }, [state, viewport, cropRect, tool, renderTick, engine, transformEdit, snapGuides, selectedLayerIds, maskSelected]);

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

  // Track the space bar globally; shared by the pan and crop pointer handlers below,
  // instead of each keeping its own duplicate keydown/keyup listener and local flag.
  useEffect(() => {
    const key = (e: KeyboardEvent) => { if (e.code === "Space") spaceRef.current = e.type === "keydown"; };
    // Holding space, Alt-Tabbing away and releasing space in another window leaves no keyup
    // event for this window to see, so also drop the flag when the window loses focus.
    const blur = () => { spaceRef.current = false; };
    window.addEventListener("keydown", key); window.addEventListener("keyup", key); window.addEventListener("blur", blur);
    return () => { window.removeEventListener("keydown", key); window.removeEventListener("keyup", key); window.removeEventListener("blur", blur); };
  }, []);

  // Drag to pan with the hand tool or the space bar.
  useEffect(() => {
    const el = glRef.current?.parentElement; if (!el) return;
    let last: { x: number; y: number } | null = null;
    const down = (e: PointerEvent) => { const s = useEditor.getState(); if (s.tool === "hand" || spaceRef.current || e.button === 1) { last = { x: e.clientX, y: e.clientY }; el.setPointerCapture(e.pointerId); } };
    const move = (e: PointerEvent) => { if (!last) return; const s = useEditor.getState(); if (!s.activeId) return; s.viewports[s.activeId].translate({ width: e.clientX - last.x, height: e.clientY - last.y }); last = { x: e.clientX, y: e.clientY }; s.invalidate(); };
    const up = () => { last = null; };
    el.addEventListener("pointerdown", down); el.addEventListener("pointermove", move); el.addEventListener("pointerup", up);
    return () => { el.removeEventListener("pointerdown", down); el.removeEventListener("pointermove", move); el.removeEventListener("pointerup", up); };
  }, []);

  // Zoom tool: a click (no drag) zooms in at the pointer by 1.5x; Alt-click zooms out.
  useEffect(() => {
    const el = glRef.current?.parentElement; if (!el) return;
    let start: { x: number; y: number } | null = null;
    const down = (e: PointerEvent) => { if (useEditor.getState().tool === "zoom" && e.button === 0) start = { x: e.clientX, y: e.clientY }; };
    const up = (e: PointerEvent) => {
      if (!start) return;
      const moved = Math.hypot(e.clientX - start.x, e.clientY - start.y);
      start = null;
      if (moved > 4) return;
      const s = useEditor.getState();
      if (s.tool !== "zoom" || !s.activeId) return;
      const vp = s.viewports[s.activeId]; const d = s.documents[s.activeId];
      const r = el.getBoundingClientRect();
      const viewPoint = { x: e.clientX - r.left, y: e.clientY - r.top };
      vp.setZoom(vp.zoom * (e.altKey ? 1 / 1.5 : 1.5), viewPoint, { width: d.width, height: d.height });
      s.invalidate();
    };
    el.addEventListener("pointerdown", down); el.addEventListener("pointerup", up);
    return () => { el.removeEventListener("pointerdown", down); el.removeEventListener("pointerup", up); };
  }, []);

  // Crop tool: drag to create, move or resize the crop rect, snapping to canvas and layer edges.
  useEffect(() => {
    const el = glRef.current?.parentElement; if (!el) return;
    let session: CropSession | null = null;
    const point = (e: PointerEvent) => { const r = el.getBoundingClientRect(); return { x: e.clientX - r.left, y: e.clientY - r.top }; };
    const down = (e: PointerEvent) => {
      const s = useEditor.getState();
      if (s.tool !== "crop" || spaceRef.current || e.button !== 0 || !s.activeId) return;
      const vp = s.viewports[s.activeId]; const d = s.documents[s.activeId];
      const size = { width: d.width, height: d.height };
      const view = point(e);
      const docPoint = vp.documentPoint(view, size);
      const rect = s.cropRect ?? { x: 0, y: 0, width: d.width, height: d.height };
      const mode = hitTest(rect, view, vp, size);
      const tolerance = SNAP_SCREEN_PX / vp.pointsPerPixel;
      session = new CropSession(mode, docPoint, rect, d, ratioValue(s.cropRatio, d), tolerance);
      el.setPointerCapture(e.pointerId);
    };
    const move = (e: PointerEvent) => {
      if (!session) return;
      const s = useEditor.getState(); if (!s.activeId) return;
      const vp = s.viewports[s.activeId]; const d = s.documents[s.activeId];
      const docPoint = vp.documentPoint(point(e), { width: d.width, height: d.height });
      const { rect, guides } = session.update(docPoint, e.altKey);
      s.setSnapGuides(guides);
      s.setCropRect(rect);
      s.invalidate();
    };
    const up = () => { if (!session) return; session = null; useEditor.getState().setSnapGuides({ xs: [], ys: [] }); useEditor.getState().invalidate(); };
    el.addEventListener("pointerdown", down); el.addEventListener("pointermove", move); el.addEventListener("pointerup", up);
    return () => { el.removeEventListener("pointerdown", down); el.removeEventListener("pointermove", move); el.removeEventListener("pointerup", up); };
  }, []);

  // Move tool: drag to move (a press outside the shape still moves it, as macOS does),
  // drag a handle to resize or rotate, Ctrl-drag a corner to distort, Alt-drag to
  // duplicate; snaps to the canvas edges/centre and other layers' bounds while moving.
  useEffect(() => {
    const el = glRef.current?.parentElement; if (!el) return;
    let session: TransformSession | null = null;
    const docPoint = (e: PointerEvent): P => {
      const s = useEditor.getState(); const vp = s.viewports[s.activeId!]; const d = s.documents[s.activeId!];
      const r = el.getBoundingClientRect();
      return vp.documentPoint({ x: e.clientX - r.left, y: e.clientY - r.top }, { width: d.width, height: d.height });
    };
    const down = (e: PointerEvent) => {
      const s0 = useEditor.getState();
      if (s0.tool !== "move" || e.button !== 0 || spaceRef.current || !s0.activeId) return;
      const d = s0.documents[s0.activeId]; const vp = s0.viewports[s0.activeId];
      if (!canTransform(d, s0.selectedLayerIds, s0.maskSelected) && !s0.transformEdit) return;
      const edited = editedShape(d, s0.transformEdit, s0.selectedLayerIds, s0.maskSelected);
      if (!edited) return;
      const geometry = overlayGeometry(edited.corners ?? edited.transform, vp, { width: d.width, height: d.height });
      const r = el.getBoundingClientRect(); const view = { x: e.clientX - r.left, y: e.clientY - r.top };
      const mode = startMode(hitOverlay(geometry, view), containsPoint(edited.transform, docPoint(e)), e.ctrlKey, !!edited.corners);
      if (!s0.transformEdit) {
        const started = s0.beginTransform({ persistent: false, duplicate: e.altKey && mode.kind === "move" && !transformsAsGroup(d, s0.selectedLayerIds) });
        if (!started) return;
      }
      const te = useEditor.getState().transformEdit!;
      if (mode.kind === "distort" && !te.corners) useEditor.getState().beginDistort();
      const after = useEditor.getState().transformEdit!;
      const tolerance = 10 / vp.pointsPerPixel;
      // Re-read the document: beginTransform may have just duplicated the layer, adding it
      // to the layers array snapTargets scans (and excludes by movingIds).
      const targets = snapTargets(useEditor.getState().documents[s0.activeId!], after.ids);
      session = new TransformSession({ mode, startDoc: docPoint(e), original: after.draft, originalCorners: after.corners ? after.corners.map(fromTuple) : null, snap: mode.kind === "move" ? { ...targets, tolerance } : null });
      el.setPointerCapture(e.pointerId);
    };
    const move = (e: PointerEvent) => {
      if (!session) return;
      const r = session.update(docPoint(e), { shift: e.shiftKey, alt: e.altKey, ctrl: e.ctrlKey });
      const s = useEditor.getState();
      s.previewTransform(r.draft, r.corners ? cornersToTuples(r.corners) : null);
      s.setSnapGuides(r.guides);
    };
    const up = () => {
      if (!session) return;
      session = null;
      const s = useEditor.getState();
      s.setSnapGuides({ xs: [], ys: [] });
      if (!s.transformEdit?.corners) s.commitTransform();
      else s.invalidate();
    };
    el.addEventListener("pointerdown", down); el.addEventListener("pointermove", move); el.addEventListener("pointerup", up);
    return () => { el.removeEventListener("pointerdown", down); el.removeEventListener("pointermove", move); el.removeEventListener("pointerup", up); };
  }, []);

  return (
    <div data-testid="canvas-view" style={{ position: "relative", flex: 1, minWidth: 0, minHeight: 0, overflow: "hidden", background: "#292929" }}>
      <canvas ref={glRef} style={{ position: "absolute", inset: 0, width: "100%", height: "100%" }} />
      <canvas ref={overlayRef} data-testid="overlay" style={{ position: "absolute", inset: 0, width: "100%", height: "100%", pointerEvents: "none" }} />
      {!state && <div style={{ position: "absolute", inset: 0, display: "grid", placeItems: "center", color: "#999" }}>Open an image or create a new canvas</div>}
    </div>
  );
}
