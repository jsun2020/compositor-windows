import { useEffect, useState } from "react";
import { EngineClient } from "./engine/client";
import { BUILD_MARKER } from "./build-info";
import { installTestApi } from "./test-api";

export function App() {
  const [engine, setEngine] = useState<EngineClient | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    EngineClient.load().then((e) => { setEngine(e); installTestApi({ engine: e }); }).catch((e) => setError(String(e)));
  }, []);
  if (error) return <div data-testid="engine-error">Engine failed to load: {error}</div>;
  if (!engine) return <div data-testid="engine-loading">Loading engine...</div>;
  return <div data-testid="engine-ready">Compositor engine {engine.version()} ({BUILD_MARKER})</div>;
}
