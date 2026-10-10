import {expect,it} from "vitest";
import {encodePalettePixels,validatePalettePixels} from "../../src/engine/palette-pixels";

function image(colors:number, offset=0):Uint8Array {
  const out=new Uint8Array(new ArrayBuffer(colors*4*3+offset),offset);
  for(let i=0;i<colors*3;i++){const n=i%colors;out.set([n%256,Math.floor(n/256),17,(n*71)%256],i*4);}
  return out;
}
function decoded(source:Uint8Array, minimumRunPixels=1):Uint8Array {
  const encoded=encodePalettePixels(source,minimumRunPixels)!;
  const {bytes,depth}=validatePalettePixels(encoded.palette,encoded.indices,source.length/4);
  const input=new DataView(encoded.indices),out=new Uint8Array(source.length);
  for(let i=0;i<out.length/4;i++){const n=depth===1?input.getUint8(i):input.getUint16(i*2,true);out.set(bytes.subarray(n*4,n*4+4),i*4);}
  return out;
}

it("round-trips every RGBA byte across the 256-color boundary and bounded 1024-color palette",()=>{
  for(const colors of [1,2,255,256,257,1024])for(const offset of [0,1])expect(decoded(image(colors,offset))).toEqual(image(colors,offset));
  expect(encodePalettePixels(image(256))!.indices.byteLength).toBe(256*3);
  expect(encodePalettePixels(image(257))!.indices.byteLength).toBe(257*3*2);
});
it("keeps source storage unchanged and refuses unbounded, empty or incomplete results",()=>{
  const source=image(1025),before=source.slice();
  expect(encodePalettePixels(source)).toBeNull();expect(source).toEqual(before);
  expect(encodePalettePixels(new Uint8Array())).toBeNull();expect(encodePalettePixels(new Uint8Array(3))).toBeNull();
});
it("rejects malformed palettes and wrong index plane lengths before staging",()=>{
  for(const palette of [[],[0,0,0],[0,0,0,256],[0,0,0,.5],new Array(4100).fill(0),"bad"])
    expect(()=>validatePalettePixels(palette,new ArrayBuffer(1),1)).toThrow("Invalid palette");
  expect(()=>validatePalettePixels([0,0,0,0],null,1)).toThrow();
  expect(()=>validatePalettePixels([0,0,0,0],new ArrayBuffer(2),1)).toThrow();
  expect(()=>validatePalettePixels(new Array(257*4).fill(0),new ArrayBuffer(1),1)).toThrow();
});
it("selects raw bytes for short bands while preserving both depths of long-band palettes",()=>{
  for(const colors of [256,300]) {
    const changing=image(colors,1),before=changing.slice();
    expect(encodePalettePixels(changing,64)).toBeNull();
    expect(changing).toEqual(before);
    // Palette support remains available to smaller callers and explicit tests.
    expect(decoded(changing)).toEqual(changing);
    const bands=new Uint8Array(new ArrayBuffer(changing.length*64+1),1);
    for(let pixel=0;pixel<changing.length/4;pixel++)for(let repeat=0;repeat<64;repeat++)
      bands.set(changing.subarray(pixel*4,pixel*4+4),(pixel*64+repeat)*4);
    expect(decoded(bands,64)).toEqual(bands);
  }
});
