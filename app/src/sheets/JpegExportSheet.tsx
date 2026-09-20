import { useEffect, useRef, useState } from "react";
import { Sheet } from "./Sheet";
import { useEditor } from "../state/store";

export function JpegExportSheet() {
  const s = useEditor();
  const doc = s.documents[s.activeId!];
  const [quality, setQuality] = useState(85);
  const [color, setColor] = useState("#ffffff");
  const [preview, setPreview] = useState<{ url: string; bytes: Uint8Array } | null>(null);
  // Mirrors `preview` so the unmount cleanup can revoke the latest object URL without
  // calling setState after the component has unmounted.
  const previewRef = useRef<{ url: string; bytes: Uint8Array } | null>(null);
  const matte = (): [number, number, number] => [1, 3, 5].map((i) => parseInt(color.slice(i, i + 2), 16) / 255) as [number, number, number];
  useEffect(() => {
    const t = setTimeout(() => {
      try {
        const bytes = s.engine!.exportJpeg(doc.id, quality / 100, matte());
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
  useEffect(() => () => { if (previewRef.current) URL.revokeObjectURL(previewRef.current.url); }, []);
  const exportNow = async () => {
    if (!preview) return;
    const path = await s.bridge!.pickExportFile(doc.path ? s.bridge!.baseName(doc.path) : "Untitled", "jpg");
    if (!path) return;
    try { await s.bridge!.writeFile(path, preview.bytes); s.closeSheet(); } catch (e) { s.setError(e instanceof Error ? e.message : String(e)); }
  };
  return (
    <Sheet title="Export JPEG" primary="Export" canConfirm={preview !== null} onCancel={s.closeSheet} onConfirm={() => void exportNow()}>
      <label>Quality <input type="range" min={0} max={100} value={quality} onChange={(e) => setQuality(Number(e.target.value))} /> {quality}</label>
      <label>Matte <input type="color" value={color} onChange={(e) => setColor(e.target.value)} /></label>
      {preview && <img alt="Preview" src={preview.url} style={{ maxWidth: 480, maxHeight: 320, objectFit: "contain" }} />}
      <div data-testid="jpeg-size">{preview ? `${(preview.bytes.length / 1024).toFixed(1)} KB` : ""}</div>
    </Sheet>
  );
}
