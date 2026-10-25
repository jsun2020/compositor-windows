import type {ShapeOptions} from "./shape-draft";
type Point={x:number;y:number};
/** Compositor 1.5.7 ShapeTool.swift, also used by the engine's shape_geometry. */
export function smoothedRectPath(ctx:CanvasRenderingContext2D,x:number,y:number,w:number,h:number,radius:number,smoothing:boolean):void {
 const r=Math.min(Math.max(0,radius),w/2,h/2), s=r>0&&smoothing?Math.min(.6,Math.max(0,Math.min(w,h)/(2*r)-1)):0;
 if(!s){ctx.roundRect(x,y,w,h,r);return;}
 const reach=(1+s)*r,angle=s*Math.PI/4,t=r*Math.tan(angle/2),c=t*Math.cos(angle),d=t*Math.sin(angle),arc=r*Math.SQRT2*Math.sin((1-s)*Math.PI/4),b=(reach-arc-c-d)/3,a=2*b;
 [[x+w,y],[x+w,y+h],[x,y+h],[x,y]].forEach(([cx,cy],i)=>{
  ctx.save();ctx.translate(cx,cy);ctx.rotate(i*Math.PI/2);
  if(i===0)ctx.moveTo(-reach,0);else ctx.lineTo(-reach,0);
  ctx.bezierCurveTo(-reach+a,0,-reach+a+b,0,-reach+a+b+c,d);
  ctx.arc(-r,r,r,-Math.PI/2+angle,-angle);
  ctx.bezierCurveTo(0,reach-a-b,0,reach-a,0,reach);ctx.restore();
 });ctx.closePath();
}
export function drawStyledLine(ctx:CanvasRenderingContext2D,a:Point,b:Point,width:number,style:NonNullable<ShapeOptions["lineStyle"]>,cap:NonNullable<ShapeOptions["lineCap"]>):void {
 const length=Math.hypot(b.x-a.x,b.y-a.y),ux=length?(b.x-a.x)/length:1,uy=length?(b.y-a.y)/length:0,total=length+width;
 const point=(d:number)=>({x:a.x+ux*(d-width/2),y:a.y+uy*(d-width/2)});
 const [dash,gap]=style==="Solid"?[total,0]:style==="Dotted"?[width,width]:style==="Dashed"?[3*width,2*width]:[7*width,2*width];
 ctx.lineWidth=width;ctx.setLineDash([]);ctx.lineCap=cap==="Square"?"butt":"round";
 for(let d=0;d<total;){const next=Math.min(total,d+dash);ctx.beginPath();
  if(cap==="Round"&&next-d-width<=.001){const p=point((d+next)/2);ctx.ellipse(p.x,p.y,width/2,width/2,0,0,2*Math.PI);ctx.fill();}
  else{const p=point(d+(cap==="Round"?width/2:0)),q=point(next-(cap==="Round"?width/2:0));ctx.moveTo(p.x,p.y);ctx.lineTo(q.x,q.y);ctx.stroke();}
  d=next+gap;
 }
}
