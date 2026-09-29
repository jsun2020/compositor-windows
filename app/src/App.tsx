import { useEffect, useRef, useState } from "react";
import { EngineClient } from "./engine/client";
import { JobClient } from "./engine/jobs";
import { BUILD_MARKER } from "./build-info";
import { installTestApi } from "./test-api";
import { useEditor } from "./state/store";
import { getBridge } from "./shell/bridge";
import { importImages, openProject } from "./actions/files";
import { useShortcuts } from "./shortcuts/useShortcuts";
import { CanvasView } from "./canvas/CanvasView";
import { MenuBar } from "./panels/MenuBar";
import { ProjectTabs } from "./panels/ProjectTabs";
import { LayersList } from "./panels/LayersList";
import { LayerProperties } from "./panels/LayerProperties";
import { ToolRail } from "./panels/ToolRail";
import { NewCanvasSheet } from "./sheets/NewCanvasSheet";
import { CanvasSizeSheet } from "./sheets/CanvasSizeSheet";
import { ImageSizeSheet } from "./sheets/ImageSizeSheet";
import { JpegExportSheet } from "./sheets/JpegExportSheet";
import { CropOptions } from "./panels/CropOptions";
import { SelectionOptions } from "./panels/SelectionOptions";
import { GradientOptions } from "./panels/GradientOptions";
import { ShapeOptions } from "./panels/ShapeOptions";
import { SelectionAmountSheet } from "./sheets/SelectionAmountSheet";
import { TransformInspector } from "./panels/TransformInspector";
import { AdjustPanel } from "./panels/AdjustPanel";
import { UndrawnNotice } from "./panels/UndrawnNotice";
import { ColorPickerPanel } from "./panels/ColorPickerPanel";
import "./styles.css";

export function App() {
  const [ready, setReady] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // Cached once instead of re-read from JSX on every render (see the loadedRef note
  // below for why a second live wasm call at the wrong moment is unsafe here).
  const [version, setVersion] = useState("");
  const sheet = useEditor((s) => s.sheet);
  const banner = useEditor((s) => s.error);
  const working = useEditor((s) => s.working);
  // React 18 StrictMode double-invokes effects in dev (which is what `pnpm dev` - and
  // so every e2e run - uses). This effect has no cleanup, so without a guard that
  // double-invoke calls EngineClient.load() twice, constructing two WasmEngine
  // instances that share one wasm module/memory. Only the second is ever stored, so the
  // first sits orphaned; whenever V8 later garbage-collects it, wasm-bindgen's
  // generated finalizer frees its pointer, which corrupts the live instance's
  // Rc-refcount bookkeeping (they share the same underlying allocator) and the next
  // unrelated wasm call - wherever it happens to land - crashes with a "recursive use
  // of an object" / "unreachable" panic. That matched what we saw: a crash on
  // `.version()` after opening the New Canvas sheet, and, once that specific repeated
  // call was removed, the identical crash simply relocated to the next wasm call
  // (`newDocument` from Create). The `loadedRef` guard makes `EngineClient.load()` run
  // exactly once, which removes the orphaned instance (and the dangling finalizer)
  // entirely instead of papering over one crash site.
  const loadedRef = useRef(false);
  useEffect(() => {
    if (loadedRef.current) return;
    loadedRef.current = true;
    Promise.all([EngineClient.load(), getBridge()]).then(([engine, bridge]) => {
      useEditor.getState().setEngine(engine); useEditor.getState().setBridge(bridge);
      // The job worker: a second engine for work on one layer off the UI thread (engine jobs.rs).
      useEditor.getState().setJobs(new JobClient(engine.module, () => new Worker(new URL("./engine/job-worker.ts", import.meta.url), { type: "module" })));
      installTestApi({ engine, bridge, store: useEditor });
      bridge.onFileDrop((paths, position) => {
        const projects = paths.filter((p) => p.toLowerCase().endsWith(".comp"));
        const images = paths.filter((p) => !p.toLowerCase().endsWith(".comp"));
        for (const p of projects) void openProject(p);
        if (images.length) {
          const s = useEditor.getState();
          let at: { x: number; y: number } | undefined;
          if (position && s.activeId) {
            const el = document.querySelector('[data-testid="canvas-view"]') as HTMLElement | null;
            const r = el?.getBoundingClientRect();
            if (r) {
              // Tauri reports drop positions in physical pixels relative to the window; the
              // mock bridge (used in tests) already reports CSS pixels.
              const dpr = window.devicePixelRatio || 1;
              const view = bridge.positionIsPhysical ? { x: position.x / dpr, y: position.y / dpr } : position;
              const d = s.documents[s.activeId];
              at = s.viewports[s.activeId].documentPoint({ x: view.x - r.left, y: view.y - r.top }, { width: d.width, height: d.height });
            }
          }
          void importImages(images, at);
        }
      });
      setVersion(engine.version());
      setReady(true);
    }).catch((e) => setError(String(e)));
  }, []);
  useShortcuts();
  if (error) return <div data-testid="engine-error">Engine failed to load: {error}</div>;
  if (!ready) return <div data-testid="engine-loading">Loading engine...</div>;
  return (
    <div className="app">
      <MenuBar />
      <ProjectTabs />
      <CropOptions />
      <SelectionOptions />
      <GradientOptions />
      <ShapeOptions />
      <TransformInspector />
      <div className="workspace">
        <ToolRail />
        <CanvasView />
        <div className="right-column">
          <LayerProperties />
          <LayersList />
        </div>
      </div>
      <AdjustPanel />
      <ColorPickerPanel />
      <UndrawnNotice />
      <div className="status" data-testid="engine-ready">Compositor engine {version} ({BUILD_MARKER}){working && <span data-testid="working"> - Working...</span>}</div>
      {banner && <div data-testid="error-banner" className="error-banner">{banner}<button onClick={() => useEditor.getState().setError(null)}>Dismiss</button></div>}
      {sheet?.kind === "new" && <NewCanvasSheet />}
      {sheet?.kind === "canvasSize" && <CanvasSizeSheet />}
      {sheet?.kind === "imageSize" && <ImageSizeSheet />}
      {sheet?.kind === "jpeg" && <JpegExportSheet />}
      {sheet?.kind === "selectionAmount" && <SelectionAmountSheet operation={sheet.operation} />}
    </div>
  );
}
