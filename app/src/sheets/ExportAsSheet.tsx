import {useColorProof} from "../state/color-proof";
import {proofPng} from "../canvas/color-proof";
import {ColorProfilePicker} from "./ColorProofSheet";
import {useEffect,useState} from "react";
import {useEditor} from "../state/store";
import {Sheet} from "./Sheet";

export function ExportAsSheet(){
  const s=useEditor(),d=s.activeId?s.documents[s.activeId]:null;
  const [width,setWidth]=useState(String(d?.width??1)),[height,setHeight]=useState(String(d?.height??1));
  const [format,setFormat]=useState<"PNG"|"JPEG"|"PDF"|"PNG16"|"EXR"|"TIFF"|"ChannelPack"|"GIF">("PNG"),[quality,setQuality]=useState(85),[background,setBackground]=useState("Transparent"),[locked,setLocked]=useState(true);
  const [channels,setChannels]=useState<string[]>(()=>[["r","red"],["g","green"],["b","blue"],["a","alpha"]].map((names,c)=>d?.layers.slice().reverse().find(l=>names.includes(l.name.trim().toLowerCase()))?.id ?? (c===3?"White":"Black")));
  const [gifLoop,setGifLoop]=useState(true);
  const [colorSpace,setColorSpace]=useState("RGB"),color=useColorProof();
  const printFormat=format==="JPEG"||format==="TIFF"||format==="PDF",cmyk=printFormat&&colorSpace==="CMYK";
  const canPrint=!cmyk||!!(color.profile&&color.lut&&!color.loading&&s.bridge?.exportCmyk);
  const [url,setUrl]=useState<string|null>(null),[saving,setSaving]=useState(false),[error,setError]=useState<string|null>(null);
  const w=Number(width),h=Number(height),valid=Number.isInteger(w)&&Number.isInteger(h)&&w>0&&h>0&&w<=30000&&h<=30000&&w*h<=100_000_000;
  const matte=background==="Transparent"&&format!=="JPEG"&&!cmyk?null:background==="Black"?[0,0,0]as [number,number,number]:[1,1,1]as [number,number,number];
  const options={width:w,height:h,format,quality:quality/100,background:matte,gifLoop,...(format==="ChannelPack"?{channelSources:channels.map(v=>v==="White"||v==="Black"?{kind:v as "White"|"Black"}:{kind:"Layer" as const,id:v})}: {})};
  const setDimension=(horizontal:boolean,value:string)=>{if(horizontal)setWidth(value);else setHeight(value);if(locked&&d&&Number(value)>0){if(horizontal)setHeight(String(Math.max(1,Math.round(Number(value)*d.height/d.width))));else setWidth(String(Math.max(1,Math.round(Number(value)*d.width/d.height))));}};
  useEffect(()=>{
    if(!valid||!d||!s.engine||!canPrint){setUrl(null);return;}let object:string|null=null,cancelled=false;
    const timer=setTimeout(()=>{void(async()=>{try{
      let bytes=s.engine!.exportAs(d.id,cmyk?{...options,format:"PNG"}:options,640);
      if(cmyk)bytes=await proofPng(bytes,color);
      if(cancelled)return;object=URL.createObjectURL(new Blob([bytes.slice().buffer],{type:!cmyk&&format==="JPEG"?"image/jpeg":format==="GIF"?"image/gif":"image/png"}));setUrl(object);setError(null);
    }catch(e){if(!cancelled)setError(String(e));}})();},150);
    return()=>{cancelled=true;clearTimeout(timer);if(object)URL.revokeObjectURL(object);};
  },[valid,w,h,format,quality,background,channels,gifLoop,d?.id,s.engine,cmyk,canPrint,color.revision]);
  const save=async()=>{
    if(!valid||!canPrint||!d||!s.bridge||!s.engine||saving)return;setSaving(true);s.setBusy(true);
    try{const path=await s.bridge.pickExportFile(d.path?s.bridge.baseName(d.path):s.untitledNames[d.id]??"Untitled",format==="GIF"?"gif":format==="JPEG"?"jpg":format==="PDF"?"pdf":format==="EXR"?"exr":format==="TIFF"?"tif":"png");if(!path)return;
      await new Promise<void>(resolve=>requestAnimationFrame(()=>resolve()));
      const bytes=cmyk&&printFormat?await s.bridge.exportCmyk!(s.engine.exportAs(d.id,{...options,format:"PNG"}),color.profile!.path,format,quality/100,d.resolution):s.engine.exportAs(d.id,options);await s.bridge.writeFile(path,bytes);s.closeSheet();
    }catch(e){setError(String(e));}finally{setSaving(false);useEditor.getState().setBusy(false);}
  };
  if(!d)return null;
  return <Sheet title="Export As" primary={saving?"Exporting...":"Export"} canConfirm={valid&&canPrint&&!saving} onCancel={()=>{if(!saving)s.closeSheet();}} onConfirm={()=>void save()}>
    <label>Format <select aria-label="Format" value={format} disabled={saving} onChange={e=>{const next=e.target.value as typeof format;setFormat(next);if(next==="JPEG"&&background==="Transparent")setBackground("White");}}>{["PNG","PNG16","JPEG","TIFF","EXR","PDF","ChannelPack","GIF"].map(f=><option key={f} value={f}>{f==="PNG16"?"PNG (16-bit)":f==="ChannelPack"?"Channel Pack (PNG)":f}</option>)}</select></label>
    {printFormat&&<><label>Color space <select aria-label="Export Color Space" value={colorSpace} disabled={saving} onChange={e=>setColorSpace(e.target.value)}><option>RGB</option><option>CMYK</option></select></label>{cmyk&&<ColorProfilePicker disabled={saving}/>}</>}
    <label>Width <input aria-label="Export Width" value={width} disabled={saving} onChange={e=>setDimension(true,e.target.value)}/> px</label>
    <label>Height <input aria-label="Export Height" value={height} disabled={saving} onChange={e=>setDimension(false,e.target.value)}/> px</label>
    <label><input type="checkbox" aria-label="Lock Proportions" checked={locked} disabled={saving} onChange={e=>setLocked(e.target.checked)}/> Lock Proportions</label>
    {format==="GIF"&&<><label><input type="checkbox" aria-label="Loop forever" checked={gifLoop} disabled={saving} onChange={e=>setGifLoop(e.target.checked)}/> Loop forever</label><p>Visible animation layers share one timeline; shorter tracks hold their last frame. Export supports 4096 frames and 100 megapixels across all frames.</p></>}
    {format==="JPEG"&&<label>Quality <input type="number" aria-label="Quality" min={1} max={100} value={quality} disabled={saving} onChange={e=>setQuality(Math.min(100,Math.max(1,Number(e.target.value))))}/>%</label>}
    {format!=="ChannelPack"&&<label>Background <select aria-label="Export Background" value={cmyk&&background==="Transparent"?"White":background} disabled={saving} onChange={e=>setBackground(e.target.value)}>{(format==="JPEG"||cmyk?["White","Black"]:["Transparent","White","Black"]).map(b=><option key={b}>{b}</option>)}</select></label>}
    {format==="ChannelPack"&&<>{["Red","Green","Blue","Alpha"].map((name,i)=><label key={name}>{name} <select aria-label={`${name} Source`} value={channels[i]} disabled={saving} onChange={e=>setChannels(old=>old.map((v,k)=>k===i?e.target.value:v))}><option value="Black">Black</option><option value="White">White</option>{d.layers.map(l=><option key={l.id} value={l.id}>{l.name}</option>)}</select></label>)}<p>Each source is read as grayscale over black. The preview shows RGB; exported alpha keeps the color data even where it is zero.</p></>}
    {url&&valid&&<img className="export-preview" data-testid="export-preview" src={url} alt="Export preview"/>}
    {format==="PDF"&&valid&&<p>Page: {(w/d.resolution*72).toFixed(2)} × {(h/d.resolution*72).toFixed(2)} pt · {d.resolution} DPI</p>}
    {!valid&&<p>Choose dimensions up to 30,000 px per side and 100 megapixels.</p>}{error&&<p role="alert">{error}</p>}
  </Sheet>;
}
