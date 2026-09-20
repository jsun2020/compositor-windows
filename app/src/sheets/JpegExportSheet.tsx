import { useEffect, useRef, useState } from "react";
import { Sheet } from "./Sheet";
import { useEditor } from "../state/store";

/** Display box the preview image is fit into; also the downscale target for the fast preview. */
const PREVIEW_MAX_SIDE = 1000;

function sameMatte(a: [number, number, number], b: [number, number, number]): boolean {
  return a[0] === b[0] && a[1] === b[1] && a[2] === b[2];
}

export function JpegExportSheet() {
  const s = useEditor();
  const doc = s.documents[s.activeId!];
  const [quality, setQuality] = useState(85);
  const [color, setColor] = useState("#ffffff");
  // The downscaled preview image, recomputed 150 ms after the last change: cheap enough to
  // run on every tick even on a large canvas, so the picture never freezes the UI.
  const [preview, setPreview] = useState<{ url: string; bytes: Uint8Array } | null>(null);
  // The full-resolution encode, recomputed 500 ms after the last change: this is what the
  // size readout reports and what Export reuses, so a large canvas is only encoded once per
  // settled quality/matte instead of on every slider tick.
  const [fullSize, setFullSize] = useState<{ bytes: Uint8Array; quality: number; matte: [number, number, number] } | null>(null);
  const [computingSize, setComputingSize] = useState(false);
  // Mirrors `preview` so the unmount cleanup can revoke the latest object URL without
  // calling setState after the component has unmounted.
  const previewRef = useRef<{ url: string; bytes: Uint8Array } | null>(null);
  const matte = (): [number, number, number] => [1, 3, 5].map((i) => parseInt(color.slice(i, i + 2), 16) / 255) as [number, number, number];
  useEffect(() => {
    const t = setTimeout(() => {
      try {
        const bytes = s.engine!.exportJpegPreview(doc.id, quality / 100, matte(), PREVIEW_MAX_SIDE);
        setPreview((old) => {
          if (old) URL.revokeObjectURL(old.url);
          const next = { url: URL.createObjectURL(new Blob([bytes as unknown as BlobPart], { type: "image/jpeg" })), bytes };
          previewRef.current = next;
          return next;
        });
      } catch (e) { s.setError(e instanceof Error ? e.message : String(e)); }
    }, 150);
    return () => clearTimeout(t);
  }, [quality, color, doc.id]);
  useEffect(() => {
    setComputingSize(true);
    const t = setTimeout(() => {
      try {
        const q = quality / 100; const m = matte();
        const bytes = s.engine!.exportJpeg(doc.id, q, m);
        setFullSize({ bytes, quality: q, matte: m });
      } catch (e) { s.setError(e instanceof Error ? e.message : String(e)); }
      finally { setComputingSize(false); }
    }, 500);
    return () => clearTimeout(t);
  }, [quality, color, doc.id]);
  useEffect(() => () => { if (previewRef.current) URL.revokeObjectURL(previewRef.current.url); }, []);
  const exportNow = async () => {
    if (!preview) return;
    const path = await s.bridge!.pickExportFile(doc.path ? s.bridge!.baseName(doc.path) : "Untitled", "jpg");
    if (!path) return;
    try {
      const q = quality / 100; const m = matte();
      // Reuse the last full-resolution encode when quality and matte have not changed since
      // it ran, otherwise this click is the first time this combination has been encoded at
      // full resolution.
      const bytes = fullSize && fullSize.quality === q && sameMatte(fullSize.matte, m) ? fullSize.bytes : s.engine!.exportJpeg(doc.id, q, m);
      await s.bridge!.writeFile(path, bytes);
      s.closeSheet();
    } catch (e) { s.setError(e instanceof Error ? e.message : String(e)); }
  };
  const sizeText = computingSize ? "Computing size..." : fullSize ? `${(fullSize.bytes.length / 1024).toFixed(1)} KB` : "";
  return (
    <Sheet title="Export JPEG" primary="Export" canConfirm={preview !== null} onCancel={s.closeSheet} onConfirm={() => void exportNow()}>
      <label>Quality <input type="range" min={0} max={100} value={quality} onChange={(e) => setQuality(Number(e.target.value))} /> {quality}</label>
      <label>Matte <input type="color" value={color} onChange={(e) => setColor(e.target.value)} /></label>
      {preview && <img alt="Preview" src={preview.url} style={{ maxWidth: 480, maxHeight: 320, objectFit: "contain" }} />}
      <div data-testid="jpeg-size">{sizeText}</div>
    </Sheet>
  );
}
