import {expect,it} from "vitest";
import {encodePalettePixels,validatePalettePixels,validatePaletteRuns} from "../../src/engine/palette-pixels";

// Check the complete typed-array payload without generic equality allocating
// an Object.entries pair for every byte. Keep the original inputs and timeout.
function expectBytes(actual:Uint8Array,expected:Uint8Array):void {
  expect(actual).toBeInstanceOf(Uint8Array);
  expect(actual.byteLength).toBe(expected.byteLength);
  expect(actual.every((value,index)=>value===expected[index])).toBe(true);
}

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
  for(const colors of [1,2,255,256,257,1024])for(const offset of [0,1])expectBytes(decoded(image(colors,offset)),image(colors,offset));
  expect(encodePalettePixels(image(256))!.indices.byteLength).toBe(256*3);
  expect(encodePalettePixels(image(257))!.indices.byteLength).toBe(257*3*2);
});
it("keeps source storage unchanged and refuses unbounded, empty or incomplete results",()=>{
  const source=image(1025),before=source.slice();
  expect(encodePalettePixels(source)).toBeNull();expectBytes(source,before);
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
    expectBytes(changing,before);
    // Palette support remains available to smaller callers and explicit tests.
    expectBytes(decoded(changing),changing);
    const bands=new Uint8Array(new ArrayBuffer(changing.length*64+1),1);
    for(let pixel=0;pixel<changing.length/4;pixel++)for(let repeat=0;repeat<64;repeat++)
      bands.set(changing.subarray(pixel*4,pixel*4+4),(pixel*64+repeat)*4);
    expectBytes(decoded(bands,64),bands);
  }
});

it("round-trips long flat runs without a per-pixel index plane, including unaligned translucent input",()=>{
  for(const colors of [2,300]){
    const count=colors*4101+13,source=new Uint8Array(new ArrayBuffer(count*4+1),1);
    for(let i=0;i<count;i++){const n=Math.min(colors-1,Math.floor(i/4101));source.set([n%90,Math.floor(n/90),17,97],i*4);}
    const before=source.slice(),encoded=encodePalettePixels(source,64)!;
    expect(encoded.runs).toBe(true);expect(encoded.indices.byteLength).toBe(colors*8);
    const {bytes,records}=validatePaletteRuns(encoded.palette,encoded.indices,count),out=new Uint8Array(source.length);
    let at=0;
    for(let r=0;r<records.byteLength;r+=8){const index=records.getUint32(r,true),length=records.getUint32(r+4,true),color=bytes.subarray(index*4,index*4+4);for(let i=0;i<length;i++)out.set(color,(at+i)*4);at+=length;}
    expect(at).toBe(count);expect(out.every((value,i)=>value===before[i])).toBe(true);expect(source.every((value,i)=>value===before[i])).toBe(true);
  }
});

it("rejects invalid run indices, zero counts, truncated records and incomplete or overflowing output sizes",()=>{
  const palette=[17,31,43,97,0,0,0,0];
  const records=(pairs:number[][])=>{const b=new ArrayBuffer(pairs.length*8),v=new DataView(b);pairs.forEach(([index,length],i)=>{v.setUint32(i*8,index,true);v.setUint32(i*8+4,length,true);});return b;};
  expect(validatePaletteRuns(palette,records([[0,7],[1,3]]),10).records.byteLength).toBe(16);
  for(const b of [null,new ArrayBuffer(0),new ArrayBuffer(7),records([[0,9]]),records([[0,11]]),records([[0,10],[1,0]]),records([[0,7],[2,3]]),records([[0,7],[1,0xffffffff]])])
    expect(()=>validatePaletteRuns(palette,b,10)).toThrow("Invalid palette job runs");
  for(const count of [0,-1,.5,100_000_001])expect(()=>validatePaletteRuns(palette,records([[0,10]]),count)).toThrow();
});
