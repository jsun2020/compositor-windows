import { useEditor } from "../state/store";
import { activeLayer } from "../state/selection";
import {releaseJobResult,type JobResult} from "../engine/jobs";

export interface ContentFillEdit { token:number; document: string; layer: string; input: string; result: JobResult | null; preview: boolean; }
let nextToken=1;
export function canContentFill(): boolean {
  const s=useEditor.getState(),doc=s.activeId?s.documents[s.activeId]:null,l=doc?activeLayer(doc):null;
  return !!doc?.selection&&!doc.selection.empty&&!!l?.hasPixels&&!l.isGroup&&!l.adjustment&&!s.maskTargeted()&&!s.working&&!s.panelOwnsDocument()&&!s.brushDraft;
}
export async function beginContentFill(): Promise<void> {
  if(!canContentFill())return;
  let s=useEditor.getState();s.commitTransform();s.commitGradient();s=useEditor.getState();
  const {engine,jobs,activeId}=s;if(!engine||!jobs||!activeId||s.working)return;
  const layer=activeLayer(s.documents[activeId])!.id;
  let edit:ContentFillEdit|null=null;
  try {
    edit={token:nextToken++,document:activeId,layer,input:"",result:null,preview:true};
    useEditor.setState({contentFill:edit,working:true});
    const copy=await engine.jobInputAsync(activeId,layer);
    if(useEditor.getState().contentFill?.token!==edit.token)return;
    edit={...edit,input:copy.input};useEditor.setState({contentFill:edit});
    const outPerDoc=s.viewports[activeId].pointsPerPixel*(window.devicePixelRatio||1);
    const result=await jobs.run(`contentFill:${activeId}`,{kind:"edit",...copy,command:JSON.stringify({type:"ContentAwareFill",id:layer}),outPerDoc});
    if(useEditor.getState().contentFill!==edit||!result?.pixels||!result.header)return;
    edit={...edit,result};useEditor.setState({contentFill:edit});
    const expected=edit.token;
    await engine.installJobAsync(activeId,layer,edit.input,result.header,result.pixels,result.mask,result.display??null,true,()=>useEditor.getState().contentFill?.token===expected);
    useEditor.getState().refresh(activeId);
  } catch(error) {
    if(!edit||useEditor.getState().contentFill?.token===edit.token){cancelContentFill();useEditor.setState({error:error instanceof Error?error.message:String(error)});}
  } finally {
    const now=useEditor.getState().contentFill;
    if(now?.token===edit?.token)useEditor.setState({working:false});
  }
}
export async function toggleContentFillPreview(on:boolean):Promise<void>{
  const s=useEditor.getState(),edit=s.contentFill,result=edit?.result;if(!edit||!result?.pixels||!result.header||!s.engine)return;
  useEditor.setState({contentFill:{...edit,preview:on}});
  if(on){useEditor.setState({working:true});try{await s.engine.installJobAsync(edit.document,edit.layer,edit.input,result.header,result.pixels,result.mask,result.display??null,true,()=>useEditor.getState().contentFill?.token===edit.token);}catch(error){if(useEditor.getState().contentFill?.token===edit.token)s.setError(String(error));}finally{if(useEditor.getState().contentFill?.token===edit.token)useEditor.setState({working:false});}}
  else s.engine.setPreview(edit.document,null);
  useEditor.getState().refresh(edit.document);
}
export function cancelContentFill():void{
  const s=useEditor.getState(),edit=s.contentFill;if(!edit)return;
  s.jobs?.cancel(`contentFill:${edit.document}`);s.engine?.setPreview(edit.document,null);
  releaseJobResult(edit.result);
  useEditor.setState({contentFill:null,working:false});s.refresh(edit.document);
}
export async function commitContentFill():Promise<void>{
  const s=useEditor.getState(),edit=s.contentFill,result=edit?.result;
  if(!edit||!result?.pixels||!result.header||!s.engine||s.working)return;
  useEditor.setState({working:true});
  try{await s.engine.installJobAsync(edit.document,edit.layer,edit.input,result.header,result.pixels,result.mask,result.display??null,false,()=>useEditor.getState().contentFill?.token===edit.token);releaseJobResult(result);if(useEditor.getState().contentFill?.token===edit.token)useEditor.setState({contentFill:null,working:false});}
  catch(error){if(useEditor.getState().contentFill?.token===edit.token){s.engine.setPreview(edit.document,null);useEditor.setState({contentFill:null,working:false});s.setError(error instanceof Error?error.message:String(error));}}
  s.refresh(edit.document);
}
