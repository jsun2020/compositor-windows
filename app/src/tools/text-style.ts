/** Mac LayerTextStyle: run locations and lengths are UTF-16 units, as JS strings are. */
export interface TextColorRun {location:number;length:number;red:number;green:number;blue:number;[key:string]:unknown;}
export interface TextFontRun {location:number;length:number;fontName:string;[key:string]:unknown;}
export interface TextStyle {
  content:string;fontName:string;fontSize:number;red:number;green:number;blue:number;
  alignment:"Left"|"Center"|"Right";tracking:number;leading:number;
  boxSize?:[number,number]|null;colorRuns?:TextColorRun[]|null;fontRuns?:TextFontRun[]|null;
  [key:string]:unknown;
}
export const DEFAULT_TEXT:TextStyle={content:"",fontName:"Arial",fontSize:72,red:0,green:0,blue:0,alignment:"Left",tracking:0,leading:0};
export type TextRange={start:number;end:number};
const bounded=(style:TextStyle,range:TextRange)=>({start:Math.min(style.content.length,Math.max(0,range.start)),end:Math.min(style.content.length,Math.max(range.start,range.end))});
function runAt<T extends {location:number;length:number}>(runs:T[]|null|undefined,index:number):T|undefined{let lo=0,hi=runs?.length??0;while(lo<hi){const mid=(lo+hi)>>>1;if(runs![mid].location<=index)lo=mid+1;else hi=mid;}const r=runs?.[lo-1];return r&&index<r.location+r.length?r:undefined;}
export const textColorAt=(style:TextStyle,index:number):[number,number,number]=>{const r=runAt(style.colorRuns,index);return r?[r.red,r.green,r.blue]:[style.red,style.green,style.blue];};
export const textFontAt=(style:TextStyle,index:number)=>runAt(style.fontRuns,index)?.fontName??style.fontName;
function compact<T>(units:T[],base:T,equal:(a:T,b:T)=>boolean,make:(value:T,location:number,length:number)=>any){
  const runs:any[]=[];let i=0;while(i<units.length){let end=i+1;while(end<units.length&&equal(units[i],units[end]))end++;if(!equal(units[i],base))runs.push(make(units[i],i,end-i));i=end;}return runs.length?runs:null;
}
const sameColor=(a:[number,number,number],b:[number,number,number])=>a.every((v,i)=>v===b[i]);
function colors(style:TextStyle,units:[number,number,number][]){return compact(units,[style.red,style.green,style.blue],sameColor,(v,location,length)=>({location,length,red:v[0],green:v[1],blue:v[2]})) as TextColorRun[]|null;}
function fonts(style:TextStyle,units:string[]){return compact(units,style.fontName,(a,b)=>a===b,(fontName,location,length)=>({location,length,fontName})) as TextFontRun[]|null;}
export function colorText(style:TextStyle,range:TextRange,color:[number,number,number]):TextStyle{
  const{start,end}=bounded(style,range);if(start===end||start===0&&end===style.content.length)return{...style,red:color[0],green:color[1],blue:color[2],colorRuns:null};
  const units=Array.from({length:style.content.length},(_,i)=>i>=start&&i<end?color:textColorAt(style,i));return{...style,colorRuns:colors(style,units)};
}
export function fontText(style:TextStyle,range:TextRange,fontName:string):TextStyle{
  const{start,end}=bounded(style,range);if(start===end||start===0&&end===style.content.length)return{...style,fontName,fontRuns:null};
  const units=Array.from({length:style.content.length},(_,i)=>i>=start&&i<end?fontName:textFontAt(style,i));
  if(units.every(v=>v===units[0]))return{...style,fontName:units[0],fontRuns:null};return{...style,fontRuns:fonts(style,units)};
}
/** A replacement inherits the preceding letter's face/color; runs after it move by the delta. */
export function replaceText(style:TextStyle,next:string):TextStyle{
  let start=0,end=style.content.length,newEnd=next.length;
  while(start<end&&start<newEnd&&style.content[start]===next[start])start++;
  while(end>start&&newEnd>start&&style.content[end-1]===next[newEnd-1]){end--;newEnd--;}
  const inherited=start>0?start-1:Math.min(start,Math.max(0,style.content.length-1));
  const result={...style,content:next};
  if(style.colorRuns){const units=Array.from({length:style.content.length},(_,i)=>textColorAt(style,i));units.splice(start,end-start,...Array.from({length:newEnd-start},()=>textColorAt(style,inherited)));result.colorRuns=colors(style,units);}
  if(style.fontRuns){const units=Array.from({length:style.content.length},(_,i)=>textFontAt(style,i));units.splice(start,end-start,...Array.from({length:newEnd-start},()=>textFontAt(style,inherited)));result.fontRuns=fonts(style,units);}
  return result;
}
export function validText(style:TextStyle):boolean{
  if(typeof style.content!=="string"||style.content.length>100_000||typeof style.fontName!=="string"||!style.fontName||style.fontName.length>200||/[\r\n]/.test(style.fontName))return false;
  const range=(n:number,a:number,b:number)=>Number.isFinite(n)&&n>=a&&n<=b;
  if(!range(style.fontSize,1,2000)||![style.red,style.green,style.blue].every(n=>range(n,0,1))||!range(style.tracking,-100,1000)||!range(style.leading,0,5000)||!["Left","Center","Right"].includes(style.alignment))return false;
  if(style.boxSize&&(!style.boxSize.every(n=>range(n,16,30_000))||style.boxSize[0]*style.boxSize[1]>100_000_000))return false;
  const check=(runs:(TextFontRun|TextColorRun)[]|null|undefined)=>{let end=0;if(runs?.length===0)return false;for(const r of runs??[]){if(!Number.isInteger(r.location)||!Number.isInteger(r.length)||r.length<1||r.location<end)return false;end=r.location+r.length;if("fontName"in r){if(typeof r.fontName!=="string"||!r.fontName||r.fontName.length>200||/[\r\n]/.test(r.fontName))return false;}else if(![r.red,r.green,r.blue].every(n=>typeof n==="number"&&range(n,0,1)))return false;}return end<=style.content.length;};
  return check(style.fontRuns)&&check(style.colorRuns);
}
