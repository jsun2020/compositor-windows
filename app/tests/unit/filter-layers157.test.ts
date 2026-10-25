import {describe,it,expect} from "vitest";
import {defaultAdjustment,defaultFilterParams,resetAdjustment,isAdjustIdentity} from "../../src/state/adjust-edit";
import {filterLayerParams,withFilterLayerParams,defaultFilterLayerSettings} from "../../src/state/filter-layers";
describe("version 16 filter layers",()=>{
 it("edits only the chosen filter and preserves other settings and document handles",()=>{
  const a={...defaultAdjustment("Progressive Blur"),filterLayerSettings:{...defaultFilterLayerSettings(),bloomAmount:72,progressiveStart:[10,20] as [number,number],progressiveEnd:[100,200] as [number,number]}};
  const p=filterLayerParams(a)!;if(p.filter!=="ProgressiveBlur")throw Error("kind");p.settings.to=75;p.settings.inverted=true;
  const changed=withFilterLayerParams(a,p);expect(changed.filterLayerSettings?.bloomAmount).toBe(72);expect(changed.filterLayerSettings?.progressiveStart).toEqual([10,20]);expect(changed.filterLayerSettings?.progressiveTo).toBe(75);expect(a.filterLayerSettings.progressiveTo).toBe(20);
  const reset=resetAdjustment(changed,a);expect(reset.filterLayerSettings?.progressiveTo).toBe(20);expect(reset.filterLayerSettings?.progressiveStart).toEqual([10,20]);expect(reset.filterLayerSettings?.bloomAmount).toBe(72);
 });
 it("absent optional settings survive an untouched reset",()=>{
  for(const kind of ["Progressive Blur","Bloom / Glow","RGB Split"]as const){const a=defaultAdjustment(kind);expect(resetAdjustment(a,a)).toEqual(a);expect(filterLayerParams(a)).not.toBeNull();}
 });
 it("zero-strength destructive filters are identities",()=>{
  const test=(params:ReturnType<typeof defaultFilterParams>)=>isAdjustIdentity({params,adjustment:null,original:null},()=>false);
  expect(test({filter:"BloomGlow",amount:0,radius:24})).toBe(true);expect(test({filter:"RgbSplit",settings:{distance:0,blur:0,angle:0,radial:false,center:null,reach:1000}})).toBe(true);
  expect(test({filter:"ProgressiveBlur",settings:{from:0,to:0,start:null,end:null,radial:false,inverted:false}})).toBe(true);
  expect(test(defaultFilterParams("ProgressiveBlur"))).toBe(false);
 });
});
