//! Versions 16–20 filter-layer settings, using the Mac manifest's flat field names.
use crate::{AdjustmentKind,LayerAdjustment,LayerTransform,Point,Raster,Rect,Size};
use super::{filters::FilterParams,progressive::ProgressiveBlurSettings,rgb_split::RgbSplitSettings};
use serde::{Deserialize,Serialize};
use super::finishing::{VignetteLayerSettings,TonalLayerSettings,SharpenLayerSettings};
use super::lens_effects::LensEffectSettings;
#[derive(Clone,Debug,PartialEq,Serialize,Deserialize)]
#[serde(default,rename_all="camelCase")]
pub struct FilterLayerSettings {
 #[serde(skip_serializing_if="Option::is_none")]pub vignette:Option<VignetteLayerSettings>,
 #[serde(skip_serializing_if="Option::is_none")]pub tonal:Option<TonalLayerSettings>,
 #[serde(skip_serializing_if="Option::is_none")]pub lens:Option<LensEffectSettings>,
 #[serde(skip_serializing_if="Option::is_none")]pub sharpen:Option<SharpenLayerSettings>,
 pub progressive_from:f64,pub progressive_to:f64,
 #[serde(skip_serializing_if="Option::is_none")]pub progressive_start:Option<Point>,
 #[serde(skip_serializing_if="Option::is_none")]pub progressive_end:Option<Point>,
 pub progressive_radial:bool,pub progressive_inverted:bool,
 pub bloom_amount:f64,pub bloom_radius:f64,
 pub rgb_split_distance:f64,pub rgb_split_blur:f64,pub rgb_split_angle:f64,pub rgb_split_radial:bool,
 #[serde(skip_serializing_if="Option::is_none")]pub rgb_split_center:Option<Point>,pub rgb_split_reach:f64,
}
impl Default for FilterLayerSettings {fn default()->Self{Self{
 vignette:None,tonal:None,lens:None,sharpen:None,
 progressive_from:0.,progressive_to:20.,progressive_start:None,progressive_end:None,progressive_radial:false,progressive_inverted:false,
 bloom_amount:40.,bloom_radius:24.,rgb_split_distance:10.,rgb_split_blur:0.,rgb_split_angle:0.,rgb_split_radial:false,rgb_split_center:None,rgb_split_reach:1000.,
}}}
impl FilterLayerSettings {
 pub fn params(&self,kind:AdjustmentKind)->Option<FilterParams>{Some(match kind {
  AdjustmentKind::ProgressiveBlur=>FilterParams::ProgressiveBlur{settings:ProgressiveBlurSettings{from:self.progressive_from,to:self.progressive_to,start:self.progressive_start,end:self.progressive_end,radial:self.progressive_radial,inverted:self.progressive_inverted}},
  AdjustmentKind::BloomGlow=>FilterParams::BloomGlow{amount:self.bloom_amount,radius:self.bloom_radius},
  AdjustmentKind::RgbSplit=>FilterParams::RgbSplit{settings:RgbSplitSettings{distance:self.rgb_split_distance,blur:self.rgb_split_blur,angle:self.rgb_split_angle,radial:self.rgb_split_radial,center:self.rgb_split_center,reach:self.rgb_split_reach}},
  AdjustmentKind::Vignette=>FilterParams::Vignette{settings:self.vignette.clone().unwrap_or_default()},
  AdjustmentKind::TonalContrast=>FilterParams::TonalContrast{settings:self.tonal.clone().unwrap_or_default()},
  AdjustmentKind::LensEffects=>FilterParams::LensEffects{settings:self.lens.clone().unwrap_or_default()},
  AdjustmentKind::Sharpen=>{let s=self.sharpen.clone().unwrap_or_default();FilterParams::Sharpen{amount:s.amount,radius:s.radius}},
  _=>return None,
 })}
 pub fn is_valid(&self)->bool {
  [AdjustmentKind::ProgressiveBlur,AdjustmentKind::BloomGlow,AdjustmentKind::RgbSplit,AdjustmentKind::Vignette,AdjustmentKind::TonalContrast,AdjustmentKind::LensEffects,AdjustmentKind::Sharpen].into_iter().all(|k|{let p=self.params(k).unwrap();p==p.normalized()})
  && [self.progressive_start,self.progressive_end,self.rgb_split_center].into_iter().flatten().all(|p|p.x.is_finite()&&p.y.is_finite())
 }
}
impl LayerAdjustment {
 pub fn minimum_version(&self)->u32 {
  let mut version=self.kind.minimum_version();
  if let Some(f)=&self.filter_layer_settings {
   version=version.max(16);
   if f.vignette.is_some()||f.tonal.is_some()||f.lens.is_some(){version=version.max(18);}
   if f.sharpen.is_some(){version=version.max(19);}
   if f.lens.as_ref().is_some_and(|s|s.prism_fringe.is_some()){version=20;}
  }version
 }
 pub fn filter_layer(&self)->FilterLayerSettings {self.filter_layer_settings.clone().unwrap_or_default()}
 pub fn filter_params(&self)->Option<FilterParams>{self.filter_layer().params(self.kind)}
 /// The raster is an already composited region; no layer growth here. Its caller
 /// pads the render by sampling_margin before asking for a partial view.
 pub fn apply_spatial_filter(&self,r:&Raster,region:Rect)->Raster {
  self.apply_spatial_filter_on_canvas(r,region,Size{width:region.width,height:region.height})
 }
 pub fn apply_spatial_filter_on_canvas(&self,r:&Raster,region:Rect,canvas:Size)->Raster {
  let Some(p)=self.filter_params()else{return r.clone();};
  let transform=LayerTransform{origin:Point{x:region.x,y:region.y},size:Size{width:region.width,height:region.height},rotation:0.,flip_x:false,flip_y:false,sampling:crate::Sampling::High};
  let p=p.normalized().scaled(r.width as f64/region.width).mapped(&transform,r.width,r.height);
  if let FilterParams::RgbSplit{settings}=&p {
      let scale=r.width as f64/region.width;
      return settings.apply_pixels_at(r,Point{x:region.x*scale,y:region.y*scale});
  }
  if let FilterParams::Vignette{settings}=&p {
      let sx=r.width as f64/region.width;let sy=r.height as f64/region.height;
      return super::finishing::vignette(r,settings,Rect{x:-region.x*sx,y:-region.y*sy,width:canvas.width*sx,height:canvas.height*sy},false);
  }
  if let FilterParams::LensEffects{settings}=&p {
      return super::lens_effects::apply_with_side(r,settings,canvas.width.max(canvas.height)*r.width as f64/region.width);
  }
  // Validated settings are shared with the destructive filter implementation.
  super::filters::apply_filter_pixels(r,&p).expect("validated spatial filter")
 }
}
