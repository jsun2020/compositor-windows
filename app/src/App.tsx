import { useEffect, useRef, useState } from "react";
import { EngineClient } from "./engine/client";
import { BUILD_MARKER } from "./build-info";
import { installTestApi } from "./test-api";
import { useEditor } from "./state/store";
import { getBridge } from "./shell/bridge";
import { CanvasView } from "./canvas/CanvasView";
import { MenuBar } from "./panels/MenuBar";
import { ProjectTabs } from "./panels/ProjectTabs";
import { LayersList } from "./panels/LayersList";
import { ToolRail } from "./panels/ToolRail";
import { NewCanvasSheet } from "./sheets/NewCanvasSheet";
import { CanvasSizeSheet } from "./sheets/CanvasSizeSheet";
import { ImageSizeSheet } from "./sheets/ImageSizeSheet";
import { JpegExportSheet } from "./sheets/JpegExportSheet";
import { CropOptions } from "./panels/CropOptions";
import "./styles.css";

export function App() {
  const [ready, setReady] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // Cached once instead of re-read from JSX on every render (see the loadedRef note
  // below for why a second live wasm call at the wrong moment is unsafe here).
  const [version, setVersion] = useState("");
  const sheet = useEditor((s) => s.sheet);
  const banner = useEditor((s) => s.error);
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
      installTestApi({ engine, bridge, store: useEditor });
      setVersion(engine.version());
      setReady(true);
    }).catch((e) => setError(String(e)));
  }, []);
  if (error) return <div data-testid="engine-error">Engine failed to load: {error}</div>;
  if (!ready) return <div data-testid="engine-loading">Loading engine...</div>;
  return (
    <div className="app">
      <MenuBar />
      <ProjectTabs />
      <CropOptions />
      <div className="workspace">
        <ToolRail />
        <CanvasView />
        <LayersList />
      </div>
      <div className="status" data-testid="engine-ready">Compositor engine {version} ({BUILD_MARKER})</div>
      {banner && <div data-testid="error-banner" className="error-banner">{banner}<button onClick={() => useEditor.getState().setError(null)}>Dismiss</button></div>}
      {sheet?.kind === "new" && <NewCanvasSheet />}
      {sheet?.kind === "canvasSize" && <CanvasSizeSheet />}
      {sheet?.kind === "imageSize" && <ImageSizeSheet />}
      {sheet?.kind === "jpeg" && <JpegExportSheet />}
    </div>
  );
}
