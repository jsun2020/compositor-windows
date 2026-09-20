import { useEffect, useState } from "react";
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
import "./styles.css";

export function App() {
  const [ready, setReady] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const sheet = useEditor((s) => s.sheet);
  const banner = useEditor((s) => s.error);
  useEffect(() => {
    Promise.all([EngineClient.load(), getBridge()]).then(([engine, bridge]) => {
      useEditor.getState().setEngine(engine); useEditor.getState().setBridge(bridge);
      installTestApi({ engine, bridge, store: useEditor });
      setReady(true);
    }).catch((e) => setError(String(e)));
  }, []);
  if (error) return <div data-testid="engine-error">Engine failed to load: {error}</div>;
  if (!ready) return <div data-testid="engine-loading">Loading engine...</div>;
  return (
    <div className="app">
      <MenuBar />
      <ProjectTabs />
      <div className="workspace">
        <ToolRail />
        <CanvasView />
        <LayersList />
      </div>
      <div className="status" data-testid="engine-ready">Compositor engine {useEditor.getState().engine!.version()} ({BUILD_MARKER})</div>
      {banner && <div data-testid="error-banner" className="error-banner">{banner}<button onClick={() => useEditor.getState().setError(null)}>Dismiss</button></div>}
      {sheet?.kind === "new" && <NewCanvasSheet />}
    </div>
  );
}
