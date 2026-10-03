import { useEditor } from "../state/store";
import { activeLayer } from "../state/selection";
import { containsPoint } from "../tools/transform-geometry";
import { DEFAULT_TEXT, validText, type TextRange, type TextStyle } from "../tools/text-style";

export interface TextEdit {
  token:number; document:string; layer:string|null; origin:[number,number]; stamp:string|null;
  style:TextStyle; range:TextRange; loading:boolean; preview:boolean;
  rendered:{width:number;height:number;pixels:ArrayBuffer}|null; bitmap:HTMLCanvasElement|null;
}
let token=0;
export function canEditText():boolean {
  const s=useEditor.getState(),doc=s.activeId?s.documents[s.activeId]:null;
  return !!doc&&!!activeLayer(doc)?.text&&!s.working&&!s.panelOwnsDocument()&&!s.maskTargeted();
}
export function beginText(origin:[number,number],boxSize?:[number,number],editActive=false):void {
  let s=useEditor.getState();if(!s.activeId||!s.engine||s.working||s.panelOwnsDocument())return;
  s.commitTransform();s.commitGradient();s=useEditor.getState();if(s.working)return;
  const id=s.activeId!,doc=s.documents[id];
  const layer=editActive?activeLayer(doc):!boxSize?[...doc.layers].reverse().find(l=>l.text&&l.visible&&containsPoint(l.transform,{x:origin[0],y:origin[1]})):null;
  if(editActive&&!layer?.text)return;
  if(layer?.text){s.selectLayers([layer.id],layer.id);}
  const fg=s.palette.foreground;
  const style:TextStyle=layer?.text?{...DEFAULT_TEXT,...structuredClone(layer.text)}:{...DEFAULT_TEXT,red:fg.red,green:fg.green,blue:fg.blue,...(boxSize?{boxSize}:{} )};
  const edit:TextEdit={token:++token,document:id,layer:layer?.id??null,origin:layer?.transform.origin??origin,stamp:layer?s.engine!.textStamp(id,layer.id):null,
    style,range:{start:0,end:style.content.length},loading:false,preview:true,rendered:null,bitmap:null};
  useEditor.setState({textEdit:edit});void renderDraft(edit);
}
export function editActiveText():void {beginText([0,0],undefined,true);}
export function updateText(style:TextStyle):void {
  const s=useEditor.getState(),edit=s.textEdit;if(!edit||s.working)return;
  if(!validText(style)){s.setError("Text settings are out of range");return;}
  const next={...edit,token:++token,style,rendered:null};useEditor.setState({textEdit:next});void renderDraft(next);
}
async function renderDraft(edit:TextEdit):Promise<void> {
  const s=useEditor.getState();if(!s.jobs||!s.engine)return;
  useEditor.setState({textEdit:{...edit,loading:true}});
  try{
    const result=await s.jobs.run(`text:${edit.document}`,{kind:"text",input:JSON.stringify(edit.style),pixels:null,mask:null});
    const now=useEditor.getState().textEdit;
    if(now?.token!==edit.token||!result?.pixels||!result.header)return;
    const {width,height}=JSON.parse(result.header) as {width:number;height:number};
    const rendered={width,height,pixels:result.pixels};let bitmap:HTMLCanvasElement|null=null;
    if(edit.layer){if(now.preview)s.engine.keepTextPreview(edit.document,edit.layer,edit.style,width,height,result.pixels,edit.stamp!);}
    else {
      // Canvas 2D takes straight RGBA; engine and worker buffers are premultiplied.
      const rgba=new Uint8ClampedArray(result.pixels.slice(0));
      for(let i=0;i<rgba.length;i+=4)if(rgba[i+3])for(let k=0;k<3;k++)rgba[i+k]=Math.min(255,Math.round(rgba[i+k]*255/rgba[i+3]));
      bitmap=document.createElement("canvas");bitmap.width=width;bitmap.height=height;bitmap.getContext("2d")!.putImageData(new ImageData(rgba,width,height),0,0);
    }
    useEditor.setState({textEdit:{...now,rendered,bitmap,loading:false}});s.refresh(edit.document);
  }catch(error){if(useEditor.getState().textEdit?.token===edit.token){useEditor.setState({textEdit:{...useEditor.getState().textEdit!,loading:false}});s.setError(error instanceof Error?error.message:String(error));}}
}
export function toggleTextPreview(preview:boolean):void {
  const s=useEditor.getState(),edit=s.textEdit;if(!edit||s.working)return;
  const next={...edit,preview};useEditor.setState({textEdit:next});
  if(edit.layer&&s.engine){if(preview&&edit.rendered){const r=edit.rendered;s.engine.keepTextPreview(edit.document,edit.layer,edit.style,r.width,r.height,r.pixels,edit.stamp!);}else s.engine.setPreview(edit.document,null);}
  s.refresh(edit.document);
}
export function cancelText():void {
  const s=useEditor.getState(),edit=s.textEdit;if(!edit||s.working)return;
  s.jobs?.cancel(`text:${edit.document}`);s.engine?.setPreview(edit.document,null);useEditor.setState({textEdit:null});s.refresh(edit.document);
}
export function commitText():void {
  const s=useEditor.getState(),edit=s.textEdit,r=edit?.rendered;if(!edit||!r||edit.loading||!s.engine||s.working)return;
  try{
    s.engine.installText(edit.document,edit.layer,edit.style,r.width,r.height,r.pixels,edit.origin,edit.stamp);
    s.jobs?.cancel(`text:${edit.document}`);useEditor.setState({textEdit:null});s.refresh(edit.document);s.revealActiveLayer();
  }catch(error){s.setError(error instanceof Error?error.message:String(error));}
}
