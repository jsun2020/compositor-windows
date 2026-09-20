import { useEffect, useState } from "react";
import { EngineClient } from "./engine/client";
import { BUILD_MARKER } from "./build-info";
import { installTestApi } from "./test-api";
import { useEditor } from "./state/store";
import { CanvasView } from "./canvas/CanvasView";

export function App() {
  const [engine, setEngine] = useState<EngineClient | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    EngineClient.load().then((e) => {
      setEngine(e);
      useEditor.getState().setEngine(e);
      installTestApi({ engine: e, store: useEditor });
    }).catch((e) => setError(String(e)));
  }, []);
  if (error) return <div data-testid="engine-error">Engine failed to load: {error}</div>;
  if (!engine) return <div data-testid="engine-loading">Loading engine...</div>;
  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100vh" }}>
      <div data-testid="engine-ready" style={{ flex: "0 0 auto", padding: 4, fontSize: 12, color: "#999" }}>
        Compositor engine {engine.version()} ({BUILD_MARKER})
      </div>
      <CanvasView />
    </div>
  );
}
