import { useEditor } from "../state/store";
import { activeLayer } from "../state/selection";
import { duplicateSelected } from "./layers";
import type { EngineClient } from "../engine/client";
let wholeLayerClipboard:{token:string;snapshot:ReturnType<EngineClient["clipboardInput"]>}|null=null;
async function cloneSnapshot(s:ReturnType<EngineClient["clipboardInput"]>){
  const clone=async(b:ArrayBuffer|null)=>{if(!b)return null;const out=new Uint8Array(b.byteLength);let start=performance.now();for(let i=0;i<b.byteLength;i+=4*1024*1024){const length=Math.min(4*1024*1024,b.byteLength-i);out.set(new Uint8Array(b,i,length),i);if(performance.now()-start>=8){await new Promise<void>(r=>requestAnimationFrame(()=>r()));start=performance.now();}}return out.buffer;};
  const layers=[];for(const l of s.layers)layers.push({pixels:await clone(l.pixels),mask:await clone(l.mask)});return{...s,layers,points:s.points?.slice(0)??null};
}

export function canCopyPixels(merged = false): boolean {
  const s = useEditor.getState(), doc = s.activeId ? s.documents[s.activeId] : null;
  if (!doc || s.panelOwnsDocument() || s.working || s.busy || s.cropRect || s.selectionDraft || doc.selection?.empty) return false;
  if (merged) return true;
  const l = activeLayer(doc);
  return !!l && (s.maskTargeted() ? l.hasMask : doc.selection ? !l.isGroup && !l.adjustment&&l.hasPixels : l.hasPixels||l.isGroup||!!l.adjustment);
}

export async function copyPixels(merged = false, cut = false): Promise<void> {
  if (!canCopyPixels(merged)) return;
  let s = useEditor.getState(); s.commitTransform(); s.commitGradient();
  s = useEditor.getState();
  const { engine, bridge, jobs, activeId } = s;
  if (!engine || !bridge || !jobs || !activeId || s.working) return;
  const doc = s.documents[activeId], target = activeLayer(doc)?.id ?? null, mask = s.maskTargeted();
  useEditor.setState({ working: true });
  try {
    const input = await engine.clipboardInputAsync(activeId, target, mask, merged);
    const whole=!merged&&!mask&&!doc.selection;
    const cached=whole?{token:crypto.randomUUID(),snapshot:input}:null;
    const result = await jobs.run("clipboard", { kind: "clipboard", ...(whole?await cloneSnapshot(input):input), pixels: null, mask: null, png: true });
    if (!result?.pixels || !result.header) return;
    const rect = JSON.parse(result.header) as { x: number; y: number };
    await bridge.writeClipboardImage(new Uint8Array(result.pixels), [rect.x, rect.y],cached?.token);
    wholeLayerClipboard=cached;
    if (cut && target) {
      // The document remains locked across both operations. A failed clipboard
      // write reaches the catch before any pixels are cleared.
      engine.execute(activeId, whole?{type:"DeleteLayers",ids:[target],bake:true}:{ type: "CutPixels", id: target, mask });
      useEditor.getState().refresh(activeId);
    }
  } catch (e) { useEditor.setState({ error: e instanceof Error ? e.message : String(e) }); }
  finally { useEditor.setState({ working: false }); }
}

export async function pastePixels(): Promise<void> {
  let s = useEditor.getState();
  if (!s.activeId || s.panelOwnsDocument() || s.working || s.busy || s.cropRect) return;
  s.commitTransform(); s.commitGradient(); s = useEditor.getState();
  const { engine, bridge, jobs, activeId } = s;
  if (!engine || !bridge || !jobs || !activeId || s.working) return;
  useEditor.setState({ working: true });
  try {
    const image = await bridge.readClipboardImage();
    if(image.layerToken&&wholeLayerClipboard?.token===image.layerToken){engine.pasteCopiedLayers(activeId,wholeLayerClipboard.snapshot);useEditor.getState().refresh(activeId);useEditor.getState().revealActiveLayer();return;}
    const result = await jobs.run("clipboard", { kind: "decodeClipboard", input: "", pixels: image.bytes.slice().buffer, mask: null });
    if (!result?.pixels || !result.header) return;
    const { width, height } = JSON.parse(result.header) as { width: number; height: number };
    engine.pastePixels(activeId, width, height, result.pixels, image.origin);
    useEditor.getState().refresh(activeId); useEditor.getState().revealActiveLayer();
  } catch (e) { useEditor.setState({ error: e instanceof Error ? e.message : String(e) }); }
  finally { useEditor.setState({ working: false }); }
}

export async function layerViaCopy(): Promise<void> {
  let s = useEditor.getState();
  let doc = s.activeId ? s.documents[s.activeId] : null;
  if (!doc?.selection) { duplicateSelected(); return; }
  if (!canCopyPixels()) return;
  s.commitTransform(); s.commitGradient(); s = useEditor.getState();
  doc = s.activeId ? s.documents[s.activeId] : null;
  if (!doc) return;
  const { engine, jobs, activeId } = s, target = activeLayer(doc)?.id;
  if (!engine || !jobs || !activeId || !target) return;
  useEditor.setState({ working: true });
  try {
    const input = await engine.clipboardInputAsync(activeId, target, s.maskTargeted(), false);
    const result = await jobs.run("clipboard", { kind: "clipboard", ...input, pixels: null, mask: null, png: false });
    if (!result?.pixels || !result.header) return;
    const r = JSON.parse(result.header) as { x: number; y: number; width: number; height: number };
    engine.pastePixels(activeId, r.width, r.height, result.pixels, [r.x, r.y], true);
    useEditor.getState().refresh(activeId); useEditor.getState().revealActiveLayer();
  } catch (e) { useEditor.setState({ error: e instanceof Error ? e.message : String(e) }); }
  finally { useEditor.setState({ working: false }); }
}
