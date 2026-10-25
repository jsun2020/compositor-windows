import type {PackageFiles} from "../engine/types";
import type {ColorProfile} from "../canvas/color-proof";
export interface NativeCmyk {files:PackageFiles;metadata:string;profile:ColorProfile}
export function parseNativeCmyk(buffer:ArrayBuffer):NativeCmyk|null {
  const bytes=new Uint8Array(buffer);if(!bytes.length)return null;
  if(bytes.length<8||bytes.length>512*1024*1024||new TextDecoder().decode(bytes.subarray(0,4))!=="NCP1")throw new Error("Invalid color-managed image response");
  const length=new DataView(buffer).getUint32(4,true);if(length>8*1024*1024||length>bytes.length-8)throw new Error("Invalid color-managed image header");
  const header=JSON.parse(new TextDecoder().decode(bytes.subarray(8,8+length)));
  if(typeof header.manifest!=="string"||header.manifest.length>4*1024*1024||!Array.isArray(header.images)||header.images.length>20000||!Array.isArray(header.artboards)||!Array.isArray(header.conversions)||typeof header.profile?.path!=="string"||typeof header.profile?.name!=="string")throw new Error("Invalid color-managed image metadata");
  let offset=8+length;const images=[];
  for(const image of header.images){if(typeof image.name!=="string"||image.name.length>255||!Number.isSafeInteger(image.length)||image.length<1||image.length>bytes.length-offset)throw new Error("Invalid color-managed image asset");images.push({name:image.name,bytes:bytes.subarray(offset,offset+image.length)});offset+=image.length;}
  if(offset!==bytes.length)throw new Error("Unexpected trailing color-managed image bytes");
  return {files:{manifest:header.manifest,images},metadata:JSON.stringify({artboards:header.artboards,conversions:header.conversions}),profile:header.profile};
}
