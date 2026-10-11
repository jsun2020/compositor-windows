import {useEffect,useState} from "react";
import {useEditor} from "../state/store";
import {Sheet} from "./Sheet";

export function ExportAsSheet(){
  const s=useEditor(),d=s.activeId?s.documents[s.activeId]:null;
  const [width,setWidth]=useState(String(d?.width??1)),[height,setHeight]=useState(String(d?.height??1));
  const [format,setFormat]=useState<"PNG"|"JPEG"|"PDF">("PNG"),[quality,setQuality]=useState(85),[background,setBackground]=useState("Transparent"),[locked,setLocked]=useState(true);
  const [url,setUrl]=useState<string|null>(null),[saving,setSaving]=useState(false),[error,setError]=useState<string|null>(null);
  const w=Number(width),h=Number(height),valid=Number.isInteger(w)&&Number.isInteger(h)&&w>0&&h>0&&w<=30000&&h<=30000&&w*h<=100_000_000;
  const matte=background==="Transparent"&&format!=="JPEG"?null:background==="Black"?[0,0,0]as [number,number,number]:[1,1,1]as [number,number,number];
  const options={width:w,height:h,format,quality:quality/100,background:matte};
  const setDimension=(horizontal:boolean,value:string)=>{if(horizontal)setWidth(value);else setHeight(value);if(locked&&d&&Number(value)>0){if(horizontal)setHeight(String(Math.max(1,Math.round(Number(value)*d.height/d.width))));else setWidth(String(Math.max(1,Math.round(Number(value)*d.width/d.height))));}};
  useEffect(()=>{
    if(!valid||!d||!s.engine)return;let object:string|null=null;
    const timer=setTimeout(()=>{try{const bytes=s.engine!.exportAs(d.id,options,640);object=URL.createObjectURL(new Blob([bytes.slice().buffer],{type:format==="JPEG"?"image/jpeg":"image/png"}));setUrl(object);setError(null);}catch(e){setError(String(e));}},150);
    return()=>{clearTimeout(timer);if(object)URL.revokeObjectURL(object);};
  },[valid,w,h,format,quality,background,d?.id,s.engine]);
  const save=async()=>{
    if(!valid||!d||!s.bridge||!s.engine||saving)return;setSaving(true);s.setBusy(true);
    try{const path=await s.bridge.pickExportFile(d.path?s.bridge.baseName(d.path):s.untitledNames[d.id]??"Untitled",format==="JPEG"?"jpg":format==="PDF"?"pdf":"png");if(!path)return;
      await new Promise<void>(resolve=>requestAnimationFrame(()=>resolve()));
      const bytes=s.engine.exportAs(d.id,options);await s.bridge.writeFile(path,bytes);s.closeSheet();
    }catch(e){setError(String(e));}finally{setSaving(false);useEditor.getState().setBusy(false);}
  };
  if(!d)return null;
  return <Sheet title="Export As" primary={saving?"Exporting...":"Export"} canConfirm={valid&&!saving} onCancel={()=>{if(!saving)s.closeSheet();}} onConfirm={()=>void save()}>
    <label>Format <select aria-label="Format" value={format} disabled={saving} onChange={e=>{const next=e.target.value as typeof format;setFormat(next);if(next==="JPEG"&&background==="Transparent")setBackground("White");}}>{["PNG","JPEG","PDF"].map(f=><option key={f}>{f}</option>)}</select></label>
    <label>Width <input aria-label="Export Width" value={width} disabled={saving} onChange={e=>setDimension(true,e.target.value)}/> px</label>
    <label>Height <input aria-label="Export Height" value={height} disabled={saving} onChange={e=>setDimension(false,e.target.value)}/> px</label>
    <label><input type="checkbox" aria-label="Lock Proportions" checked={locked} disabled={saving} onChange={e=>setLocked(e.target.checked)}/> Lock Proportions</label>
    {format==="JPEG"&&<label>Quality <input type="number" aria-label="Quality" min={1} max={100} value={quality} disabled={saving} onChange={e=>setQuality(Math.min(100,Math.max(1,Number(e.target.value))))}/>%</label>}
    <label>Background <select aria-label="Export Background" value={background} disabled={saving} onChange={e=>setBackground(e.target.value)}>{(format==="JPEG"?["White","Black"]:["Transparent","White","Black"]).map(b=><option key={b}>{b}</option>)}</select></label>
    {url&&valid&&<img className="export-preview" data-testid="export-preview" src={url} alt="Export preview"/>}
    {format==="PDF"&&valid&&<p>Page: {(w/d.resolution*72).toFixed(2)} × {(h/d.resolution*72).toFixed(2)} pt · {d.resolution} DPI</p>}
    {!valid&&<p>Choose dimensions up to 30,000 px per side and 100 megapixels.</p>}{error&&<p role="alert">{error}</p>}
  </Sheet>;
}
