import type { FilterLayerSettings, FilterParams, LayerAdjustment } from "../engine/types";
export function defaultFilterLayerSettings():FilterLayerSettings {return {progressiveFrom:0,progressiveTo:20,progressiveRadial:false,progressiveInverted:false,bloomAmount:40,bloomRadius:24,rgbSplitDistance:10,rgbSplitBlur:0,rgbSplitAngle:0,rgbSplitRadial:false,rgbSplitReach:1000};}
export function filterLayerParams(a:LayerAdjustment|null|undefined):FilterParams|null {
 if(!a)return null;const s={...defaultFilterLayerSettings(),...a.filterLayerSettings};
 if(a.kind==="Progressive Blur")return {filter:"ProgressiveBlur",settings:{from:s.progressiveFrom,to:s.progressiveTo,start:s.progressiveStart??null,end:s.progressiveEnd??null,radial:s.progressiveRadial,inverted:s.progressiveInverted}};
 if(a.kind==="Bloom / Glow")return {filter:"BloomGlow",amount:s.bloomAmount,radius:s.bloomRadius};
 if(a.kind==="RGB Split")return {filter:"RgbSplit",settings:{distance:s.rgbSplitDistance,blur:s.rgbSplitBlur,angle:s.rgbSplitAngle,radial:s.rgbSplitRadial,center:s.rgbSplitCenter??null,reach:s.rgbSplitReach}};
 return null;
}
export function withFilterLayerParams(a:LayerAdjustment,p:FilterParams):LayerAdjustment {
 const s={...defaultFilterLayerSettings(),...a.filterLayerSettings};
 if(p.filter==="ProgressiveBlur"){s.progressiveFrom=p.settings.from;s.progressiveTo=p.settings.to;s.progressiveRadial=p.settings.radial;s.progressiveInverted=p.settings.inverted;s.progressiveStart=p.settings.start??undefined;s.progressiveEnd=p.settings.end??undefined;}
 else if(p.filter==="BloomGlow"){s.bloomAmount=p.amount;s.bloomRadius=p.radius;}
 else if(p.filter==="RgbSplit"){s.rgbSplitDistance=p.settings.distance;s.rgbSplitBlur=p.settings.blur;s.rgbSplitAngle=p.settings.angle;s.rgbSplitRadial=p.settings.radial;s.rgbSplitCenter=p.settings.center??undefined;s.rgbSplitReach=p.settings.reach;}
 else return a;
 return {...a,filterLayerSettings:s};
}
