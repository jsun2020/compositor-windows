import {useEffect,useRef} from "react";
import {useEditor} from "../state/store";
import {cancelText,commitText,toggleTextPreview,updateText} from "../actions/text";
import {colorText,fontText,replaceText,textColorAt,textFontAt} from "../tools/text-style";
import {hexOf,parseHex} from "../tools/color";
export function TextPanel(){
  const edit=useEditor(s=>s.textEdit),ref=useRef<HTMLTextAreaElement>(null);
  useEffect(()=>{if(edit){ref.current?.focus();ref.current?.select();}},[edit?.document,edit?.layer]);
  useEffect(()=>{if(!edit)return;const key=(e:KeyboardEvent)=>{if(e.isComposing)return;if(e.key==="Escape"){e.preventDefault();cancelText();}else if(e.key==="Enter"&&(e.ctrlKey||e.metaKey)){e.preventDefault();commitText();}};window.addEventListener("keydown",key);return()=>window.removeEventListener("keydown",key);},[!!edit]);
  if(!edit)return null;const{style,range}=edit,index=Math.min(range.start,Math.max(0,style.content.length-1)),rgb=textColorAt(style,index);
  const set=(key:string,value:unknown)=>updateText({...style,[key]:value});
  const selected=()=>{const el=ref.current;if(el){const e=useEditor.getState().textEdit;if(e)useEditor.setState({textEdit:{...e,range:{start:el.selectionStart,end:el.selectionEnd}}});}};
  const number=(label:string,key:"fontSize"|"tracking"|"leading",min:number,max:number)=> <label>{label}<input aria-label={label} type="number" min={min} max={max} value={style[key]} onChange={e=>{const n=e.currentTarget.valueAsNumber;if(Number.isFinite(n)&&n>=min&&n<=max)set(key,n);}}/></label>;
  return <section className="adjust-panel authoring-panel" role="dialog" aria-label="Text" data-testid="text-panel">
    <div className="adjust-header">{edit.layer?"Edit Text":"New Text"}</div>
    <div className="adjust-body">
      <textarea ref={ref} aria-label="Text content" value={style.content} maxLength={100000} rows={5} onSelect={selected} onChange={e=>updateText(replaceText(style,e.target.value))}/>
      <div className="authoring-grid">
        <label>Font<input aria-label="Font" list="text-fonts" value={textFontAt(style,index)} onChange={e=>{if(e.target.value)updateText(fontText(style,range,e.target.value));}}/></label>
        <datalist id="text-fonts">{["Arial","Arial Black","Calibri","Cambria","Consolas","Georgia","Segoe UI","Times New Roman","Verdana","Microsoft YaHei","SimSun"].map(n=><option key={n} value={n}/>)}</datalist>
        {number("Font size","fontSize",1,2000)}
        <label>Color<input aria-label="Text color" type="color" value={`#${hexOf({red:rgb[0],green:rgb[1],blue:rgb[2]})}`} onChange={e=>{const c=parseHex(e.target.value)!;updateText(colorText(style,range,[c.red,c.green,c.blue]));}}/></label>
        <label>Alignment<select aria-label="Text alignment" value={style.alignment} onChange={e=>set("alignment",e.target.value)}>{["Left","Center","Right"].map(v=><option key={v}>{v}</option>)}</select></label>
        {number("Tracking","tracking",-100,1000)}{number("Leading (0 = auto)","leading",0,5000)}
        <label><input aria-label="Fixed text box" type="checkbox" checked={!!style.boxSize} onChange={e=>set("boxSize",e.target.checked?[Math.max(100,edit.rendered?.width??400),Math.max(100,edit.rendered?.height??200)]:null)}/>Fixed text box</label>
        {style.boxSize&&["Width","Height"].map((label,i)=><label key={label}>{label}<input aria-label={`Text box ${label.toLowerCase()}`} type="number" min={16} max={30000} value={style.boxSize![i]} onChange={e=>{const n=e.currentTarget.valueAsNumber;if(n>=16&&n<=30000){const b=[...style.boxSize!] as [number,number];b[i]=n;set("boxSize",b);}}}/></label>)}
      </div>
      <p>{range.end>range.start?`${range.end-range.start} UTF-16 units selected; font and color apply to the selection.`:"Font and color apply to all text."} {edit.loading?"Rendering...":"Ctrl+Enter applies; Escape cancels."}</p>
    </div>
    <div className="adjust-footer"><label><input aria-label="Preview text" type="checkbox" checked={edit.preview} onChange={e=>toggleTextPreview(e.target.checked)}/>Preview</label><button onClick={cancelText}>Cancel</button><button className="primary" onClick={commitText} disabled={edit.loading||!edit.rendered}>Apply</button></div>
  </section>;
}
