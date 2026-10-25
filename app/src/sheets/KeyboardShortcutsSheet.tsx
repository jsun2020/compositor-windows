import {useState} from "react";
import {useEditor} from "../state/store";
import {Sheet} from "./Sheet";
import {assignShortcut,effectiveShortcuts,readShortcutOverrides,saveShortcutOverrides,shortcutKey,type ActionId,type Shortcut} from "../shortcuts/keymap";
const title=(id:string)=>id.split("-").map(w=>w.charAt(0).toUpperCase()+w.slice(1)).join(" ");
const label=(s:Shortcut)=>[s.ctrl?"Ctrl":null,s.alt?"Alt":null,s.shift?"Shift":null,s.key.length===1?s.key.toUpperCase():s.key].filter(Boolean).join("+");
export function KeyboardShortcutsSheet(){
  const close=useEditor(s=>s.closeSheet),[overrides,setOverrides]=useState(readShortcutOverrides),[query,setQuery]=useState(""),[capture,setCapture]=useState<ActionId|null>(null),[error,setError]=useState<string|null>(null);
  const save=()=>{try{saveShortcutOverrides(overrides);close();}catch{setError("The shortcut settings could not be saved. Your previous shortcuts are unchanged.");}};
  const bindings=effectiveShortcuts(overrides);
  return <Sheet title="Keyboard Shortcuts" primary="Save" canConfirm={!capture} fieldOwnsEnter onConfirm={save} onCancel={close}>
    <label>Search commands<input aria-label="Search shortcuts" value={query} onChange={e=>setQuery(e.target.value)}/></label>
    <p>Select a shortcut, then press the new key combination. Delete and Backspace can be assigned; use Clear to remove a shortcut.</p>
    <div className="shortcut-list">{(Object.entries(bindings) as [ActionId,Shortcut[]][]).filter(([id])=>title(id).toLowerCase().includes(query.toLowerCase())).map(([id,list])=><div className="shortcut-row" key={id}>
      <span>{title(id)}</span><button aria-label={`Shortcut for ${title(id)}`} onClick={()=>setCapture(id)} onKeyDown={e=>{
        if(capture!==id)return;e.preventDefault();e.stopPropagation();
        if(e.key==="Escape"){setCapture(null);return;}
        if(e.nativeEvent.isComposing||["Control","Shift","Alt","Meta"].includes(e.key))return;
        setOverrides(assignShortcut(overrides,id,{key:shortcutKey(e.nativeEvent),ctrl:e.ctrlKey||e.metaKey,alt:e.altKey,shift:e.shiftKey}));setCapture(null);
      }}>{capture===id?"Press keys…":list.map(label).join(" / ")||"Unassigned"}</button>
      <button aria-label={`Clear ${title(id)} shortcut`} onClick={()=>{setOverrides(assignShortcut(overrides,id,null));setCapture(null);}}>Clear</button>
    </div>)}</div>
    <button onClick={()=>{setOverrides({});setCapture(null);}}>Restore Defaults</button>
    {error&&<p role="alert">{error}</p>}
  </Sheet>;
}
