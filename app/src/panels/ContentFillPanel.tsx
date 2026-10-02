import { useEffect } from "react";
import { useEditor } from "../state/store";
import { cancelContentFill, commitContentFill, toggleContentFillPreview } from "../actions/content-fill";
export function ContentFillPanel(){
  const s=useEditor(),edit=s.contentFill;
  useEffect(()=>{
    if(!edit)return;
    const key=(e:KeyboardEvent)=>{if(e.key==="Escape"){e.preventDefault();cancelContentFill();}else if(e.key==="Enter"&&!useEditor.getState().working){e.preventDefault();commitContentFill();}};
    window.addEventListener("keydown",key);return()=>window.removeEventListener("keydown",key);
  },[!!edit]);
  if(!edit)return null;
  return <section className="adjust-panel" role="dialog" aria-label="Content-Aware Fill" data-testid="content-fill-panel">
    <h2>Content-Aware Fill</h2><p>{s.working?"Synthesizing pixels...":"Review the fill, then apply or cancel."}</p>
    <label><input type="checkbox" aria-label="Preview fill" checked={edit.preview} disabled={s.working} onChange={e=>toggleContentFillPreview(e.target.checked)}/> Preview</label>
    <div className="panel-footer"><button onClick={cancelContentFill}>Cancel</button><button disabled={s.working||!edit.result} onClick={commitContentFill}>Apply</button></div>
  </section>;
}
