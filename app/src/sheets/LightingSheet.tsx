import { useEffect, useRef, useState } from "react";
import { lightingSource, visionWorker } from "../actions/depth";
import type { Reply } from "../vision/depth-worker";
import {
  defaultVolume,
  firstLamp,
  type LightSettings,
  type RimLamp,
  type VolumeSettings,
} from "../vision/lighting";
import { useEditor } from "../state/store";
import { Sheet } from "./Sheet";

type Rendered = Extract<Reply, { kind: "render" }>;
function Slider({
  label,
  value,
  change,
  min = 0,
  max = 100,
}: {
  label: string;
  value: number;
  change: (n: number) => void;
  min?: number;
  max?: number;
}) {
  return (
    <label className="light-slider">
      <span>{label}</span>
      <input
        type="range"
        aria-label={label}
        min={min}
        max={max}
        value={value}
        onChange={(e) => change(Number(e.target.value))}
      />
      <output>{value}</output>
    </label>
  );
}
export function LightingSheet({ kind }: { kind: "rim" | "volume" }) {
  const [settings, setSettings] = useState<LightSettings>(() =>
    kind === "rim" ? { kind, lights: [firstLamp], glow: 10 } : defaultVolume,
  );
  const [ready, setReady] = useState(false),
    [rendered, setRendered] = useState<Rendered | null>(null);
  const [error, setError] = useState(""),
    [extracting, setExtracting] = useState(false);
  const canvas = useRef<HTMLCanvasElement>(null),
    worker = useRef<Worker | null>(null);
  const source = useRef<ReturnType<typeof lightingSource> | null>(null);
  const revision = useRef(0),
    inFlight = useRef(false),
    latest = useRef(settings);
  latest.current = settings;
  const completed = useRef<LightSettings | null>(null),
    submitted = useRef<LightSettings | null>(null);
  const render = () => {
    if (
      inFlight.current ||
      !worker.current ||
      submitted.current === latest.current
    )
      return;
    inFlight.current = true;
    submitted.current = latest.current;
    worker.current.postMessage({
      kind: "render",
      settings: latest.current,
      id: ++revision.current,
    });
  };
  useEffect(() => {
    let current: Worker;
    try {
      const state = useEditor.getState();
      state.commitTransform();
      state.commitGradient();
      const captured = lightingSource();
      source.current = captured;
      current = visionWorker();
      worker.current = current;
      current.onerror = (e) =>
        setError(e.message || "Lighting could not start.");
      current.onmessage = (e: MessageEvent<Reply>) => {
        if (worker.current !== current) return;
        const r = e.data;
        if (r.kind === "error") {
          setError(r.error);
          setExtracting(false);
          return;
        }
        if (r.kind === "ready") {
          setReady(true);
          render();
          return;
        }
        if (r.kind === "map") {
          try {
            if (useEditor.getState().activeId !== captured.doc)
              throw new Error("The active document changed.");
            captured.engine.insertDepthMap(
              captured.doc,
              captured.layer.id,
              captured.layer.pixelsRevision,
              r.width,
              r.height,
              r.values,
            );
            state.refresh(captured.doc);
            state.revealActiveLayer();
            state.closeSheet();
          } catch (e) {
            setError(String(e));
            setExtracting(false);
          }
          return;
        }
        if (r.kind === "render") {
          inFlight.current = false;
          if (submitted.current === latest.current) {
            completed.current = submitted.current;
            setRendered(r);
            const el = canvas.current;
            if (el) {
              el.width = r.width;
              el.height = r.height;
              const ctx = el.getContext("2d");
              if (ctx) {
                const image = ctx.createImageData(r.width, r.height);
                image.data.set(r.preview);
                ctx.putImageData(image, 0, 0);
              }
            }
          }
          render();
        }
      };
      current.postMessage(
        { kind: "prepare", depth: kind === "volume", input: captured.input },
        [captured.input.pixels.buffer],
      );
    } catch (e) {
      setError(String(e));
    }
    return () => {
      worker.current = null;
      current?.terminate();
      inFlight.current = false;
      submitted.current = null;
    };
  }, [kind]);
  useEffect(() => {
    if (ready) render();
  }, [settings, ready]);
  const close = useEditor((s) => s.closeSheet);
  const apply = () => {
    if (
      !rendered ||
      !source.current ||
      completed.current !== settings ||
      extracting
    )
      return;
    const { doc, layer, engine } = source.current;
    try {
      if (doc !== useEditor.getState().activeId)
        throw new Error("The active document changed.");
      engine.applyLightField(
        doc,
        layer.id,
        layer.pixelsRevision,
        rendered.width,
        rendered.height,
        rendered.field,
      );
      useEditor.getState().refresh(doc);
      close();
    } catch (e) {
      setError(String(e));
    }
  };
  const editVolume = (partial: Partial<VolumeSettings>) =>
    setSettings((previous) =>
      previous.kind === "volume" ? { ...previous, ...partial } : previous,
    );
  const editLamp = (i: number, partial: Partial<RimLamp>) =>
    setSettings((previous) =>
      previous.kind === "rim"
        ? {
            ...previous,
            lights: previous.lights.map((light, j) =>
              j === i ? { ...light, ...partial } : light,
            ),
          }
        : previous,
    );
  const changed =
    settings.kind === "volume"
      ? settings.haze > 0 || settings.intensity > 0
      : settings.lights.some((light) => light.strength > 0);
  return (
    <Sheet
      title={kind === "rim" ? "Rim Light" : "Volumetric Light"}
      primary="Apply"
      fieldOwnsEnter
      canConfirm={
        !!rendered &&
        !error &&
        !extracting &&
        completed.current === settings &&
        changed
      }
      onCancel={close}
      onConfirm={apply}
    >
      <div className="lighting-layout">
        <div className="light-preview">
          <canvas
            ref={canvas}
            aria-label="Lighting preview"
            title={
              kind === "volume" ? "Click to position the light" : undefined
            }
            onClick={(e) => {
              if (kind !== "volume" || !ready) return;
              const r = e.currentTarget.getBoundingClientRect();
              editVolume({
                center: [
                  (e.clientX - r.left) / r.width,
                  (e.clientY - r.top) / r.height,
                ],
              });
            }}
          />
          {(!ready || extracting) && !error && (
            <span role="status">
              {extracting ? "Extracting depth…" : "Preparing lighting…"}
            </span>
          )}
        </div>
        <div className="light-controls">
          {settings.kind === "rim" ? (
            <>
              {settings.lights.map((light, i) => (
                <fieldset key={i}>
                  <legend>Light {i + 1}</legend>
                  <label>
                    Color{" "}
                    <input
                      type="color"
                      aria-label={`Light ${i + 1} color`}
                      value={light.color}
                      onChange={(e) => editLamp(i, { color: e.target.value })}
                    />
                  </label>
                  {(["angle", "strength", "width", "choke"] as const).map(
                    (key) => (
                      <Slider
                        key={key}
                        label={key[0].toUpperCase() + key.slice(1)}
                        min={key === "angle" ? -180 : 0}
                        max={key === "angle" ? 180 : 100}
                        value={light[key]}
                        change={(n) => editLamp(i, { [key]: n })}
                      />
                    ),
                  )}
                  {settings.lights.length > 1 && (
                    <button
                      onClick={() =>
                        setSettings({
                          ...settings,
                          lights: settings.lights.filter((_, j) => j !== i),
                        })
                      }
                    >
                      Remove light
                    </button>
                  )}
                </fieldset>
              ))}
              {settings.lights.length < 3 && (
                <button
                  onClick={() => {
                    const prev = settings.lights.at(-1)!;
                    setSettings({
                      ...settings,
                      lights: [
                        ...settings.lights,
                        {
                          ...prev,
                          strength: 50,
                          angle:
                            prev.angle > 0
                              ? prev.angle - 180
                              : prev.angle + 180,
                          color:
                            settings.lights.length === 1
                              ? "#b3d6ff"
                              : "#ffffff",
                        },
                      ],
                    });
                  }}
                >
                  Add light
                </button>
              )}
              <Slider
                label="Glow"
                value={settings.glow}
                change={(glow) => setSettings({ ...settings, glow })}
              />
            </>
          ) : (
            <>
              <label>
                Light color{" "}
                <input
                  type="color"
                  value={settings.color}
                  onChange={(e) => editVolume({ color: e.target.value })}
                />
              </label>
              {(
                [
                  ["intensity", "Intensity"],
                  ["length", "Length"],
                  ["haze", "Haze"],
                  ["size", "Size"],
                  ["shafts", "Shafts"],
                  ["distance", "Distance"],
                  ["depthContrast", "Depth Contrast"],
                ] as const
              ).map(([key, label]) => (
                <Slider
                  key={key}
                  label={label}
                  value={settings[key]}
                  change={(n) => editVolume({ [key]: n })}
                />
              ))}
              <label>
                <input
                  type="checkbox"
                  checked={settings.keepsSubjectClear}
                  onChange={(e) =>
                    editVolume({ keepsSubjectClear: e.target.checked })
                  }
                />{" "}
                Keep subject clear
              </label>
              <button
                disabled={!ready || extracting || !!error}
                onClick={() => {
                  setExtracting(true);
                  worker.current?.postMessage({
                    kind: "map",
                    contrast: settings.depthContrast,
                    keep: settings.keepsSubjectClear,
                  });
                }}
              >
                Extract Depth
              </button>
              <small>Click the preview to place the light.</small>
            </>
          )}
        </div>
      </div>
      {error && <p role="alert">{error}</p>}
    </Sheet>
  );
}
