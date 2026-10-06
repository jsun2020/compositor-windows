import {useEffect,useRef,useState} from "react";
import {createRoot} from "react-dom/client";
import {Sheet} from "./Sheet";
import {NumberInput} from "../panels/NumberInput";
import type {RawInfo,RawDevelopSettings,ShellBridge} from "../shell/bridge";
import {useEditor} from "../state/store";
export const isCameraRaw=(path:string)=>/\.(dng|cr2|cr3|crw|nef|nrw|arw|srf|sr2|raf|rw2|orf|pef|ptx|raw|rwl|srw|3fr|fff|iiq|kdc|dcr|mos|mef|erf|x3f)$/i.test(path);
function RawDevelopSheet({path,bridge,done}:{path:string;bridge:ShellBridge;done(bytes:Uint8Array|null):void}){
  const [info,setInfo]=useState<RawInfo|null>(null);const [settings,setSettings]=useState<RawDevelopSettings|null>(null);
  const [preview,setPreview]=useState<string|null>(null);const [pending,setPending]=useState(false);const [applying,setApplying]=useState(false);const [error,setError]=useState<string|null>(null);
  const alive=useRef(true),generation=useRef(0),url=useRef<string|null>(null),active=useRef<string|null>(null),full=useRef<string|null>(null);
  const cancelToken=(token:string|null)=>{if(token)void bridge.rawCancel!(token).catch(()=>{});};
  useEffect(()=>{let live=true,pending=true;const token=crypto.randomUUID();void bridge.rawInspect!(path,token).then(info=>{if(!live)return;setInfo(info);setSettings({exposure:0,temperature:info.asShotTemperature,tint:info.asShotTint,tone:1});}).catch(e=>{if(live)setError(String(e));}).finally(()=>{pending=false;});return()=>{live=false;if(pending)cancelToken(token);};},[path,bridge]);
  useEffect(()=>{alive.current=true;return()=>{alive.current=false;generation.current++;cancelToken(active.current);cancelToken(full.current);if(url.current)URL.revokeObjectURL(url.current);};},[]);
  useEffect(()=>{
    if(!settings)return;const request=++generation.current;setPending(true);setError(null);cancelToken(active.current);active.current=null;
    const timer=setTimeout(()=>{if(!alive.current||generation.current!==request)return;const token=crypto.randomUUID();active.current=token;void bridge.rawDevelop!(path,settings,true,token).then(bytes=>{
      if(!alive.current||generation.current!==request)return;if(url.current)URL.revokeObjectURL(url.current);url.current=URL.createObjectURL(new Blob([bytes as unknown as BlobPart],{type:"image/png"}));setPreview(url.current);
    }).catch(e=>{if(alive.current&&generation.current===request)setError(String(e));}).finally(()=>{if(active.current===token)active.current=null;if(alive.current&&generation.current===request)setPending(false);});},180);
    return()=>{clearTimeout(timer);generation.current++;cancelToken(active.current);active.current=null;};
  },[settings,path,bridge]);
  const apply=async()=>{
    if(!settings||applying)return;setApplying(true);setPending(false);generation.current++;cancelToken(active.current);active.current=null;const token=crypto.randomUUID();full.current=token;setError(null);
    try{const bytes=await bridge.rawDevelop!(path,settings,false,token);full.current=null;if(alive.current)done(bytes);}catch(e){full.current=null;if(alive.current){setError(String(e));setApplying(false);}}
  };
  return <Sheet title="Develop Camera RAW" fieldOwnsEnter primary={applying?"Developing…":"Import"} canConfirm={!!settings&&!applying&&!error} onConfirm={()=>void apply()} onCancel={()=>done(null)}>
    {info?<div>{info.make} {info.model} · {info.width} × {info.height}</div>:<div>Reading camera sensor metadata…</div>}
    {preview&&<img data-testid="raw-preview" src={preview} alt="Camera RAW preview" style={{maxWidth:640,maxHeight:Math.max(160,Math.min(320,window.innerHeight-360)),objectFit:"contain"}}/>}
    {pending&&<div role="status">Developing preview…</div>}
    {settings&&<fieldset disabled={applying} style={{border:0,padding:0}}>{([ ["Exposure","exposure",-5,5,.05],["Temperature","temperature",2000,15000,25],["Tint","tint",-150,150,1],["Tone curve","tone",0,1,.01]] as const).map(([label,key,min,max,step])=><label key={key}>{label}<input type="range" aria-hidden tabIndex={-1} min={min} max={max} step={step} value={settings[key]} onChange={e=>setSettings({...settings,[key]:Number(e.target.value)})}/><NumberInput label={label} value={settings[key]} min={min} max={max} step={step} onChange={v=>setSettings({...settings,[key]:v})}/></label>)}
      <button onClick={()=>setSettings({exposure:0,temperature:info!.asShotTemperature,tint:info!.asShotTint,tone:1})}>Reset to As Shot</button></fieldset>}
    <small>White balance starts from the camera settings. Temperature is estimated; Windows uses LibRaw's tone interpretation.</small>
    {error&&<div role="alert">{error}</div>}
  </Sheet>;
}
/** The import guard remains busy while this dialog is open. Nothing reaches the
 * engine until Import has finished a full sensor development successfully. */
export function developCameraRaw(path:string,bridge:ShellBridge):Promise<Uint8Array|null>{
  if(!bridge.rawInspect||!bridge.rawDevelop||!bridge.rawCancel)return Promise.reject(new Error("Camera RAW import requires the Windows desktop package."));
  return new Promise(resolve=>{useEditor.getState().openSheet({kind:"rawDevelop"});const host=document.createElement("div");host.dataset.rawDevelop="true";document.body.append(host);const root=createRoot(host);let settled=false;
    const done=(bytes:Uint8Array|null)=>{if(settled)return;settled=true;queueMicrotask(()=>{root.unmount();host.remove();if(useEditor.getState().sheet?.kind==="rawDevelop")useEditor.getState().closeSheet();resolve(bytes);});};root.render(<RawDevelopSheet path={path} bridge={bridge} done={done}/>);
  });
}
