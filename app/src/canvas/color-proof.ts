export interface ColorProfile {path:string;name:string}
export interface ColorProof {profile:ColorProfile|null;lut:Uint8Array|null;proof:boolean;warning:boolean;revision:number}
export const LUT_SIDE=33;
export function validateProofLut(values:number[]):Uint8Array {
  if(values.length!==LUT_SIDE**3*3||values.some(v=>!Number.isInteger(v)||v<0||v>255))throw new Error("Invalid ICC proof lookup");
  return new Uint8Array(values);
}
/** Display-only trilinear 33³ ICC lookup, over straight RGB with original alpha. */
export function applyColorProof(pixels:Uint8Array,settings:ColorProof):void {
  const lut=settings.lut;if(!lut||(!settings.proof&&!settings.warning))return;
  for(let i=0;i<pixels.length;i+=4){
    const a=pixels[i+3];if(a===0)continue;
    const rgb=[pixels[i],pixels[i+1],pixels[i+2]].map(c=>Math.min(255,c*255/a));
    const pos=rgb.map(c=>c/255*32),lo=pos.map(v=>Math.min(31,Math.floor(v))),f=pos.map((v,c)=>v-lo[c]);
    const output=[0,0,0];let warning=0;
    for(let z=0;z<2;z++)for(let y=0;y<2;y++)for(let x=0;x<2;x++){
      const r=lo[0]+x,g=lo[1]+y,b=lo[2]+z,index=((b*33+g)*33+r)*3;
      const weight=(x?f[0]:1-f[0])*(y?f[1]:1-f[1])*(z?f[2]:1-f[2]);
      for(let c=0;c<3;c++)output[c]+=lut[index+c]*weight;
      if(settings.warning&&Math.max(Math.abs(lut[index]-Math.round(r*255/32)),Math.abs(lut[index+1]-Math.round(g*255/32)),Math.abs(lut[index+2]-Math.round(b*255/32)))>10)warning+=weight;
    }
    for(let c=0;c<3;c++)pixels[i+c]=Math.round(((settings.proof?output[c]:rgb[c])*(1-warning)+127.5*warning)*a/255);
  }
}
export async function proofPng(bytes:Uint8Array,settings:ColorProof):Promise<Uint8Array>{
  const image=await createImageBitmap(new Blob([bytes.slice().buffer],{type:"image/png"}));
  try{
    const canvas=document.createElement("canvas");canvas.width=image.width;canvas.height=image.height;const ctx=canvas.getContext("2d")!;ctx.drawImage(image,0,0);
    const data=ctx.getImageData(0,0,image.width,image.height),pixels=new Uint8Array(data.data);
    for(let i=0;i<pixels.length;i+=4)for(let c=0;c<3;c++)pixels[i+c]=Math.round(pixels[i+c]*pixels[i+3]/255);
    applyColorProof(pixels,{...settings,proof:true,warning:false});
    for(let i=0;i<pixels.length;i+=4)for(let c=0;c<3;c++)pixels[i+c]=pixels[i+3]?Math.round(pixels[i+c]*255/pixels[i+3]):0;
    data.data.set(pixels);ctx.putImageData(data,0,0);
    const blob=await new Promise<Blob>((resolve,reject)=>canvas.toBlob(b=>b?resolve(b):reject(new Error("Cannot render ICC preview")),"image/png"));return new Uint8Array(await blob.arrayBuffer());
  }finally{image.close();}
}
