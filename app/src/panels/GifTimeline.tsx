import {useEffect,useRef,useState,memo} from "react";
import {useEditor} from "../state/store";
import type {GifEdit,GifTiming} from "../engine/types";

const Thumbnail=memo(function Thumbnail({doc,layer,index,revision}:{doc:string;layer:string;index:number;revision:number|null}){
  const engine=useEditor(s=>s.engine),[url,setUrl]=useState<string>();
  useEffect(()=>{
    if(!engine)return;
    let url:string|undefined;
    try{const bytes=engine.gifThumbnail(doc,layer,index);url=URL.createObjectURL(new Blob([bytes.slice().buffer],{type:"image/png"}));setUrl(url);}
    catch{setUrl(undefined);}
    return()=>{if(url)URL.revokeObjectURL(url);};
  },[engine,doc,layer,index,revision]);
  return url?<img src={url} alt=""/>:<span className="gif-thumbnail-placeholder"/>;
});

export function GifTimeline(){
  const s=useEditor(),doc=s.activeId?s.documents[s.activeId]:null;
  const layer=doc?.layers.find(l=>l.id===doc.activeLayerId),timeline=layer?.gif;
  const [playing,setPlaying]=useState(false),[speed,setSpeed]=useState(1),[delay,setDelay]=useState("100");
  const latest=useRef({timeline,doc:doc?.id,layer:layer?.id});latest.current={timeline,doc:doc?.id,layer:layer?.id};
  const disabled=!!(s.busy||s.working||s.sheet||s.adjustEdit||s.transformEdit||s.gradientEdit||s.brushDraft||s.textEdit||s.effectsEdit);
  const selected=timeline?.frames.findIndex(f=>f.index===timeline.selectedFrameIndex)??0;
  const duration=timeline?.timing.mode==="Same for all"?timeline.timing.uniformDelay:timeline?.frames[selected]?.duration;
  useEffect(()=>setDelay(String(Math.round((duration??.1)*1000))),[duration,layer?.id,timeline?.selectedFrameIndex]);
  useEffect(()=>{setPlaying(false);},[doc?.id,layer?.id,disabled,doc?.undoEntryId]);
  const timingKey=timeline?JSON.stringify([timeline.frames.map(f=>[f.index,f.duration]),timeline.timing]):"";
  useEffect(()=>{
    if(!playing||disabled||!timeline||timeline.frames.length<2||!doc||!layer)return;
    const frames=timeline.frames,durations=frames.map(f=>f.duration*1000/speed),total=durations.reduce((a,b)=>a+b,0);
    const initial=frames.findIndex(f=>f.index===timeline.selectedFrameIndex);
    const offset=durations.slice(0,initial).reduce((a,b)=>a+b,0),started=performance.now();let raf=0;
    const tick=(now:number)=>{
      if(document.hidden){setPlaying(false);return;}
      let time=(now-started+offset)%total,index=0;
      while(index<durations.length-1&&time>=durations[index]){time-=durations[index];index++;}
      const current=latest.current;
      if(current.doc!==doc.id||current.layer!==layer.id)return;
      if(current.timeline?.selectedFrameIndex!==frames[index].index){
        if(!useEditor.getState().run({type:"SelectGifFrame",id:layer.id,index:frames[index].index})){setPlaying(false);return;}
      }
      raf=requestAnimationFrame(tick);
    };
    raf=requestAnimationFrame(tick);
    const stop=(event:Event)=>{if(!(event.target instanceof Element)||!event.target.closest(".gif-timeline"))setPlaying(false);};
    document.addEventListener("pointerdown",stop,true);document.addEventListener("keydown",stop,true);
    return()=>{cancelAnimationFrame(raf);document.removeEventListener("pointerdown",stop,true);document.removeEventListener("keydown",stop,true);};
    // Navigation changes selectedFrameIndex each tick; it must not restart the playback clock.
  },[playing,disabled,doc?.id,layer?.id,timingKey,speed]);
  if(!doc||!layer||!timeline)return null;
  const select=(position:number)=>{setPlaying(false);const f=timeline.frames[position];if(f)s.run({type:"SelectGifFrame",id:layer.id,index:f.index});};
  const edit=(edit:GifEdit)=>{setPlaying(false);s.run({type:"EditGif",id:layer.id,edit});};
  const setTiming=(timing:GifTiming)=>edit({type:"Timing",timing});
  const commitDelay=()=>{
    const ms=Number(delay);if(!delay.trim()||!Number.isFinite(ms)||ms<20||ms>655350){setDelay(String(Math.round((duration??.1)*1000)));return;}
    if(ms/1000===duration)return;
    const timing=structuredClone(timeline.timing);
    if(timing.mode==="Same for all")timing.uniformDelay=ms/1000;
    else{timing.frameDelays=timing.frameDelays.filter(f=>f.index!==timeline.selectedFrameIndex);timing.frameDelays.push({index:timeline.selectedFrameIndex,duration:ms/1000});}
    setTiming(timing);
  };
  // A bounded thumbnail window keeps 4096-frame imports from allocating thousands of URLs.
  const start=Math.max(0,Math.floor(selected/20)*20-2),visible=timeline.frames.slice(start,start+24);
  return <section className="gif-timeline" aria-label="Animation timeline">
    <div className="gif-controls">
      <button disabled={disabled||timeline.frames.length<2} onClick={()=>setPlaying(!playing)}>{playing?"Pause animation":"Play animation"}</button>
      <button disabled={disabled||selected===0} aria-label="Previous frame" onClick={()=>select(selected-1)}>◀</button>
      <label>Frame <input aria-label="Animation frame" type="number" min={1} max={timeline.frames.length} value={selected+1} disabled={disabled} onChange={e=>select(Number(e.target.value)-1)}/></label><span>/ {timeline.frames.length}</span>
      <button disabled={disabled||selected===timeline.frames.length-1} aria-label="Next frame" onClick={()=>select(selected+1)}>▶</button>
      <label>Timing <select aria-label="Animation timing" value={timeline.timing.mode} disabled={disabled} onChange={e=>setTiming({...timeline.timing,mode:e.target.value as GifTiming["mode"]})}><option>Frame delay</option><option>Same for all</option></select></label>
      <label>Delay <input aria-label="Frame delay" type="number" min={20} max={655350} value={delay} disabled={disabled} onChange={e=>setDelay(e.target.value)} onBlur={commitDelay} onKeyDown={e=>{if(e.key==="Enter"){e.preventDefault();commitDelay();}}}/> ms</label>
      <label>Preview speed <select aria-label="Animation preview speed" value={speed} onChange={e=>setSpeed(Number(e.target.value))}>{[.25,.5,1,1.5,2,4].map(v=><option key={v} value={v}>{v}×</option>)}</select></label>
      <button disabled={disabled||timeline.frames.length>=4096} onClick={()=>edit({type:"Duplicate",index:timeline.selectedFrameIndex})}>Duplicate frame</button>
      <button disabled={disabled||timeline.frames.length<2} onClick={()=>edit({type:"Delete",index:timeline.selectedFrameIndex})}>Delete frame</button>
      <button disabled={disabled||selected===0} onClick={()=>edit({type:"Move",index:timeline.selectedFrameIndex,position:selected-1})}>Move frame left</button>
      <button disabled={disabled||selected===timeline.frames.length-1} onClick={()=>edit({type:"Move",index:timeline.selectedFrameIndex,position:selected+1})}>Move frame right</button>
    </div>
    <div className="gif-frames" role="list" aria-label="Animation frames">
      {visible.map((frame,i)=><button key={frame.index} role="listitem" disabled={disabled} aria-label={`Frame ${start+i+1}`} aria-current={frame.index===timeline.selectedFrameIndex?"true":undefined} onClick={()=>select(start+i)}>
        <Thumbnail doc={doc.id} layer={layer.id} index={frame.index} revision={frame.edited?doc.undoEntryId:null}/>
        <span>{start+i+1}{frame.edited?" · edited":""} · {Math.round(frame.duration*1000)} ms</span>
      </button>)}
    </div>
  </section>;
}
