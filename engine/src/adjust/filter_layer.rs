//! Version 16 filter-layer settings, using the Mac manifest's flat field names.
use crate::{AdjustmentKind,LayerAdjustment,LayerTransform,Point,Raster,Rect,Size};
use super::{filters::FilterParams,progressive::ProgressiveBlurSettings,rgb_split::RgbSplitSettings};
use serde::{Deserialize,Serialize};
#[derive(Clone,Debug,PartialEq,Serialize,Deserialize)]
#[serde(default,rename_all="camelCase")]
pub struct FilterLayerSettings {
 pub progressive_from:f64,pub progressive_to:f64,
 #[serde(skip_serializing_if="Option::is_none")]pub progressive_start:Option<Point>,
 #[serde(skip_serializing_if="Option::is_none")]pub progressive_end:Option<Point>,
 pub progressive_radial:bool,pub progressive_inverted:bool,
 pub bloom_amount:f64,pub bloom_radius:f64,
 pub rgb_split_distance:f64,pub rgb_split_blur:f64,pub rgb_split_angle:f64,pub rgb_split_radial:bool,
 #[serde(skip_serializing_if="Option::is_none")]pub rgb_split_center:Option<Point>,pub rgb_split_reach:f64,
}
impl Default for FilterLayerSettings {fn default()->Self{Self{
 progressive_from:0.,progressive_to:20.,progressive_start:None,progressive_end:None,progressive_radial:false,progressive_inverted:false,
 bloom_amount:40.,bloom_radius:24.,rgb_split_distance:10.,rgb_split_blur:0.,rgb_split_angle:0.,rgb_split_radial:false,rgb_split_center:None,rgb_split_reach:1000.,
}}}
impl FilterLayerSettings {
 pub fn params(&self,kind:AdjustmentKind)->Option<FilterParams>{Some(match kind {
  AdjustmentKind::ProgressiveBlur=>FilterParams::ProgressiveBlur{settings:ProgressiveBlurSettings{from:self.progressive_from,to:self.progressive_to,start:self.progressive_start,end:self.progressive_end,radial:self.progressive_radial,inverted:self.progressive_inverted}},
  AdjustmentKind::BloomGlow=>FilterParams::BloomGlow{amount:self.bloom_amount,radius:self.bloom_radius},
  AdjustmentKind::RgbSplit=>FilterParams::RgbSplit{settings:RgbSplitSettings{distance:self.rgb_split_distance,blur:self.rgb_split_blur,angle:self.rgb_split_angle,radial:self.rgb_split_radial,center:self.rgb_split_center,reach:self.rgb_split_reach}},
  _=>return None,
 })}
 pub fn is_valid(&self)->bool {
  [AdjustmentKind::ProgressiveBlur,AdjustmentKind::BloomGlow,AdjustmentKind::RgbSplit].into_iter().all(|k|{let p=self.params(k).unwrap();p==p.normalized()})
  && [self.progressive_start,self.progressive_end,self.rgb_split_center].into_iter().flatten().all(|p|p.x.is_finite()&&p.y.is_finite())
 }
}
impl LayerAdjustment {
 pub fn filter_layer(&self)->FilterLayerSettings {self.filter_layer_settings.clone().unwrap_or_default()}
 pub fn filter_params(&self)->Option<FilterParams>{self.filter_layer().params(self.kind)}
 /// The raster is an already composited region; no layer growth here. Its caller
 /// pads the render by sampling_margin before asking for a partial view.
 pub fn apply_spatial_filter(&self,r:&Raster,region:Rect)->Raster {
  let Some(p)=self.filter_params()else{return r.clone();};
  let transform=LayerTransform{origin:Point{x:region.x,y:region.y},size:Size{width:region.width,height:region.height},rotation:0.,flip_x:false,flip_y:false,sampling:crate::Sampling::High};
  let p=p.normalized().scaled(r.width as f64/region.width).mapped(&transform,r.width,r.height);
  if let FilterParams::RgbSplit{settings}=&p {
      let scale=r.width as f64/region.width;
      return settings.apply_pixels_at(r,Point{x:region.x*scale,y:region.y*scale});
  }
  // These three kernels have no recoverable external failure.
  super::filters::apply_filter_pixels(r,&p).expect("validated spatial filter")
 }
}
