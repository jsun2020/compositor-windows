import {filterLayerParams,withFilterLayerParams} from "../state/filter-layers";
import { useEditor } from "../state/store";
import type { OverlayState } from "./overlay";

/** Spatial filter handles stay in document coordinates when zooming or reducing a preview. */
export function filterHandleLine(): OverlayState["gradientLine"] {
  const s=useEditor.getState(),p=s.adjustEdit?.params??filterLayerParams(s.adjustEdit?.adjustment);
  if(!s.activeId||!p)return null;
  const d=s.documents[s.activeId],vp=s.viewports[s.activeId];
  const view=(p:[number,number])=>vp.viewPoint({x:p[0],y:p[1]},d);
  if(p.filter==="ProgressiveBlur"&&p.settings.start&&p.settings.end) return {
    start:view(p.settings.start),end:view(p.settings.end),radial:p.settings.radial,
    from:p.settings.inverted?[1,1,1,1]:[0,0,0,1],to:p.settings.inverted?[0,0,0,1]:[1,1,1,1],
  };
  if(p.filter==="RgbSplit"&&p.settings.radial&&p.settings.center) {
    const c=p.settings.center;return {start:view(c),end:view([c[0]+p.settings.reach,c[1]]),radial:true,from:[1,1,1,1],to:[1,1,1,1]};
  }
  return null;
}

function updateParams(params:import("../engine/types").FilterParams):void {
 const s=useEditor.getState(),e=s.adjustEdit;if(!e)return;
 if(e.params)s.updateAdjust({params});else if(e.adjustment)s.updateAdjust({adjustment:withFilterLayerParams(e.adjustment,params)});
}
export function installFilterHandles(el:HTMLElement,spaceHeld:()=>boolean):()=>void {
  let drag:{pointer:number;document:string;layer:string;handle:"start"|"end"}|null=null;
  const point=(e:PointerEvent)=>{const s=useEditor.getState(),id=s.activeId!,r=el.getBoundingClientRect();return s.viewports[id].documentPoint({x:e.clientX-r.left,y:e.clientY-r.top},s.documents[id]);};
  const stop=(e:PointerEvent)=>{e.preventDefault();e.stopImmediatePropagation();};
  const apply=(e:PointerEvent)=>{
    const s=useEditor.getState(),p=s.adjustEdit?.params??filterLayerParams(s.adjustEdit?.adjustment);
    if(!drag||s.activeId!==drag.document||s.adjustEdit?.layerId!==drag.layer||!p)return;
    const at=point(e);
    if(p.filter==="ProgressiveBlur") updateParams({...p,settings:{...p.settings,[drag.handle]:[at.x,at.y]}});
    else if(p.filter==="RgbSplit"&&p.settings.radial) {
      const c=p.settings.center??[at.x,at.y];
      updateParams({...p,settings:{...p.settings,...(drag.handle==="start"?{center:[at.x,at.y] as [number,number]}:{reach:Math.max(1,Math.hypot(at.x-c[0],at.y-c[1]))})}});
    }
    s.invalidate();
  };
  const down=(e:PointerEvent)=>{
    if(e.button!==0||spaceHeld()||(e.target as Element).closest?.('[data-testid="navigator"]'))return;
    const s=useEditor.getState(),p=s.adjustEdit?.params??filterLayerParams(s.adjustEdit?.adjustment);
    if(!s.activeId||s.working||!p||s.colorPicker||s.adjustEdit?.sampleMode)return;
    if(p.filter!=="ProgressiveBlur"&&!(p.filter==="RgbSplit"&&p.settings.radial))return;
    const line=filterHandleLine(),r=el.getBoundingClientRect(),v={x:e.clientX-r.left,y:e.clientY-r.top};
    const near=(q:{x:number;y:number})=>Math.hypot(v.x-q.x,v.y-q.y)<=10;
    const handle=line&&near(line.end)?"end":line&&near(line.start)?"start":null;
    drag={pointer:e.pointerId,document:s.activeId,layer:s.adjustEdit!.layerId,handle:handle??(p.filter==="ProgressiveBlur"?"end":"start")};
    if(!handle&&p.filter==="ProgressiveBlur") {const a=point(e);updateParams({...p,settings:{...p.settings,start:[a.x,a.y],end:[a.x,a.y+1]}});}
    else apply(e);
    el.setPointerCapture(e.pointerId);stop(e);s.invalidate();
  };
  const move=(e:PointerEvent)=>{if(!drag||e.pointerId!==drag.pointer)return;apply(e);stop(e);};
  const up=(e:PointerEvent)=>{if(!drag||e.pointerId!==drag.pointer)return;apply(e);drag=null;stop(e);};
  const cancel=()=>{drag=null;};
  el.addEventListener("pointerdown",down,true);el.addEventListener("pointermove",move,true);el.addEventListener("pointerup",up,true);el.addEventListener("pointercancel",cancel);el.addEventListener("lostpointercapture",cancel);
  return()=>{el.removeEventListener("pointerdown",down,true);el.removeEventListener("pointermove",move,true);el.removeEventListener("pointerup",up,true);el.removeEventListener("pointercancel",cancel);el.removeEventListener("lostpointercapture",cancel);};
}
