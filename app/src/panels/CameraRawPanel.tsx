import { useEffect,useRef,useId, useState,type ReactNode } from "react";
import { curveSamples, insertPoint, movePoint, nearestPoint, removePoint } from "../tools/curves-editor";
import { useEditor } from "../state/store";
import { NumberInput } from "./NumberInput";
import { CAMERA_RAW_FIELDS, DEFAULT_CAMERA_RAW,effectiveCameraRaw,resetCameraRawGroup, type CameraRawGroupName,type CameraRawSettings, type CameraRawPointColor } from "../engine/camera-raw";

const caption=(field:string)=>field.replace(/([A-Z])/g," $1").replace(/^./,c=>c.toUpperCase());
const families=["Reds","Oranges","Yellows","Greens","Aquas","Blues","Purples","Magentas"];
const linear=()=>[{x:0,y:0},{x:1,y:1}];
function GradingWheel({name,wheel,disabled,onChange}:{name:string;wheel:CameraRawSettings["grading"]["global"];disabled:boolean;onChange(value:CameraRawSettings["grading"]["global"]):void}){
  const gradient=useId();
  const pick=(e:React.PointerEvent<SVGSVGElement>)=>{if(disabled)return;const box=e.currentTarget.getBoundingClientRect();const x=(e.clientX-box.left)/box.width*128-64,y=(e.clientY-box.top)/box.height*128-64;onChange({...wheel,hue:(Math.atan2(y,x)*180/Math.PI+360)%360,saturation:Math.min(100,Math.hypot(x,y)/62*100)});};
  const angle=wheel.hue*Math.PI/180,radius=wheel.saturation/100*62;
  return <svg role="group" aria-label={`${name} Color Wheel`} data-testid={`grading-wheel-${name.toLowerCase()}`} viewBox="0 0 128 128" width="128" height="128" style={{touchAction:"none",opacity:disabled?0.4:1,pointerEvents:disabled?"none":"auto"}}
    onPointerDown={e=>{if(e.button!==0||disabled)return;e.currentTarget.setPointerCapture(e.pointerId);pick(e);}} onPointerMove={e=>{if(e.currentTarget.hasPointerCapture(e.pointerId))pick(e);}}>
    <defs><radialGradient id={gradient}><stop stopColor="white"/><stop offset="1" stopColor="white" stopOpacity="0"/></radialGradient></defs>
    {Array.from({length:72},(_,i)=>{const a=i*5*Math.PI/180,b=(i+1)*5*Math.PI/180;return <path key={i} d={`M64,64 L${64+Math.cos(a)*62},${64+Math.sin(a)*62} A62,62 0 0,1 ${64+Math.cos(b)*62},${64+Math.sin(b)*62} Z`} fill={`hsl(${i*5} 100% 50%)`}/>;})}
    <circle cx="64" cy="64" r="62" fill={`url(#${gradient})`}/><circle cx={64+Math.cos(angle)*radius} cy={64+Math.sin(angle)*radius} r="4" fill="none" stroke="black" strokeWidth="3"/><circle cx={64+Math.cos(angle)*radius} cy={64+Math.sin(angle)*radius} r="4" fill="none" stroke="white"/>
  </svg>;
}
function CameraRawScope(){
  const s=useEditor(),edit=s.adjustEdit!;const canvas=useRef<HTMLCanvasElement>(null);const [mode,setMode]=useState<"Histogram"|"Vectorscope">("Histogram");
  const [scope,setScope]=useState<ReturnType<NonNullable<typeof s.engine>["cameraRawScope"]>|null>(null);
  const settings=edit.params?.filter==="CameraRaw"?JSON.stringify(effectiveCameraRaw(edit.params.settings,edit.cameraRawBypass)):null;
  useEffect(()=>{setScope(null);if(!s.engine||!s.activeId||!settings)return;const timer=setTimeout(()=>{try{setScope(s.engine!.cameraRawScope(s.activeId!,edit.layerId,JSON.parse(settings)));}catch{setScope(null);}},150);return()=>clearTimeout(timer);},[s.engine,s.activeId,edit.layerId,settings]);
  useEffect(()=>{const c=canvas.current,ctx=c?.getContext("2d");if(!c||!ctx)return;ctx.clearRect(0,0,c.width,c.height);if(!scope)return;
    if(mode==="Histogram"){const peak=Math.max(...scope.red,...scope.green,...scope.blue);if(!peak)return;for(const [bins,color] of [[scope.red,"#f55"],[scope.green,"#5f5"],[scope.blue,"#58f"]] as const){ctx.strokeStyle=color;ctx.beginPath();bins.forEach((n,i)=>{const x=i*c.width/255,y=c.height-n/peak*c.height;i?ctx.lineTo(x,y):ctx.moveTo(x,y);});ctx.stroke();}}
    else{const peak=Math.max(...scope.vectorscope);if(!peak)return;scope.vectorscope.forEach((n,i)=>{if(n<=0)return;ctx.fillStyle=`rgba(255,255,255,${.15+.85*n/peak})`;ctx.fillRect(i%64*c.width/64,(63-Math.floor(i/64))*c.height/64,c.width/64+1,c.height/64+1);});}
  },[scope,mode]);
  return <div><label>Scope<select aria-label="Scope" value={mode} onChange={e=>setMode(e.target.value as typeof mode)}><option>Histogram</option><option>Vectorscope</option></select></label><canvas ref={canvas} width={256} height={110} aria-label={mode==="Histogram"?"RGB histogram":"Vectorscope"} data-testid="camera-raw-scope" style={{width:"100%",background:"#222"}}/><small>{scope?"Graded pixels; bounded sampling, diagnostics excluded.":"Reading graded pixels…"}</small></div>;
}
function Numeric({name,value,min,max,step=1,onChange}:{name:string;value:number;min:number;max:number;step?:number;onChange(v:number):void}){
  return <span className="number-field"><span className="number-field-label">{name}</span>
    <input type="range" aria-hidden tabIndex={-1} min={min} max={max} step={step} value={value} onChange={e=>onChange(Number(e.target.value))}/>
    <NumberInput label={name} value={value} min={min} max={max} step={step} onChange={onChange}/></span>;
}
function CameraRawGroup({name,open=false,children}:{name:CameraRawGroupName;open?:boolean;children:ReactNode}){
  const s=useEditor();const edit=s.adjustEdit!;const bypass=edit.cameraRawBypass??[];const enabled=!bypass.includes(name);
  return <details open={open}><summary>{name}</summary><div className="camera-raw-group-actions"><label><input type="checkbox" aria-label={`Enable ${name}`} checked={enabled} onChange={e=>s.updateAdjust({cameraRawBypass:e.target.checked?bypass.filter(n=>n!==name):[...bypass,name]})}/>Enabled</label><button onClick={()=>{if(edit.params?.filter==="CameraRaw")s.updateAdjust({params:{filter:"CameraRaw",settings:resetCameraRawGroup(edit.params.settings,name)},cameraRawBypass:bypass.filter(n=>n!==name)});}}>Reset {name}</button></div><fieldset disabled={!enabled} style={{padding:0,border:0,minWidth:0}}>{children}</fieldset></details>;
}

export function CameraRawPanel(){
  const s=useEditor();const params=s.adjustEdit!.params!;
  const [channel,setChannel]=useState<"rgb"|"red"|"green"|"blue">("rgb");
  const dragging=useRef<number|null>(null);
  if(params.filter!=="CameraRaw")return null;
  const settings=params.settings;
  const view=s.adjustEdit!.cameraRawView??{clipping:0,visualize:-1,sharpen_mask:false,shadow_overlay:false,highlight_overlay:false};
  const diagnostic=(patch:Partial<typeof view>)=>s.updateAdjust({cameraRawView:{...view,...patch}});
  const update=(next:CameraRawSettings)=>s.updateAdjust({params:{filter:"CameraRaw",settings:next}});
  const set=<K extends keyof CameraRawSettings>(key:K,value:CameraRawSettings[K])=>update({...settings,[key]:value});
  const nested=(key:"curve"|"detail"|"optics"|"geometry"|"calibration",field:string,value:number)=>update({...settings,[key]:{...settings[key],[field]:value}});
  const numbers=(group:string,key?:"curve"|"detail"|"optics"|"geometry"|"calibration",only?:string[])=>CAMERA_RAW_FIELDS.filter(f=>f.group===group&&(!only||only.includes(f.field))).map(f=>{
    const values=(key?settings[key]:settings) as unknown as Record<string,number>;
    return <Numeric key={f.field} name={`${key?caption(key)+" ":""}${caption(f.field)}`} value={values[f.field]} min={f.min} max={f.max} step={f.field==="exposure"?0.01:1}
      onChange={v=>key?nested(key,f.field,v):set(f.field as keyof CameraRawSettings,v as never)}/>;
  });
  const select=<K extends keyof CameraRawSettings>(name:string,key:K,values:CameraRawSettings[K][])=>
    <label>{name}<select aria-label={name} value={String(settings[key])} onChange={e=>set(key,e.target.value as CameraRawSettings[K])}>{values.map(v=><option key={String(v)}>{String(v)}</option>)}</select></label>;
  const curvePoints=settings.curve[channel];
  const points255=curvePoints.map(p=>({x:p.x*255,y:p.y*255}));
  const setPoints=(points:typeof points255)=>set("curve",{...settings.curve,[channel]:points.map(p=>({x:p.x/255,y:p.y/255}))});
  const curvePosition=(e:React.PointerEvent<SVGSVGElement>|React.MouseEvent<SVGSVGElement>)=>{const b=e.currentTarget.getBoundingClientRect();return{x:Math.max(0,Math.min(255,(e.clientX-b.left)/b.width*255)),y:Math.max(0,Math.min(255,(1-(e.clientY-b.top)/b.height)*255))};};
  return <div data-testid="camera-raw-controls">
    <CameraRawScope/>
    <fieldset><legend>Preview diagnostics</legend><label>Clipping view<select aria-label="Clipping view" value={view.clipping} onChange={e=>diagnostic({clipping:Number(e.target.value)})}><option value="0">Off</option><option value="1">Highlights</option><option value="2">Shadows</option></select></label>
      {([['sharpen_mask','Sharpening mask'],['shadow_overlay','Shadow clipping overlay'],['highlight_overlay','Highlight clipping overlay']] as const).map(([key,label])=><label key={key}><input type="checkbox" aria-label={label} checked={view[key]} onChange={e=>diagnostic({[key]:e.target.checked})}/>{label}</label>)}
      <small>Diagnostic views affect the preview only.</small></fieldset>
    <CameraRawGroup name="Light" open>{numbers("CameraRawSettings",undefined,["exposure","contrast","highlights","shadows","whites","blacks"])}</CameraRawGroup>
    <CameraRawGroup name="Color" open>{numbers("CameraRawSettings",undefined,["temperature","tint","vibrance","saturation"])}
      <button onClick={()=>{if(!s.engine||!s.activeId)return;const value=s.engine.cameraRawAutoBalance(s.activeId,s.adjustEdit!.layerId);if(value){const [temperature,tint]=value;update({...settings,whiteBalance:"Auto",temperature:Math.max(-100,Math.min(100,temperature)),tint:Math.max(-100,Math.min(100,tint))});}}}>Auto White Balance</button>
      <button aria-pressed={s.adjustEdit!.sampleMode==="CameraWhiteBalance"} onClick={()=>s.setAdjustSample(s.adjustEdit!.sampleMode==="CameraWhiteBalance"?null:"CameraWhiteBalance")}>White Balance Eyedropper</button>
    </CameraRawGroup>
    <CameraRawGroup name="Curve">{numbers("CameraRawCurve","curve")}
      <label>Point Channel<select aria-label="Point Channel" value={channel} onChange={e=>{dragging.current=null;setChannel(e.target.value as typeof channel);}}>{["rgb","red","green","blue"].map(c=><option key={c} value={c}>{c.toUpperCase()}</option>)}</select></label>
      <svg data-testid="camera-raw-curve" aria-label="Point curve" role="img" viewBox="0 0 256 256" width="256" height="256" style={{background:"#333",touchAction:"none"}}
        onPointerDown={e=>{if(e.button!==0)return;const p=curvePosition(e);const hit=nearestPoint(points255,p,8);if(hit!==null)dragging.current=hit;else{const next=insertPoint(points255,p);if(next.length===points255.length)return;dragging.current=next.findIndex(q=>q.x===Math.round(p.x));setPoints(next);}e.currentTarget.setPointerCapture(e.pointerId);}}
        onPointerMove={e=>{if(dragging.current!==null&&e.currentTarget.hasPointerCapture(e.pointerId))setPoints(movePoint(points255,dragging.current,curvePosition(e)));}}
        onPointerUp={()=>{dragging.current=null;}} onPointerCancel={()=>{dragging.current=null;}}
        onContextMenu={e=>{e.preventDefault();const hit=nearestPoint(points255,curvePosition(e),8);if(hit!==null)setPoints(removePoint(points255,hit));dragging.current=null;}}>
        <path d={curveSamples(points255).map((y,i)=>`${i?"L":"M"}${i},${255-y}`).join(" ")} fill="none" stroke="white"/>
        {curvePoints.map((p,i)=><circle key={i} cx={p.x*256} cy={(1-p.y)*256} r="4" fill="white"/>)}</svg>
      <button onClick={()=>set("curve",{...settings.curve,[channel]:linear()})}>Reset Point Curve</button>
    </CameraRawGroup>
    <CameraRawGroup name="Color Mixer">{families.map((name,i)=><fieldset key={name}><legend>{name}</legend>{(["hue","saturation","luminance"] as const).map(k=><Numeric key={k} name={`${name} ${caption(k)}`} value={settings.mixer[k][i]} min={-100} max={100} onChange={v=>{const values=[...settings.mixer[k]];values[i]=v;set("mixer",{...settings.mixer,[k]:values});}}/>)}</fieldset>)}</CameraRawGroup>
    <CameraRawGroup name="Point Color">{settings.mixer.points.map((p,i)=><fieldset key={i}><legend>Color {i+1}</legend>{CAMERA_RAW_FIELDS.filter(f=>f.group==="CameraRawPointColor").map(f=><Numeric key={f.field} name={`Color ${i+1} ${caption(f.field)}`} value={p[f.field as keyof CameraRawPointColor]} min={f.min} max={f.max} step={f.max<=1?0.01:1} onChange={v=>set("mixer",{...settings.mixer,points:settings.mixer.points.map((q,j)=>j===i?{...q,[f.field]:v}:q)})}/>)}<button onClick={()=>set("mixer",{...settings.mixer,points:settings.mixer.points.filter((_,j)=>j!==i)})}>Remove Color {i+1}</button></fieldset>)}
      <button disabled={settings.mixer.points.length>=8} onClick={()=>set("mixer",{...settings.mixer,points:[...settings.mixer.points,{hue:0,saturation:0,luminance:0,hueShift:0,saturationShift:0,luminanceShift:0,hueRange:30,saturationRange:0.4,luminanceRange:0.4}]})}>Add Point Color</button>
      <button disabled={settings.mixer.points.length>=8} aria-pressed={s.adjustEdit!.sampleMode==="CameraPointColor"} onClick={()=>s.setAdjustSample(s.adjustEdit!.sampleMode==="CameraPointColor"?null:"CameraPointColor")}>Point Color Eyedropper</button>
      <label>Visualize range<select aria-label="Visualize range" value={view.visualize} onChange={e=>diagnostic({visualize:Number(e.target.value)})}><option value="-1">Off</option>{settings.mixer.points.map((_,i)=><option key={i} value={i}>Color {i+1}</option>)}</select></label></CameraRawGroup>
    <CameraRawGroup name="Color Grading">{(["shadows","midtones","highlights","global"] as const).map(k=><fieldset key={k}><legend>{caption(k)}</legend><GradingWheel name={caption(k)} wheel={settings.grading[k]} disabled={s.adjustEdit!.cameraRawBypass?.includes("Color Grading")??false} onChange={wheel=>set("grading",{...settings.grading,[k]:wheel})}/>{(["hue","saturation","luminance"] as const).map(f=><Numeric key={f} name={`${caption(k)} Grading ${caption(f)}`} value={settings.grading[k][f]} min={f==="luminance"?-100:0} max={f==="hue"?360:100} onChange={v=>set("grading",{...settings.grading,[k]:{...settings.grading[k],[f]:v}})}/>)}</fieldset>)}
      <Numeric name="Grading Blending" value={settings.grading.blending} min={0} max={100} onChange={v=>set("grading",{...settings.grading,blending:v})}/>
      <Numeric name="Grading Balance" value={settings.grading.balance} min={-100} max={100} onChange={v=>set("grading",{...settings.grading,balance:v})}/></CameraRawGroup>
    <CameraRawGroup name="Effects">{select("Glow Style","glowStyle",["Diffusion","Bloom","Halation"])}{select("Vignette Style","vignetteStyle",["Highlight Priority","Color Priority","Paint Overlay"])}{numbers("CameraRawSettings",undefined,["texture","clarity","dehaze","glow","glowRange","glowSpread","glowWarmth","vignetteAmount","vignetteMidpoint","vignetteRoundness","vignetteFeather","vignetteHighlights","grainAmount","grainSize","grainRoughness"])}</CameraRawGroup>
    <CameraRawGroup name="Detail">{numbers("CameraRawDetail","detail")}</CameraRawGroup>
    <CameraRawGroup name="Optics">{(["removeChromaticAberration","enableLensProfile"] as const).map(k=><label key={k}><input type="checkbox" aria-label={caption(k)} checked={settings.optics[k]} onChange={e=>set("optics",{...settings.optics,[k]:e.target.checked})}/>{caption(k)}</label>)}<button aria-pressed={s.adjustEdit!.sampleMode==="CameraDefringe"} onClick={()=>s.setAdjustSample(s.adjustEdit!.sampleMode==="CameraDefringe"?null:"CameraDefringe")}>Defringe Eyedropper</button>{numbers("CameraRawOptics","optics")}</CameraRawGroup>
    <CameraRawGroup name="Geometry">{numbers("CameraRawGeometry","geometry")}
      <label>Upright<select aria-label="Upright" value={settings.geometry.upright} onChange={e=>set("geometry",{...settings.geometry,upright:e.target.value as "Off"|"Guided"})}><option>Off</option><option>Guided</option></select></label>
      {settings.geometry.upright==="Guided"&&<><small>Guides use percentages of the layer's width and height.</small>{settings.geometry.guides.map((guide,i)=><fieldset key={i}><legend>Guide {i+1}</legend>{(["startX","startY","endX","endY"] as const).map(k=><Numeric key={k} name={`Guide ${i+1} ${caption(k)} (%)`} value={guide[k]*100} min={0} max={100} step={.1} onChange={v=>set("geometry",{...settings.geometry,guides:settings.geometry.guides.map((g,j)=>j===i?{...g,[k]:v/100}:g)})}/>)}<button onClick={()=>set("geometry",{...settings.geometry,guides:settings.geometry.guides.filter((_,j)=>j!==i)})}>Remove Guide {i+1}</button></fieldset>)}<button disabled={settings.geometry.guides.length>=4} onClick={()=>set("geometry",{...settings.geometry,guides:[...settings.geometry.guides,settings.geometry.guides.length%2?{startX:.1,startY:.5,endX:.9,endY:.5}:{startX:.25,startY:.1,endX:.25,endY:.9}]})}>Add Upright Guide</button></>}
      <label>Projection<select aria-label="Projection" value={settings.geometry.projection} onChange={e=>set("geometry",{...settings.geometry,projection:e.target.value as "Perspective"|"Rectilinear"})}><option>Perspective</option><option>Rectilinear</option></select></label>
      <label><input type="checkbox" aria-label="Constrain Crop" checked={settings.geometry.constrainCrop} onChange={e=>set("geometry",{...settings.geometry,constrainCrop:e.target.checked})}/>Constrain Crop</label></CameraRawGroup>
    <CameraRawGroup name="Calibration"><label>Process<select aria-label="Process" value={settings.calibration.process} onChange={e=>set("calibration",{...settings.calibration,process:e.target.value as CameraRawSettings["calibration"]["process"]})}>{[1,2,3,4,5,6].map(n=><option key={n}>Version {n}</option>)}</select></label>{numbers("CameraRawCalibration","calibration")}</CameraRawGroup>
    <button onClick={()=>s.updateAdjust({params:{filter:"CameraRaw",settings:structuredClone(DEFAULT_CAMERA_RAW)},cameraRawBypass:[]})}>Reset All Camera Raw</button>
  </div>;
}
