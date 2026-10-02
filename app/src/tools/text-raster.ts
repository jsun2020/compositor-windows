import {validText,textFontAt,textColorAt,type TextStyle} from "./text-style";
const PADDING=12;
function family(name:string){return JSON.stringify(name.replace(/[\r\n]/g,""))+", sans-serif";}
type Glyph={text:string;index:number;font:string;color:[number,number,number];width:number;};
export function renderText(style:TextStyle):{width:number;height:number;pixels:ArrayBuffer}{
  if(!validText(style))throw Error("Text settings are out of range");
  const canvas=new OffscreenCanvas(1,1),ctx=canvas.getContext("2d",{willReadFrequently:true})!;
  const leading=style.leading||style.fontSize*1.2;
  const limit=style.boxSize?Math.max(1,style.boxSize[0]-2*PADDING):100_000;
  // Grapheme boundaries protect surrogate pairs, combining marks and joined emoji.
  const segments=Array.from(new Intl.Segmenter(undefined,{granularity:"grapheme"}).segment(style.content));
  const lines:Glyph[][]=[[]];let width=0;
  for(const segment of segments){
    if(/[\r\n]/.test(segment.segment)){lines.push([]);width=0;continue;}
    const font=textFontAt(style,segment.index);ctx.font=`${style.fontSize}px ${family(font)}`;
    const text=segment.segment==="\t"?"    ":segment.segment,w=ctx.measureText(text).width+style.tracking;
    if(style.boxSize&&width+w>limit&&lines[lines.length-1].length){
      const line=lines[lines.length-1];let breakAt=line.length-1;while(breakAt>=0&&!/\s/.test(line[breakAt].text))breakAt--;
      if(breakAt>=0){const tail=line.splice(breakAt+1);lines.push(tail);width=tail.reduce((n,g)=>n+g.width,0);}
      else{lines.push([]);width=0;}
    }
    lines[lines.length-1].push({text,index:segment.index,font,color:textColorAt(style,segment.index),width:w});width+=w;
  }
  const measured=lines.reduce((n,line)=>Math.max(n,line.reduce((n,g)=>n+g.width,0)),0);
  const w=Math.ceil(style.boxSize?.[0]??Math.max(16,measured+2*PADDING+style.fontSize*0.1));
  const h=Math.ceil(style.boxSize?.[1]??Math.max(16,Math.max(leading,lines.length*leading)+2*PADDING));
  if(w>30_000||h>30_000||w*h>100_000_000)throw Error("That text box exceeds the 30,000-pixel or 100-megapixel limit");
  canvas.width=w;canvas.height=h;ctx.textBaseline="alphabetic";
  const spacing="letterSpacing"in ctx;
  for(let row=0;row<lines.length;row++){
    const line=lines[row],length=line.reduce((n,g)=>n+g.width,0);
    let x=PADDING+(style.alignment==="Right"?w-2*PADDING-length:style.alignment==="Center"?(w-2*PADDING-length)/2:0);
    for(let i=0;i<line.length;){
      const first=line[i];let end=i+1;
      if(spacing)while(end<line.length&&line[end].font===first.font&&line[end].color.every((v,k)=>v===first.color[k]))end++;
      const text=line.slice(i,end).map(g=>g.text).join("");ctx.font=`${style.fontSize}px ${family(first.font)}`;
      if(spacing)(ctx as any).letterSpacing=`${style.tracking}px`;
      ctx.fillStyle=`rgb(${first.color.map(v=>Math.round(v*255)).join(",")})`;
      const descent=ctx.measureText("Mg").fontBoundingBoxDescent||style.fontSize*.2;
      ctx.fillText(text,x,PADDING+(row+1)*leading-descent);
      x+=spacing?ctx.measureText(text).width:line[i].width;i=end;
    }
  }
  const data=ctx.getImageData(0,0,w,h).data;
  for(let i=0;i<data.length;i+=4){const alpha=data[i+3]/255;for(let k=0;k<3;k++)data[i+k]=Math.round(data[i+k]*alpha);}
  return{width:w,height:h,pixels:data.buffer as ArrayBuffer};
}
