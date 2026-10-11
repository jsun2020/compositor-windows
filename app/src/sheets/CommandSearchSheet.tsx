import { useState } from "react";
import { useEditor } from "../state/store";
import { commandMenus } from "../panels/MenuBar";
import { TOOLS } from "../panels/ToolRail";
import { Sheet } from "./Sheet";

export function CommandSearchSheet() {
  const s=useEditor(),[query,setQuery]=useState(""),[index,setIndex]=useState(0);
  // The search sheet itself should not disable its own commands; other owners still do.
  const state={...s,sheet:null,panelOwnsDocument:()=>s.panelOwnsDocument()||!!s.working};
  const commands=commandMenus(state).flatMap(m=>m.items.filter(it=>it!=="separator").map(it=>({...it,group:m.title}))).filter(it=>it.id!=="search-commands");
  commands.push(...TOOLS.map(t=>({id:`tool-${t.id}`,label:`${t.label} Tool`,group:"Tools",checked:s.tool===t.id,enabled:!state.panelOwnsDocument(),run:()=>useEditor.getState().setTool(t.id)})));
  const mode=(id:string,label:string,run:()=>void)=>commands.push({id,label,group:"Tool Modes",enabled:!state.panelOwnsDocument(),run});
  for(const kind of ["Rectangle","Ellipse"] as const)mode(`marquee-${kind}`,`${kind} Marquee`,()=>{s.setTool("marquee");s.setSelectionOptions({marquee:kind});});
  for(const kind of ["Freehand","Polygonal"] as const)mode(`lasso-${kind}`,`${kind} Lasso`,()=>{s.setTool("lasso");s.setSelectionOptions({lasso:kind});});
  for(const shape of ["Linear","Radial"] as const)mode(`gradient-${shape}`,`${shape} Gradient`,()=>{s.setTool("gradient");s.setGradientOptions({shape});});
  for(const smearMode of ["Blur","Liquify","Smudge"] as const)mode(`smear-${smearMode}`,`${smearMode} Mode`,()=>{s.setTool("blur");s.setBrushOptions({smearMode});});
  for(const kind of ["Rectangle","Ellipse","Line"] as const)mode(`shape-${kind}`,`${kind} Shape`,()=>{s.setTool("shape");s.setShapeOptions({kind});});
  for(const healingMode of ["Content-Aware","Create Texture","Proximity Match"] as const)mode(`healing-${healingMode}`,`${healingMode} Healing`,()=>{s.setTool("healing");s.setBrushOptions({healingMode});});
  for(const selectionMode of ["Replace","Add","Subtract"] as const)mode(`selection-${selectionMode}`,`${selectionMode} Selection`,()=>s.setSelectionOptions({mode:selectionMode}));
  commands.push({id:"tool-mode-next",label:"Next Tool Mode",group:"Tools",enabled:!state.panelOwnsDocument(),run:()=>useEditor.getState().cycleToolMode()});
  const matches=commands.filter(c=>`${c.group} ${c.label}`.toLowerCase().includes(query.toLowerCase()));
  const selected=matches[Math.min(index,matches.length-1)];
  const execute=(c=selected)=>{if(!c||c.enabled===false||s.busy||s.working)return;s.closeSheet();c.run();};
  return <Sheet title="Search Commands" primary="Run" canConfirm={!!selected&&selected.enabled!==false&&!s.busy&&!s.working} onCancel={s.closeSheet} onConfirm={()=>execute()}>
    <input autoFocus aria-label="Search Commands" className="command-query" value={query} onChange={e=>{setQuery(e.target.value);setIndex(0);}}
      onKeyDown={e=>{if(e.key==="ArrowDown"||e.key==="ArrowUp"){e.preventDefault();setIndex(i=>Math.max(0,Math.min(matches.length-1,i+(e.key==="ArrowDown"?1:-1))));}if(e.key==="Enter"){e.preventDefault();e.stopPropagation();execute();}}}/>
    <div className="command-results" role="listbox" aria-label="Commands">
      {matches.map((c,i)=><button key={c.id} role="option" aria-selected={i===index} disabled={c.enabled===false||s.busy||s.working} onMouseEnter={()=>setIndex(i)} onClick={()=>execute(c)}>{c.checked && <span aria-hidden="true">✓ </span>}{c.group} › {c.label}</button>)}
      {!matches.length&&<p>No matching commands</p>}
    </div>
  </Sheet>;
}
