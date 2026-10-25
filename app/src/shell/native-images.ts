export interface NativeImages {resolution:number;images:{name:string;bytes:Uint8Array}[]}
export function parseNativeImages(buffer:ArrayBuffer):NativeImages {
  const bytes=new Uint8Array(buffer),view=new DataView(buffer);
  if(bytes.length<8||bytes.length>512*1024*1024||String.fromCharCode(...bytes.subarray(0,4))!=="NIM1")throw new Error("Invalid native image response");
  const length=view.getUint32(4,true);if(length>1024*1024||length>bytes.length-8)throw new Error("Invalid native image header");
  const header=JSON.parse(new TextDecoder().decode(bytes.subarray(8,8+length))) as {resolution:number;images:{name:string;length:number}[]};
  if(!Number.isFinite(header.resolution)||header.resolution<1||header.resolution>9600||!Array.isArray(header.images)||!header.images.length||header.images.length>10000)throw new Error("Invalid native image metadata");
  let offset=8+length;const images=[];
  for(const image of header.images){
    if(typeof image.name!=="string"||!image.name.trim()||image.name.length>16384||!Number.isSafeInteger(image.length)||image.length<1||image.length>bytes.length-offset)throw new Error("Invalid native image page");
    images.push({name:image.name,bytes:bytes.subarray(offset,offset+image.length)});offset+=image.length;
  }
  if(offset!==bytes.length)throw new Error("Unexpected trailing native image bytes");
  return {resolution:header.resolution,images};
}
