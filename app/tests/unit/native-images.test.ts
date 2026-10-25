import {describe,it,expect} from "vitest";
import {parseNativeImages} from "../../src/shell/native-images";
function response(header:unknown,data:number[]){const json=new TextEncoder().encode(JSON.stringify(header)),bytes=new Uint8Array(8+json.length+data.length);bytes.set([78,73,77,49]);new DataView(bytes.buffer).setUint32(4,json.length,true);bytes.set(json,8);bytes.set(data,8+json.length);return bytes.buffer;}
describe("native image transfer",()=>{
 it("retains page order and exact encoded bytes",()=>{const decoded=parseNativeImages(response({resolution:300,images:[{name:"Page 1",length:3},{name:"Page 2",length:2}]},[1,2,3,4,5]));expect(decoded.resolution).toBe(300);expect(decoded.images.map(p=>[p.name,Array.from(p.bytes)])).toEqual([["Page 1",[1,2,3]],["Page 2",[4,5]]]);});
 it("refuses truncation, excess data, invalid lengths and empty page lists",()=>{
  for(const n of [-1,0,2.5,8])expect(()=>parseNativeImages(response({resolution:300,images:[{name:"Page 1",length:n}]},[1,2,3]))).toThrow();
  expect(()=>parseNativeImages(response({resolution:300,images:[]},[]))).toThrow();
  expect(()=>parseNativeImages(response({resolution:0,images:[{name:"Page",length:3}]},[1,2,3]))).toThrow();
  expect(()=>parseNativeImages(response({resolution:300,images:[{name:"Page",length:2}]},[1,2,3]))).toThrow();
  expect(()=>parseNativeImages(new ArrayBuffer(5))).toThrow();
 });
});
