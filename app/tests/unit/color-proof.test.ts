import {describe,it,expect} from "vitest";
import {applyColorProof,validateProofLut,type ColorProof} from "../../src/canvas/color-proof";
function settings(change=false):ColorProof {
  const data:number[]=[];for(let b=0;b<33;b++)for(let g=0;g<33;g++)for(let r=0;r<33;r++)data.push(Math.round(r*255/32),Math.round(g*255/32),Math.round(b*255/32));
  if(change)for(let i=0;i<data.length;i+=3)data[i]=0;
  return {profile:{name:"Test",path:"fixture.icc"},lut:validateProofLut(data),proof:true,warning:false,revision:1};
}
describe("ICC display proof",()=>{
  it("interpolates the grid and preserves alpha without changing unproofed data",()=>{
    const original=new Uint8Array([24,58,120,128,255,255,255,255,0,0,0,0]),pixels=original.slice();applyColorProof(pixels,settings());for(let i=0;i<pixels.length;i++)expect(Math.abs(pixels[i]-original[i])).toBeLessThanOrEqual(1);
    const changed=original.slice();applyColorProof(changed,settings(true));expect(changed[0]).toBe(0);expect(changed[4]).toBe(0);expect(changed[3]).toBe(128);expect(changed[7]).toBe(255);
    const off=original.slice();applyColorProof(off,{...settings(true),proof:false});expect(off).toEqual(original);
  });
  it("marks out-of-gamut colors with gray and preserves transparency",()=>{
    const p=new Uint8Array([255,20,40,255,64,20,30,128,0,0,0,0]);applyColorProof(p,{...settings(true),proof:false,warning:true});expect([...p]).toEqual([128,128,128,255,64,64,64,128,0,0,0,0]);
  });
  it("rejects a truncated or non-byte native lookup",()=>{expect(()=>validateProofLut([1,2,3])).toThrow();const bytes=[...settings().lut!];bytes[30]=NaN;expect(()=>validateProofLut(bytes)).toThrow();});
});
