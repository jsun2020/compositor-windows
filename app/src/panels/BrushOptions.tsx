import { useEditor } from "../state/store";
import { isBrushTool } from "../tools/brush";
import type { HealingMode } from "../engine/types";
export function BrushOptions() {
  const s=useEditor(); if(!isBrushTool(s.tool)) return null;
  const warp=s.tool==="blur"&&s.brushOptions.smearMode!=="Blur";
  return <div className="tool-options" data-testid="brush-options">
    <span>{s.tool==="eraser"?"Eraser":s.tool==="blur"?s.brushOptions.smearMode:s.tool==="clone"?"Clone Stamp":s.tool==="healing"?"Spot Healing":"Brush"}</span>
    {s.tool==="blur"&&<label>Mode <select aria-label="Smear mode" value={s.brushOptions.smearMode} onChange={e=>s.setBrushOptions({smearMode:e.target.value as "Blur"|"Liquify"|"Smudge",...(e.target.value!=="Blur"?{diameter:Math.max(2,s.brushOptions.diameter),hardness:Math.min(0.98,s.brushOptions.hardness)}:{})})}>{["Liquify","Blur","Smudge"].map(v=><option key={v}>{v}</option>)}</select></label>}
    <label>Size <input aria-label="Brush size" type="number" min={warp?2:1} max="2000" value={s.brushOptions.diameter} onChange={e=>s.setBrushOptions({diameter:Number(e.target.value)})}/></label>
    <label>Hardness <input aria-label="Brush hardness" type="number" min="0" max={warp?98:100} value={Math.round(s.brushOptions.hardness*100)} onChange={e=>s.setBrushOptions({hardness:Number(e.target.value)/100})}/>%</label>
    {warp?<label>Strength <input aria-label="Smear strength" type="number" min="1" max="100" value={Math.round(s.brushOptions.strength*100)} onChange={e=>s.setBrushOptions({strength:Number(e.target.value)/100})}/>%</label>:<label>Opacity <input aria-label="Brush opacity" type="number" min="1" max="100" value={Math.round(s.brushOptions.opacity*100)} onChange={e=>s.setBrushOptions({opacity:Number(e.target.value)/100})}/>%</label>}
    {s.tool==="brush" && <label>Smoothing <input aria-label="Brush smoothing" type="number" min="0" max="100" value={s.brushOptions.smoothing} onChange={e=>s.setBrushOptions({smoothing:Number(e.target.value)})}/>%</label>}
    {s.tool==="blur"&&!warp&&<label>Radius <input aria-label="Blur radius" type="number" min="0.5" max="50" step="0.5" value={s.brushOptions.blurRadius} onChange={e=>s.setBrushOptions({blurRadius:Number(e.target.value)})}/></label>}
    {s.tool==="clone"&&<><label><input type="checkbox" checked={s.brushOptions.aligned} onChange={e=>s.setBrushOptions({aligned:e.target.checked})}/>Aligned</label><label><input type="checkbox" checked={s.brushOptions.allLayers} onChange={e=>s.setBrushOptions({allLayers:e.target.checked})}/>Sample all layers</label><span>Alt-click to choose source</span></>}
    {s.tool==="healing"&&<label>Mode <select aria-label="Healing mode" value={s.brushOptions.healingMode} onChange={e=>s.setBrushOptions({healingMode:e.target.value as HealingMode})}>{(["Content-Aware","Create Texture","Proximity Match"] as const).map(v=><option key={v}>{v}</option>)}</select></label>}
  </div>;
}
