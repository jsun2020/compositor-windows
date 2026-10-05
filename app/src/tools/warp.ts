import {useEditor} from "../state/store";
import {WarpClient} from "../engine/warp-client";
import {releaseJobResult} from "../engine/jobs";
import type {BrushDraft} from "./brush";
import type {JobInputCopy} from "../engine/client";
interface Gesture {draft:BrushDraft;input:string;client:WarpClient|null;cancelled:boolean;ending:boolean;prepared:Promise<void>}
let gesture:Gesture|null=null;
function valid(g:Gesture):boolean{
  const s=useEditor.getState();return !g.cancelled&&gesture===g&&s.activeId===g.draft.document&&s.documents[g.draft.document]?.activeLayerId===g.draft.layer&&s.tool==="blur";
}
function fail(g:Gesture,error:unknown):void{
  if(!valid(g))return;
  useEditor.getState().setError(error instanceof Error?error.message:String(error));cancelWarp();
  useEditor.setState({brushDraft:null});
}
export function beginWarp(draft:BrushDraft):void{
  const g:Gesture={draft,input:"",client:null,cancelled:false,ending:false,prepared:Promise.resolve()};gesture=g;
  g.prepared=(async()=>{
    let snapshot:JobInputCopy|null=null;
    try{
      const engine=useEditor.getState().engine!;
      snapshot=await engine.jobInputAsync(draft.document,draft.layer);
      g.input=snapshot.input;
      if(!valid(g)){releaseJobResult({header:null,pixels:snapshot.pixels,mask:snapshot.mask,inputs:{pixels:null,mask:null,points:snapshot.points}});return;}
      // The cooperative copy may yield. Refuse a snapshot assembled across a
      // stored-pixel/selection mutation before any preview is shown.
      if(engine.jobHeader(draft.document,draft.layer)!==snapshot.input)throw Error("The layer changed while the stroke was preparing");
      const {points:_,...settings}=draft.warp!;
      g.client=new WarpClient(new Worker(new URL("../engine/warp-worker.ts",import.meta.url),{type:"module"}),engine.module,snapshot,settings,
        result=>{
          try{if(valid(g)&&result?.header&&result.pixels){engine.keepJobPreview(draft.document,draft.layer,snapshot!.input,result.header,result.pixels,result.mask,result.display??null);useEditor.getState().invalidate();}}
          finally{releaseJobResult(result);}
        },error=>fail(g,error));
      // Include every point received while snapshot preparation was yielding.
      for(const p of g.draft.points)g.client.append(p);
    }catch(error){if(snapshot)releaseJobResult({header:null,pixels:snapshot.pixels,mask:snapshot.mask,inputs:{pixels:null,mask:null,points:snapshot.points}});fail(g,error);}
  })();
}
export function moveWarp(draft:BrushDraft):void{
  if(!gesture||gesture.ending)return;
  gesture.draft=draft;gesture.client?.append(draft.points[draft.points.length-1]);
}
export function cancelWarp():void{
  const g=gesture;if(!g)return;g.cancelled=true;gesture=null;g.client?.dispose();
  const s=useEditor.getState();if(s.documents[g.draft.document]){s.engine?.setPreview(g.draft.document,null);s.invalidate();}
  if(g.ending)useEditor.setState({working:false});
}
export async function finishWarp():Promise<void>{
  const g=gesture;if(!g||g.ending)return;g.ending=true;useEditor.setState({working:true});
  let result:Awaited<ReturnType<WarpClient["finish"]>>=null;
  try{
    await g.prepared;if(!valid(g)||!g.client)return;
    const s=useEditor.getState(),engine=s.engine!,input=g.input;
    const scale=s.viewports[g.draft.document].pointsPerPixel*(window.devicePixelRatio||1);
    result=await g.client.finish(scale);
    if(valid(g)&&result?.header){await engine.installJobAsync(g.draft.document,g.draft.layer,input,result.header,result.pixels,result.mask,result.display??null,false,()=>valid(g));}
  }catch(error){fail(g,error);}
  finally{
    releaseJobResult(result);
    if(gesture===g){const s=useEditor.getState();gesture=null;g.client?.dispose();s.engine?.setPreview(g.draft.document,null);useEditor.setState({brushDraft:null,working:false});s.refresh(g.draft.document);s.invalidateOverlay();}
  }
}
