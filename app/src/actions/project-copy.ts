import {useEditor} from "../state/store";

export const LAYER_DRAG_TYPE="application/x-compositor-layers";
// Keep the source in this editor instance. Unrelated text/file drops never become layer commands.
let current:{token:string;source:string;ids:string[]}|null=null;
export function beginProjectLayerDrag(data:DataTransfer,id:string):void {
  const s=useEditor.getState();if(!s.activeId||s.panelOwnsDocument()||s.busy||s.working)return;
  s.commitTransform();s.commitGradient();
  const ids=s.selectedLayerIds.includes(id)?[id,...s.selectedLayerIds.filter(v=>v!==id)]:[id];
  current={token:crypto.randomUUID(),source:s.activeId,ids};data.setData(LAYER_DRAG_TYPE,current.token);
}
export function endProjectLayerDrag():void{current=null;}
export function canDropProjectLayers(data:DataTransfer,destination:string|null):boolean{
  const s=useEditor.getState();return !!current&&current.source!==destination&&data.types.includes(LAYER_DRAG_TYPE)&&!s.panelOwnsDocument()&&!s.busy&&!s.working;
}
export function dropProjectLayers(data:DataTransfer,destination:string|null,at:{x:number;y:number}|null=null):void {
  if(!canDropProjectLayers(data,destination)||data.getData(LAYER_DRAG_TYPE)!==current?.token)return;
  const drag=current!;current=null;const s=useEditor.getState();if(!s.engine)return;
  try {
    const result=s.engine.copyLayersToProject(drag.source,drag.ids,destination,at);
    if(destination){s.refresh(destination);s.setActive(destination);}else{s.openDocument(result.document);}
    useEditor.getState().selectLayers(result.selected,result.selected[0]??null);useEditor.getState().revealActiveLayer();
  }catch(error){useEditor.getState().setError(error instanceof Error?error.message:String(error));}
}
