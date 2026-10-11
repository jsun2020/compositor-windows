import {useEffect,useRef} from "react";
import {useEditor} from "../state/store";

export function Navigator(){
  const s=useEditor(), canvas=useRef<HTMLCanvasElement>(null);
  const d=s.activeId?s.documents[s.activeId]:null, vp=s.activeId?s.viewports[s.activeId]:null;
  const scale=d?Math.min(160/d.width,120/d.height):1;
  const w=d?Math.max(1,Math.round(d.width*scale)):1,h=d?Math.max(1,Math.round(d.height*scale)):1;
  const rect=d&&vp?vp.documentRect(d):null;
  const visible=!!(d&&vp&&rect&&s.showsNavigator&&!s.canvasOnly&&vp.zoom>=3&&(rect.x<0||rect.y<0||rect.x+rect.width>vp.viewSize.width||rect.y+rect.height>vp.viewSize.height));
  useEffect(()=>{
    if(!visible||!d||!s.engine)return;
    const timer=setTimeout(()=>{
      const target=canvas.current;if(!target)return;
      try{
        const data=s.engine!.composite(d.id,{x:0,y:0,width:d.width,height:d.height},w,h);
        const straight=new Uint8ClampedArray(data.length);
        for(let i=0;i<data.length;i+=4){const a=data[i+3];for(let c=0;c<3;c++)straight[i+c]=a?Math.min(255,Math.round(data[i+c]*255/a)):0;straight[i+3]=a;}
        target.getContext("2d")?.putImageData(new ImageData(straight,w,h),0,0);
      }catch(e){useEditor.getState().setError(String(e));}
    },100);return()=>clearTimeout(timer);
  },[visible,d,s.engine,w,h]);
  if(!visible||!d||!vp||!rect)return null;
  const x=Math.max(0,-rect.x/vp.pointsPerPixel),y=Math.max(0,-rect.y/vp.pointsPerPixel);
  const right=Math.min(d.width,(vp.viewSize.width-rect.x)/vp.pointsPerPixel),bottom=Math.min(d.height,(vp.viewSize.height-rect.y)/vp.pointsPerPixel);
  const move=(e:React.PointerEvent<HTMLElement>)=>{if(e.type!=="pointerdown"&&!e.currentTarget.hasPointerCapture(e.pointerId))return;e.stopPropagation();e.preventDefault();if(e.type==="pointerdown")e.currentTarget.setPointerCapture(e.pointerId);
    const r=e.currentTarget.getBoundingClientRect(),pixel={x:Math.max(0,Math.min(d.width,(e.clientX-r.left)/r.width*d.width)),y:Math.max(0,Math.min(d.height,(e.clientY-r.top)/r.height*d.height))};
    const current=vp.viewPoint(pixel,d);vp.translate({width:vp.center.x-current.x,height:vp.center.y-current.y});s.invalidate();};
  return <div data-testid="navigator" aria-label="Navigator" className="navigator" style={{width:w,height:h}} onPointerDown={move} onPointerMove={move} onPointerUp={e=>{e.stopPropagation();if(e.currentTarget.hasPointerCapture(e.pointerId))e.currentTarget.releasePointerCapture(e.pointerId);}}>
    <canvas ref={canvas} width={w} height={h}/><div data-testid="navigator-viewport" style={{position:"absolute",pointerEvents:"none",border:"1px solid white",boxShadow:"0 0 0 1px black",left:x/d.width*w,top:y/d.height*h,width:Math.max(0,right-x)/d.width*w,height:Math.max(0,bottom-y)/d.height*h}}/>
  </div>;
}
