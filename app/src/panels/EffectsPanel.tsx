import {useEffect} from "react";
import {useEditor} from "../state/store";
import {cancelEffects,commitEffects,DEFAULT_SHADOW,DEFAULT_STROKE,toggleEffectsPreview,updateEffects} from "../actions/effects";
import {hexOf,parseHex} from "../tools/color";
import type {StrokeEffect,ShadowEffect} from "../engine/types";
export function EffectsPanel(){
  const edit=useEditor(s=>s.effectsEdit);
  useEffect(()=>{if(!edit)return;const key=(e:KeyboardEvent)=>{if(e.key==="Escape"){e.preventDefault();cancelEffects();}else if(e.key==="Enter"&&!(e.target instanceof HTMLElement&&["INPUT","SELECT","BUTTON"].includes(e.target.tagName))){e.preventDefault();commitEffects();}};window.addEventListener("keydown",key);return()=>window.removeEventListener("keydown",key);},[!!edit]);
  if(!edit)return null;
  const fields=(name:"stroke"|"shadow",value:StrokeEffect|ShadowEffect)=>{
    const label=name==="stroke"?"Stroke":"Drop Shadow";
    const patch=(v:Partial<StrokeEffect&ShadowEffect>)=>updateEffects({...edit.effects,[name]:{...value,...v}});
    const number=(field:string,title:string,min:number,max:number,step=1)=> <label key={field}>{title}<input aria-label={`${label} ${title}`} type="number" min={min} max={max} step={step} value={value[field] as number} onChange={e=>{const n=e.currentTarget.valueAsNumber;if(Number.isFinite(n)&&n>=min&&n<=max)patch({[field]:n});}}/></label>;
    return <fieldset key={name}><legend><label><input aria-label={`Enable ${label}`} type="checkbox" checked={!!edit.effects[name]&&value.enabled!==false} onChange={e=>patch({enabled:e.target.checked})}/>{label}</label></legend><div className="authoring-grid">
      {name==="stroke"?<>{number("size","Size",0,500)}<label><input aria-label="Stroke inside" type="checkbox" checked={(value as StrokeEffect).inside} onChange={e=>patch({inside:e.target.checked})}/>Inside</label></>:<>{number("angle","Angle",-360,360)}{number("distance","Distance",0,5000)}{number("blur","Blur",0,500)}</>}
      <label>Color<input aria-label={`${label} color`} type="color" value={`#${hexOf(value)}`} onChange={e=>patch({...parseHex(e.target.value)!})}/></label>{number("opacity","Opacity",0,1,.01)}
    </div><button onClick={()=>{const effects={...edit.effects};delete effects[name];updateEffects(effects);}}>Remove {label}</button></fieldset>;
  };
  return <section className="adjust-panel authoring-panel" role="dialog" aria-label="Layer Effects" data-testid="effects-panel"><div className="adjust-header">Layer Effects</div><div className="adjust-body">{fields("stroke",edit.effects.stroke??{...DEFAULT_STROKE,enabled:false})}{fields("shadow",edit.effects.shadow??{...DEFAULT_SHADOW,enabled:false})}</div><div className="adjust-footer"><label><input aria-label="Preview effects" type="checkbox" checked={edit.preview} onChange={e=>toggleEffectsPreview(e.target.checked)}/>Preview</label><button onClick={cancelEffects}>Cancel</button><button className="primary" onClick={commitEffects}>Apply</button></div></section>;
}
