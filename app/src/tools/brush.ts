import { useEditor, canPaintNow } from "../state/store";
import { activeLayer } from "../state/selection";
import type { PointTuple,BrushOperation,HealingMode } from "../engine/types";

export interface BrushOptions { diameter: number; hardness: number; opacity: number; smoothing: number; blurRadius:number;aligned:boolean;allLayers:boolean;healingMode:HealingMode; }
export const DEFAULT_BRUSH: BrushOptions = { diameter: 40, hardness: 1, opacity: 1, smoothing: 0,blurRadius:5,aligned:true,allLayers:false,healingMode:"Content-Aware" };
export interface BrushDraft { document: string; layer: string; mask: boolean; erasing: boolean; color: [number,number,number,number]; points: PointTuple[]; anchor: PointTuple; pointer:PointTuple; operation:BrushOperation; }
export function isBrushTool(tool:string):boolean{return ["brush","eraser","blur","clone","healing"].includes(tool);}
let last: { document: string; layer: string; mask: boolean; point: PointTuple } | null = null;
export function beginBrush(at: PointTuple, shift: boolean): boolean {
  const s=useEditor.getState();
  if (!canPaintNow() || !s.activeId || s.working || s.brushDraft) return false;
  s.commitTransform(); s.commitGradient();
  if (useEditor.getState().working) return false;
  const doc=useEditor.getState().documents[s.activeId], layer=activeLayer(doc);
  if (!layer) return false;
  const mask=s.maskTargeted(), c=s.paletteColor(false);
  let operation:BrushOperation={kind:"Paint"};
  if(s.tool==="blur")operation={kind:"Blur",radius:s.brushOptions.blurRadius};
  if(s.tool==="healing")operation={kind:"Heal",mode:s.brushOptions.healingMode,seed:1};
  if(s.tool==="clone"){
    if(!s.cloneSource||s.cloneSource.document!==s.activeId){s.setError("Alt-click to choose a source point first");return false;}
    const offset=s.brushOptions.aligned&&s.cloneOffset?s.cloneOffset:[Math.round(s.cloneSource.point[0]-at[0]),Math.round(s.cloneSource.point[1]-at[1])] as PointTuple;
    operation={kind:"Clone",offset,allLayers:s.brushOptions.allLayers};useEditor.setState({cloneOffset:offset});
  }
  const points=shift && last?.document===s.activeId && last.layer===layer.id && last.mask===mask ? [last.point,at] : [at];
  useEditor.setState({brushDraft:{document:s.activeId,layer:layer.id,mask,erasing:s.tool==="eraser",color:[c.red,c.green,c.blue,1],points,anchor:at,pointer:at,operation}});
  s.invalidateOverlay(); return true;
}
export function moveBrush(at: PointTuple): void {
  const s=useEditor.getState(), d=s.brushDraft;
  if (!d) return;
  useEditor.setState({brushDraft:{...d,pointer:at}});
  let point=at;
  if (s.tool==="brush" && s.brushOptions.smoothing>0) {
    const radius=s.brushOptions.smoothing/Math.max(0.01,s.viewports[d.document].pointsPerPixel);
    const dx=at[0]-d.anchor[0],dy=at[1]-d.anchor[1],distance=Math.hypot(dx,dy);
    if (distance<=radius) return;
    const step=(distance-radius)/distance;point=[d.anchor[0]+dx*step,d.anchor[1]+dy*step];
  }
  const end=d.points[d.points.length-1];if (Math.hypot(point[0]-end[0],point[1]-end[1])<0.1) return;
  useEditor.setState({brushDraft:{...d,points:[...d.points,point],anchor:point,pointer:at}});s.invalidateOverlay();
}
export function cancelBrush(): void { useEditor.setState({brushDraft:null});useEditor.getState().invalidateOverlay(); }
export function finishBrush(): void {
  const s=useEditor.getState(), d=s.brushDraft;
  if (!d) return;cancelBrush();
  // The v1.4.5 string smoothing catches up to the hand on mouse-up.
  if(s.tool==="brush"&&s.brushOptions.smoothing>0&&d.pointer.some((v,i)=>v!==d.points[d.points.length-1][i]))d.points=[...d.points,d.pointer];
  last={document:d.document,layer:d.layer,mask:d.mask,point:d.points[d.points.length-1]};
  const command={type:"BrushStroke" as const,id:d.layer,mask:d.mask,brush:{...s.brushOptions,points:d.points,color:d.color,erasing:d.erasing,operation:d.operation}};
  // Every stroke is one edit job; overlapping dabs never become separate undo entries.
  if (s.jobs) void s.runEditJob(command,d.layer); else s.run(command);
}
