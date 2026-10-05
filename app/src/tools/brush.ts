import { useEditor, canPaintNow } from "../state/store";
import { activeLayer } from "../state/selection";
import type { PointTuple,BrushOperation,HealingMode,WarpSpec } from "../engine/types";
import {beginWarp,moveWarp,cancelWarp,finishWarp} from "./warp";

export interface BrushOptions { diameter: number; hardness: number; opacity: number; smoothing: number; blurRadius:number;aligned:boolean;allLayers:boolean;healingMode:HealingMode;smearMode:"Blur"|"Liquify"|"Smudge";strength:number; }
export const DEFAULT_BRUSH: BrushOptions = { diameter: 40, hardness: 1, opacity: 1, smoothing: 0,blurRadius:5,aligned:true,allLayers:false,healingMode:"Content-Aware",smearMode:"Blur",strength:1 };
export interface BrushDraft { document: string; layer: string; mask: boolean; erasing: boolean; color: [number,number,number,number]; points: PointTuple[]; anchor: PointTuple; pointer:PointTuple; operation:BrushOperation; warp?:WarpSpec; }
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
  if(s.tool==="blur"&&s.brushOptions.smearMode!=="Blur"&&mask){s.setError("Smudge and Liquify work on a layer's pixels, not its mask.");return false;}
  let operation:BrushOperation={kind:"Paint"};
  if(s.tool==="blur")operation={kind:"Blur",radius:s.brushOptions.blurRadius};
  if(s.tool==="healing")operation={kind:"Heal",mode:s.brushOptions.healingMode,seed:1};
  if(s.tool==="clone"){
    if(!s.cloneSource||s.cloneSource.document!==s.activeId){s.setError("Alt-click to choose a source point first");return false;}
    const offset=s.brushOptions.aligned&&s.cloneOffset?s.cloneOffset:[Math.round(s.cloneSource.point[0]-at[0]),Math.round(s.cloneSource.point[1]-at[1])] as PointTuple;
    operation={kind:"Clone",offset,allLayers:s.brushOptions.allLayers};useEditor.setState({cloneOffset:offset});
  }
  const warping=s.tool==="blur"&&s.brushOptions.smearMode!=="Blur";
  const points=!warping && shift && last?.document===s.activeId && last.layer===layer.id && last.mask===mask ? [last.point,at] : [at];
  const draft:BrushDraft={document:s.activeId,layer:layer.id,mask,erasing:s.tool==="eraser",color:[c.red,c.green,c.blue,1],points,anchor:at,pointer:at,operation};
  if(s.tool==="blur"&&s.brushOptions.smearMode!=="Blur")draft.warp={mode:s.brushOptions.smearMode,diameter:Math.max(2,s.brushOptions.diameter),hardness:Math.min(0.98,s.brushOptions.hardness),strength:s.brushOptions.strength,points};
  useEditor.setState({brushDraft:draft});if(draft.warp)beginWarp(draft);
  s.invalidateOverlay(); return true;
}
export function moveBrush(at: PointTuple): void {
  const s=useEditor.getState(), d=s.brushDraft;
  if (!d||s.working) return;
  useEditor.setState({brushDraft:{...d,pointer:at}});
  let point=at;
  if (s.tool==="brush" && s.brushOptions.smoothing>0) {
    const radius=s.brushOptions.smoothing/Math.max(0.01,s.viewports[d.document].pointsPerPixel);
    const dx=at[0]-d.anchor[0],dy=at[1]-d.anchor[1],distance=Math.hypot(dx,dy);
    if (distance<=radius) return;
    const step=(distance-radius)/distance;point=[d.anchor[0]+dx*step,d.anchor[1]+dy*step];
  }
  const end=d.points[d.points.length-1];if (Math.hypot(point[0]-end[0],point[1]-end[1])<0.1) return;
  const draft={...d,points:[...d.points,point],anchor:point,pointer:at};
  useEditor.setState({brushDraft:draft});if(draft.warp)moveWarp(draft);s.invalidateOverlay();
}
export function cancelBrush(): void { cancelWarp();useEditor.setState({brushDraft:null});useEditor.getState().invalidateOverlay(); }
export function finishBrush(): void {
  const s=useEditor.getState(), d=s.brushDraft;
  if (!d) return;if(d.warp){void finishWarp();return;}cancelBrush();
  // The v1.4.5 string smoothing catches up to the hand on mouse-up.
  if(s.tool==="brush"&&s.brushOptions.smoothing>0&&d.pointer.some((v,i)=>v!==d.points[d.points.length-1][i]))d.points=[...d.points,d.pointer];
  last={document:d.document,layer:d.layer,mask:d.mask,point:d.points[d.points.length-1]};
  const command={type:"BrushStroke" as const,id:d.layer,mask:d.mask,brush:{...s.brushOptions,points:d.points,color:d.color,erasing:d.erasing,operation:d.operation}};
  // Every stroke is one edit job; overlapping dabs never become separate undo entries.
  if (s.jobs) void s.runEditJob(command,d.layer); else s.run(command);
}
