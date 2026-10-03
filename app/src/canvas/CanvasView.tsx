import { useEffect, useRef } from "react";
import { useEditor } from "../state/store";
import { createRenderer, type Renderer } from "./renderer";
import { drawOverlay, type AntsState } from "./overlay";
import { installTestApi } from "../test-api";
import { EFFECTS_LIMITS } from "./effects-images";
import { CropSession, hitTest, ratioValue, SNAP_SCREEN_PX } from "../tools/crop-tool";
import { TransformSession, startMode } from "../tools/transform-session";
import { containsPoint, cornersToTuples, fromTuple, hitOverlay, overlayGeometry, snapTargets, type OverlayGeometry, type P } from "../tools/transform-geometry";
import { activeLayer, canTransform, editedShape, transformsAsGroup } from "../state/selection";
import { beginBrush, moveBrush, finishBrush, cancelBrush,isBrushTool } from "../tools/brush";
import { antsDelay, AntsPathCache, ANTS_INTERVAL_MS, nextPhase, OutlineCache, outlineStep } from "./ants";
import { isSelectionTool, outlineOffset, SelectionDraft, selectionMode, type P as DocP } from "../tools/selection-draft";
import { installSampling } from "./sampling";
import { gradientLine, installGradientTool } from "./gradient-tool";
import { installShapeTool } from "./shape-tool";
import { beginShape, dragShape } from "../tools/shape-draft";
import { gradientStops } from "../state/gradient-edit";
import {beginText} from "../actions/text";

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
  const showGuides = useEditor((s) => s.showGuides);
  const selectedLayerIds = useEditor((s) => s.selectedLayerIds);
  const maskSelected = useEditor((s) => s.maskSelected);
  const sampleMode = useEditor((s) => s.adjustEdit?.sampleMode ?? null);
  const overlayTick = useEditor((s) => s.overlayTick);
  const selection = useEditor((s) => (s.activeId ? s.documents[s.activeId]?.selection ?? null : null));
  const outlineCacheRef = useRef(new OutlineCache());
  const antsPathRef = useRef(new AntsPathCache(() => new Path2D()));
  const antsPhaseRef = useRef(0);
  const brushHoverRef=useRef<{x:number;y:number}|null>(null);

  /** Draws the overlay from the store as it is now: the pixel grid, guides, the crop frame, the
   * transform handles, the marching ants and an outline being drawn. The ants' timer and an outline
   * drag repaint only this, never the picture beneath. */
  const paintOverlay = () => {
    const gl = glRef.current, overlay = overlayRef.current; const s = useEditor.getState();
    if (!gl || !overlay || !s.activeId || !s.engine) return;
    const doc = s.documents[s.activeId], vp = s.viewports[s.activeId];
    if (!doc || !vp) return;
    const dpr = window.devicePixelRatio || 1;
    // Assigning even an unchanged canvas size resets its backing store. Ants and
    // pointer overlays repaint often; clear their pixels without reallocating it.
    if (overlay.width !== gl.width) overlay.width = gl.width;
    if (overlay.height !== gl.height) overlay.height = gl.height;
    let transformGeometry: OverlayGeometry | null = null;
    if (s.tool === "move" && (s.transformEdit || canTransform(doc, s.selectedLayerIds, s.maskSelected))) {
      const shape = editedShape(doc, s.transformEdit, s.selectedLayerIds, s.maskSelected);
      if (shape) transformGeometry = overlayGeometry(shape.corners ?? shape.transform, vp, { width: doc.width, height: doc.height });
    }
    const sel = doc.selection;
    const engine = s.engine; const id = s.activeId;
    let ants: AntsState | null = null;
    if (sel && !sel.empty) {
      // The kept path, stroked about the document's scaled origin moved by any outline drag; the
      // view box, in the path's own coordinates, lets a very detailed outline keep only nearby edges.
      const step = outlineStep(vp.zoom);
      const outline = outlineCacheRef.current.get(id, sel.revision, step, () => engine.selectionOutline(id, step));
      const scale = vp.pointsPerPixel, origin = vp.documentRect({ width: doc.width, height: doc.height }), offset = s.outlineMove ?? { dx: 0, dy: 0 };
      const at = { x: origin.x + offset.dx * scale, y: origin.y + offset.dy * scale };
      const path = antsPathRef.current.get(outline, scale, { x0: -at.x, y0: -at.y, x1: vp.viewSize.width - at.x, y1: vp.viewSize.height - at.y });
      ants = { path, at, phase: antsPhaseRef.current };
    }
    const d = s.selectionDraft;
    const line = gradientLine();
    // The palette's black or white while a mask is the target (fix round 1, I-2), as the discs on the
    // Mac's overlay show (TransformOverlay.swift:421, ColorPalette.swift:26-28).
    const [from, to] = gradientStops(s.gradientOptions, s.paletteColor(false), s.paletteColor(true));
    drawOverlay(overlay.getContext("2d")!, vp, dpr, {
      docWidth: doc.width, docHeight: doc.height, cropRect: s.tool === "crop" ? s.cropRect : null, guides: s.snapGuides,
      transform: transformGeometry, canvasGuides: s.showGuides ? doc.guides : null,
      ants, draft: d ? { kind: d.kind, points: d.points, cursor: d.cursor } : null, sampleRing: s.sampleRing,
      gradientLine: line ? { ...line, radial: s.gradientOptions.shape === "Radial", from, to } : null,
      shapeDraft: s.shapeDraft ? { kind: s.shapeDraft.kind, rect: s.shapeDraft.rect, start: s.shapeDraft.anchor, end: s.shapeDraft.end,
        cornerRadius: s.shapeDraft.cornerRadius, lineWidth: s.shapeOptions.lineWidth, color: s.palette.foreground } : null,
    });
    const text=s.textEdit;
    if(text?.document===id&&!text.layer&&text.preview&&text.bitmap){
      const ctx=overlay.getContext("2d")!,canvas=vp.documentRect(doc),p=vp.viewPoint({x:text.origin[0],y:text.origin[1]},doc);
      ctx.save();ctx.setTransform(dpr,0,0,dpr,0,0);ctx.beginPath();ctx.rect(canvas.x,canvas.y,canvas.width,canvas.height);ctx.clip();
      ctx.drawImage(text.bitmap,p.x,p.y,text.bitmap.width*vp.pointsPerPixel,text.bitmap.height*vp.pointsPerPixel);ctx.restore();
    }
    if (s.brushDraft?.document===id) {
      const ctx=overlay.getContext("2d")!, b=s.brushDraft, canvas=vp.documentRect({width:doc.width,height:doc.height});
      ctx.save();ctx.setTransform(dpr,0,0,dpr,0,0);ctx.beginPath();ctx.rect(canvas.x,canvas.y,canvas.width,canvas.height);ctx.clip();
      const c=b.operation.kind!=="Paint"?[0,0,0,0.45]:b.erasing ? [1,1,1,1] : b.color;
      ctx.strokeStyle=ctx.fillStyle=`rgba(${Math.round(c[0]*255)},${Math.round(c[1]*255)},${Math.round(c[2]*255)},${b.operation.kind!=="Paint"?0.45:s.brushOptions.opacity})`;
      ctx.lineWidth=s.brushOptions.diameter*vp.pointsPerPixel;ctx.lineCap="round";ctx.lineJoin="round";
      const points=b.points.map(([x,y])=>vp.viewPoint({x,y},{width:doc.width,height:doc.height}));
      ctx.beginPath();
      if(points.length===1){ctx.arc(points[0].x,points[0].y,ctx.lineWidth/2,0,Math.PI*2);ctx.fill();}
      else{ctx.moveTo(points[0].x,points[0].y);for(const p of points.slice(1))ctx.lineTo(p.x,p.y);ctx.stroke();}
      ctx.restore();
    }
    const hover=brushHoverRef.current;
    if(isBrushTool(s.tool)&&hover&&!s.colorPicker){
      const ctx=overlay.getContext("2d")!;ctx.save();ctx.setTransform(dpr,0,0,dpr,0,0);
      ctx.beginPath();ctx.arc(hover.x,hover.y,Math.max(2,s.brushOptions.diameter*vp.pointsPerPixel/2),0,Math.PI*2);
      ctx.lineWidth=2;ctx.strokeStyle="rgba(0,0,0,.7)";ctx.stroke();ctx.lineWidth=1;ctx.strokeStyle="white";ctx.stroke();
      if(s.tool==="clone"&&s.cloneSource?.document===id){
        const q=vp.documentPoint(hover,doc),offset=s.brushDraft?.operation.kind==="Clone"?s.brushDraft.operation.offset:s.brushOptions.aligned?s.cloneOffset:null;
        const source=offset?{x:q.x+offset[0],y:q.y+offset[1]}:{x:s.cloneSource.point[0],y:s.cloneSource.point[1]};
        const v=vp.viewPoint(source,doc);ctx.beginPath();ctx.moveTo(v.x-6,v.y);ctx.lineTo(v.x+6,v.y);ctx.moveTo(v.x,v.y-6);ctx.lineTo(v.x,v.y+6);ctx.stroke();
      }
      ctx.restore();
    }
  };

  // Renderer lifetime follows the canvas element.
  useEffect(() => {
    if (!engine || !glRef.current) return;
    const renderer = createRenderer(glRef.current, { jobs: () => useEditor.getState().jobs, landed: () => useEditor.getState().invalidate(),
      failed: (message) => useEditor.getState().setError(message) });
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
      // Where large styled layers' effects images are made (tests lower the sizes to reach the worker).
      effectsLimits: EFFECTS_LIMITS,
      // The overlay painted now, synchronously: the perf harness times an ants tick with it.
      paintOverlay: () => paintOverlay(),
      // Exposed so the perf harness can time a Shape tool draft tick (`setShapeDraft(dragShape(...))`)
      // exactly as the real pointer path (canvas/shape-tool.ts) computes it (ruling R16-3).
      beginShape, dragShape,
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
    renderer.render(engine, state, viewport, dpr, { checkerboard: checkerboardRef.current }, useEditor.getState().previewEdit());
    paintOverlay();
  }, [state, viewport, cropRect, tool, renderTick, engine, transformEdit, snapGuides, selectedLayerIds, maskSelected, showGuides]);

  // An outline being drawn or dragged repaints the overlay alone.
  useEffect(() => { paintOverlay(); }, [overlayTick]);

  // Marching ants: march every 120 ms, only while a selection with something in it exists
  // (`updateAntsTimer`, EditorCanvas.swift:2004-2020). Each step waits until the frame that drew
  // the last one is done, then `antsDelay` of what that took: an outline slow to stroke (millions of
  // edges) marches slower instead of holding the page up step after step (final review F3).
  const antsActive = !!selection && !selection.empty;
  useEffect(() => {
    if (!antsActive) return;
    let timer = 0, frame = 0;
    const step = () => {
      antsPhaseRef.current = nextPhase(antsPhaseRef.current);
      const painted = performance.now();
      paintOverlay();
      // The first frame after this draws the step; the second starts once that one is done.
      frame = requestAnimationFrame(() => { frame = requestAnimationFrame((t) => { timer = window.setTimeout(step, antsDelay(t - painted)); }); });
    };
    timer = window.setTimeout(step, ANTS_INTERVAL_MS);
    return () => { clearTimeout(timer); cancelAnimationFrame(frame); };
  }, [antsActive, activeId]);

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

  // Sampling into the colour picker while it is open (canvas/sampling.ts): ahead of every tool gesture.
  useEffect(() => {
    const el = glRef.current?.parentElement; if (!el) return;
    return installSampling(el, () => spaceRef.current);
  }, []);

  // The Shape tool's drag (canvas/shape-tool.ts).
  useEffect(() => {
    const el = glRef.current?.parentElement; if (!el) return;
    return installShapeTool(el, () => spaceRef.current);
  }, []);

  // The Gradient tool's line (canvas/gradient-tool.ts).
  useEffect(() => {
    const el = glRef.current?.parentElement; if (!el) return;
    return installGradientTool(el, () => spaceRef.current);
  }, []);

  // Adjustment eyedroppers: while a sample mode is armed (Levels' three, or Hue/Saturation's
  // replace/add/remove), a click reads the point under the cursor and feeds the open panel
  // instead of starting whatever gesture the active tool would otherwise begin. Registered with
  // `capture: true` so it runs before the pan/zoom/crop/move handlers below regardless of which
  // tool happens to be selected, and `stopImmediatePropagation` keeps the event from reaching
  // them at all -- no tool gesture starts.
  useEffect(() => {
    const el = glRef.current?.parentElement; if (!el) return;
    const down = (e: PointerEvent) => {
      const s = useEditor.getState();
      if (!s.adjustEdit?.sampleMode || !s.activeId) return;
      const vp = s.viewports[s.activeId]; const d = s.documents[s.activeId];
      const r = el.getBoundingClientRect();
      const at = vp.documentPoint({ x: e.clientX - r.left, y: e.clientY - r.top }, { width: d.width, height: d.height });
      s.sampleAt(at);
      e.stopImmediatePropagation();
    };
    el.addEventListener("pointerdown", down, { capture: true });
    return () => el.removeEventListener("pointerdown", down, { capture: true });
  }, []);

  // The cursor shows an armed eyedropper regardless of which tool is otherwise selected, and a
  // crosshair for the selection tools.
  const picking = useEditor((s) => !!s.colorPicker);
  useEffect(() => {
    const el = glRef.current?.parentElement; if (!el) return;
    el.style.cursor = tool==="text"?"text":sampleMode || picking || tool === "eyedropper" || tool === "gradient" || tool === "shape" || isSelectionTool(tool) ? "crosshair" : "";
  }, [sampleMode, tool, picking]);

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

  useEffect(()=>{
    const el=glRef.current?.parentElement;if(!el)return;let start:{point:[number,number];document:string;pointer:number}|null=null;
    const point=(e:PointerEvent):[number,number]=>{const s=useEditor.getState(),id=s.activeId!,d=s.documents[id],r=el.getBoundingClientRect(),p=s.viewports[id].documentPoint({x:e.clientX-r.left,y:e.clientY-r.top},d);return[p.x,p.y];};
    const down=(e:PointerEvent)=>{const s=useEditor.getState();if(s.tool!=="text"||!s.activeId||s.working||s.panelOwnsDocument()||spaceRef.current||e.button!==0)return;start={point:point(e),document:s.activeId,pointer:e.pointerId};el.setPointerCapture(e.pointerId);e.preventDefault();};
    const up=(e:PointerEvent)=>{if(!start||e.pointerId!==start.pointer)return;const a=start;start=null;if(useEditor.getState().activeId!==a.document)return;const p=point(e),w=Math.abs(p[0]-a.point[0]),h=Math.abs(p[1]-a.point[1]);beginText(w>=16&&h>=16?[Math.min(a.point[0],p[0]),Math.min(a.point[1],p[1])]:a.point,w>=16&&h>=16?[Math.round(w),Math.round(h)]:undefined);};
    const cancel=()=>{start=null;};el.addEventListener("pointerdown",down);el.addEventListener("pointerup",up);el.addEventListener("pointercancel",cancel);
    return()=>{el.removeEventListener("pointerdown",down);el.removeEventListener("pointerup",up);el.removeEventListener("pointercancel",cancel);};
  },[]);

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

  useEffect(()=>{
    const el=glRef.current?.parentElement;if(!el)return;
    let pointer: number|null=null;
    const point=(e:PointerEvent):[number,number]=>{
      const s=useEditor.getState(),id=s.activeId!,vp=s.viewports[id],d=s.documents[id],r=el.getBoundingClientRect();
      const p=vp.documentPoint({x:e.clientX-r.left,y:e.clientY-r.top},{width:d.width,height:d.height});return[p.x,p.y];
    };
    const down=(e:PointerEvent)=>{const s=useEditor.getState();if(e.button!==0||spaceRef.current||!s.activeId||!isBrushTool(s.tool))return;
      if(e.altKey){const [x,y]=point(e);if(s.tool==="clone"){useEditor.setState({cloneSource:{document:s.activeId,point:[x,y]},cloneOffset:null});s.invalidateOverlay();}else s.sampleForeground({x,y});return;}
      if(beginBrush(point(e),e.shiftKey)){pointer=e.pointerId;el.setPointerCapture(e.pointerId);e.preventDefault();}
    };
    const move=(e:PointerEvent)=>{const s=useEditor.getState();if(isBrushTool(s.tool)){const r=el.getBoundingClientRect();brushHoverRef.current={x:e.clientX-r.left,y:e.clientY-r.top};s.invalidateOverlay();}if(pointer===e.pointerId&&s.brushDraft)moveBrush(point(e));};
    const leave=()=>{brushHoverRef.current=null;useEditor.getState().invalidateOverlay();};
    const up=(e:PointerEvent)=>{if(pointer!==e.pointerId)return;pointer=null;finishBrush();};
    const cancel=()=>{pointer=null;cancelBrush();};
    el.addEventListener("pointerdown",down);el.addEventListener("pointermove",move);el.addEventListener("pointerup",up);el.addEventListener("pointercancel",cancel);el.addEventListener("pointerleave",leave);
    return()=>{el.removeEventListener("pointerdown",down);el.removeEventListener("pointermove",move);el.removeEventListener("pointerup",up);el.removeEventListener("pointercancel",cancel);el.removeEventListener("pointerleave",leave);};
  },[]);

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
      // An unlinked mask alone, either already mid-edit or about to start one: Alt should not
      // duplicate the layer (it should move the mask), and Ctrl on a corner should not start a
      // distortion (masks never carry corners, so that drag would silently do nothing).
      const activeLayerForEdit = activeLayer(d);
      const maskAlone = s0.maskSelected && !!activeLayerForEdit && activeLayerForEdit.hasMask && !activeLayerForEdit.maskLinked;
      const isMaskEdit = s0.transformEdit ? s0.transformEdit.kind === "mask" : maskAlone;
      const geometry = overlayGeometry(edited.corners ?? edited.transform, vp, { width: d.width, height: d.height });
      const r = el.getBoundingClientRect(); const view = { x: e.clientX - r.left, y: e.clientY - r.top };
      const mode = startMode(hitOverlay(geometry, view), containsPoint(edited.transform, docPoint(e)), e.ctrlKey && !isMaskEdit, !!edited.corners);
      if (!s0.transformEdit) {
        const started = s0.beginTransform({ persistent: false, duplicate: e.altKey && mode.kind === "move" && !transformsAsGroup(d, s0.selectedLayerIds) && !maskAlone });
        if (!started) return;
      }
      const te = useEditor.getState().transformEdit!;
      if (mode.kind === "distort" && !te.corners) useEditor.getState().beginDistort();
      const after = useEditor.getState().transformEdit!;
      const tolerance = 10 / vp.pointsPerPixel;
      // Re-read the document: beginTransform may have just duplicated the layer, adding it
      // to the layers array snapTargets scans (and excludes by movingIds).
      const targets = snapTargets(useEditor.getState().documents[s0.activeId!], after.ids);
      session = new TransformSession({ mode, startDoc: docPoint(e), original: after.draft, originalCorners: after.corners ? after.corners.map(fromTuple) : null, snap: mode.kind === "move" || mode.kind === "resize" ? { ...targets, tolerance } : null, lockRatio: useEditor.getState().locksTransformRatio });
      el.setPointerCapture(e.pointerId);
    };
    const move = (e: PointerEvent) => {
      if (!session) return;
      // Something closed the edit under us: a history-recording command (a bare opacity digit,
      // say) commits the pending transform, and shortcuts stay live while the pointer is
      // captured. `previewTransform` would already be a no-op, but the guides would keep being
      // recomputed and redrawn for a drag that is over, until pointerup. End the drag instead.
      if (!useEditor.getState().transformEdit) {
        session = null;
        useEditor.getState().setSnapGuides({ xs: [], ys: [] });
        return;
      }
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

  // The selection tools (Phase 4a; EditorCanvas.swift:1961-2002, :1513-1531, :1696-1716). A press
  // picks its mode from Shift / Alt; in New mode, inside a selection, it drags the outline instead
  // (one MoveSelection on release; a click without a drag deselects, or with the wand selects afresh
  // at that pixel). The Marquee and the Freehand Lasso draw while dragged and finish on release; the
  // Polygonal Lasso adds a corner per click and closes on its first corner or a double-click; the
  // Magic Wand selects at the click.
  useEffect(() => {
    const el = glRef.current?.parentElement; if (!el) return;
    let moveStart: DocP | null = null;
    let lastPixel: DocP | null = null;
    const view = (e: { clientX: number; clientY: number }) => { const r = el.getBoundingClientRect(); return { x: e.clientX - r.left, y: e.clientY - r.top }; };
    const docPoint = (e: { clientX: number; clientY: number }): DocP => {
      const s = useEditor.getState(); const vp = s.viewports[s.activeId!]; const d = s.documents[s.activeId!];
      return vp.documentPoint(view(e), { width: d.width, height: d.height });
    };
    // A drag that ended without its pointerup (pointercancel, lost capture, a release the page never
    // saw) is forgotten without committing anything: no outline move, no half-drawn Marquee or
    // Freehand outline (final review F4). The Polygonal Lasso's corners stay: it spans presses.
    const forget = () => {
      const s = useEditor.getState();
      lastPixel = null;
      if (moveStart) { moveStart = null; s.setOutlineMove(null); }
      if (s.selectionDraft && s.selectionDraft.kind !== "Polygonal") s.setSelectionDraft(null);
    };
    const down = (e: PointerEvent) => {
      forget();
      const s = useEditor.getState();
      if (!isSelectionTool(s.tool) || e.button !== 0 || spaceRef.current || !s.activeId || !s.engine) return;
      if (s.panelOwnsDocument(true)) return;
      const d = s.documents[s.activeId]; const vp = s.viewports[s.activeId];
      const pixel = docPoint(e);
      const draft = s.selectionDraft;
      if (draft?.kind === "Polygonal") {
        const first = vp.viewPoint(draft.points[0], { width: d.width, height: d.height });
        if (draft.click(pixel, view(e), first, 1) === "close") s.finishSelectionDraft(); else s.setSelectionDraft(draft);
        return;
      }
      const o = s.selectionOptions;
      const mode = selectionMode(o.mode, e.shiftKey, e.altKey);
      if (mode === "Replace" && s.engine.selectionContains(s.activeId, pixel)) {
        moveStart = pixel;
        s.setOutlineMove({ dx: 0, dy: 0 });
        el.setPointerCapture(e.pointerId);
        return;
      }
      if (s.tool === "wand") {
        // Off the canvas there is no colour to match (`magicWand(at:)` reads the canvas-sized sample).
        if (pixel.x >= 0 && pixel.y >= 0 && pixel.x < d.width && pixel.y < d.height) {
          s.run({ type: "MagicWand", at: [pixel.x, pixel.y], mode, settings: o.wand, antialiased: o.antialiased });
        }
        return;
      }
      lastPixel = pixel;
      s.setSelectionDraft(SelectionDraft.begin(s.tool === "marquee" ? o.marquee : o.lasso, mode, pixel, e.shiftKey));
      el.setPointerCapture(e.pointerId);
    };
    const move = (e: PointerEvent) => {
      const s = useEditor.getState();
      if (!isSelectionTool(s.tool) || !s.activeId) return;
      const pixel = docPoint(e);
      if (moveStart) { s.setOutlineMove(outlineOffset(moveStart, pixel, e.shiftKey)); return; }
      const draft = s.selectionDraft; if (!draft) return;
      if (draft.kind === "Polygonal") { draft.moveCursor(pixel); s.setSelectionDraft(draft); return; }
      if (!el.hasPointerCapture(e.pointerId)) return;
      lastPixel = pixel;
      if (draft.isMarquee) draft.dragMarquee(pixel, e.shiftKey); else draft.extend(pixel);
      s.setSelectionDraft(draft);
    };
    const up = () => {
      const s = useEditor.getState();
      if (moveStart) {
        const start = moveStart, offset = s.outlineMove;
        moveStart = null;
        s.setOutlineMove(null);
        if (offset && (offset.dx !== 0 || offset.dy !== 0)) s.run({ type: "MoveSelection", dx: offset.dx, dy: offset.dy });
        else if (s.tool === "wand") s.run({ type: "MagicWand", at: [start.x, start.y], mode: "Replace", settings: s.selectionOptions.wand, antialiased: s.selectionOptions.antialiased });
        else s.run({ type: "Deselect" });
        return;
      }
      lastPixel = null;
      const draft = s.selectionDraft;
      if (draft && draft.kind !== "Polygonal") s.finishSelectionDraft();
    };
    const dblclick = () => { const s = useEditor.getState(); if (s.selectionDraft?.kind === "Polygonal") s.finishSelectionDraft(); };
    // Shift pressed or let go mid-drag reshapes the Marquee at once, without waiting for the
    // pointer to move (EditorCanvas.swift:611-619); Shift / Alt held show in the options bar.
    const keys = (e: KeyboardEvent) => {
      const s = useEditor.getState();
      if (!isSelectionTool(s.tool)) return;
      s.setHeldSelectionMode(e.altKey ? "Subtract" : e.shiftKey ? "Add" : null);
      const draft = s.selectionDraft;
      if (e.key === "Shift" && draft?.isMarquee && lastPixel) { draft.dragMarquee(lastPixel, e.type === "keydown"); s.setSelectionDraft(draft); }
    };
    const blur = () => useEditor.getState().setHeldSelectionMode(null);
    el.addEventListener("pointerdown", down); el.addEventListener("pointermove", move); el.addEventListener("pointerup", up); el.addEventListener("dblclick", dblclick);
    // After a pointerup these find nothing left to forget; without one they end the drag.
    el.addEventListener("pointercancel", forget); el.addEventListener("lostpointercapture", forget);
    window.addEventListener("keydown", keys); window.addEventListener("keyup", keys); window.addEventListener("blur", blur);
    return () => {
      el.removeEventListener("pointerdown", down); el.removeEventListener("pointermove", move); el.removeEventListener("pointerup", up); el.removeEventListener("dblclick", dblclick);
      el.removeEventListener("pointercancel", forget); el.removeEventListener("lostpointercapture", forget);
      window.removeEventListener("keydown", keys); window.removeEventListener("keyup", keys); window.removeEventListener("blur", blur);
    };
  }, []);

  return (
    <div data-testid="canvas-view" style={{ position: "relative", flex: 1, minWidth: 0, minHeight: 0, overflow: "hidden", background: "#292929" }}>
      <canvas ref={glRef} style={{ position: "absolute", inset: 0, width: "100%", height: "100%" }} />
      <canvas ref={overlayRef} data-testid="overlay" style={{ position: "absolute", inset: 0, width: "100%", height: "100%", pointerEvents: "none" }} />
      {!state && <div style={{ position: "absolute", inset: 0, display: "grid", placeItems: "center", color: "#999" }}>Open an image or create a new canvas</div>}
    </div>
  );
}
