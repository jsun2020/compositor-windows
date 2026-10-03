import { useEditor } from "../state/store";
import { activeLayer } from "../state/selection";
import type { LayerEffects, StrokeEffect, ShadowEffect } from "../engine/types";
export interface EffectsEdit {document:string;layer:string;effects:LayerEffects;preview:boolean;}
export const DEFAULT_STROKE:StrokeEffect={size:4,red:0,green:0,blue:0,opacity:1,inside:false};
export const DEFAULT_SHADOW:ShadowEffect={angle:135,distance:10,blur:10,red:0,green:0,blue:0,opacity:.5};
export function canEditEffects():boolean {
  const s=useEditor.getState(),d=s.activeId?s.documents[s.activeId]:null,l=d?activeLayer(d):null;
  return !!l?.hasPixels&&!l.isGroup&&!l.adjustment&&!s.maskTargeted()&&!s.working&&!s.panelOwnsDocument();
}
export function beginEffects():void {
  if(!canEditEffects())return;let s=useEditor.getState();s.commitTransform();s.commitGradient();s=useEditor.getState();if(s.working)return;
  const document=s.activeId!,layer=activeLayer(s.documents[document])!;
  useEditor.setState({effectsEdit:{document,layer:layer.id,effects:structuredClone(layer.effects??{}),preview:true}});s.invalidate();
}
export function updateEffects(effects:LayerEffects):void {const s=useEditor.getState();if(!s.effectsEdit||s.working)return;useEditor.setState({effectsEdit:{...s.effectsEdit,effects}});s.invalidate();}
export function toggleEffectsPreview(preview:boolean):void {const s=useEditor.getState();if(!s.effectsEdit)return;useEditor.setState({effectsEdit:{...s.effectsEdit,preview}});s.invalidate();}
export function cancelEffects():void {const s=useEditor.getState();if(!s.effectsEdit)return;useEditor.setState({effectsEdit:null});s.invalidate();}
export function commitEffects():void {
  const s=useEditor.getState(),e=s.effectsEdit;if(!e||!s.engine||s.working)return;
  try{s.engine.execute(e.document,{type:"SetLayerEffects",id:e.layer,effects:Object.keys(e.effects).length?e.effects:null});useEditor.setState({effectsEdit:null});s.refresh(e.document);}
  catch(error){s.setError(error instanceof Error?error.message:String(error));}
}
