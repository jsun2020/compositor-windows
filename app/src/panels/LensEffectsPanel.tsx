import type { LensEffectSettings } from "../engine/types";
import { NumberInput } from "./NumberInput";

type NumericKey = {[K in keyof LensEffectSettings]: LensEffectSettings[K] extends number ? K : never}[keyof LensEffectSettings];
type Field = readonly [string, NumericKey, number, number, number?];
const fields:Record<LensEffectSettings["effect"], readonly Field[]> = {
  "Anamorphic Flare":[["Amount","flareAmount",0,100],["Length","flareLength",1,100],["Threshold","threshold",50,99],["Blue","flareBlue",0,100]],
  Starburst:[["Amount","starAmount",0,100],["Length","starLength",2,40],["Threshold","threshold",50,99],["Angle","starAngle",-180,180]],
  Mist:[["Strength","mistStrength",0,100],["Size","mistSize",0,100]],
  Prism:[["Amount","prismAmount",0,100],["Copies","prismCopies",1,8],["Offset","prismOffset",0,50],["Angle","prismAngle",-180,180],["Reach","prismSpread",10,100]],
  Bokeh:[["Size","bokehRadius",0.5,6,0.1],["Brightness","bokehBrightness",0,100],["Ring","bokehRing",0,100]],
};

export function LensEffectsPanel({settings:s,onChange}:{settings:LensEffectSettings;onChange(s:LensEffectSettings):void}) {
  const update=(patch:Partial<LensEffectSettings>)=>onChange({...s,...patch});
  return <>
    <label>Effect <select aria-label="Effect" value={s.effect} onChange={e=>update({effect:e.target.value as LensEffectSettings["effect"]})}>
      {Object.keys(fields).map(effect=><option key={effect}>{effect}</option>)}
    </select></label>
    {fields[s.effect].map(([label,key,min,max,step=1])=><span className="number-field" key={key}>
      <span className="number-field-label">{label}</span>
      <input type="range" aria-hidden tabIndex={-1} min={min} max={max} step={step} value={s[key]} onChange={e=>update({[key]:Number(e.target.value)})}/>
      <NumberInput label={label} value={s[key]} min={min} max={max} step={step} onChange={value=>update({[key]:value})}/>
    </span>)}
    {s.effect==="Starburst"&&<label>Points <select aria-label="Points" value={s.starPoints} onChange={e=>update({starPoints:Number(e.target.value)})}>
      {[4,6,8,12,14].map(n=><option key={n}>{n}</option>)}
    </select></label>}
    {s.effect==="Bokeh"&&<label>Shape <select aria-label="Shape" value={s.bokehShape} onChange={e=>update({bokehShape:e.target.value as LensEffectSettings["bokehShape"]})}>
      {["Round","Hexagon","Oval","Cat's Eye"].map(shape=><option key={shape}>{shape}</option>)}
    </select></label>}
  </>;
}
