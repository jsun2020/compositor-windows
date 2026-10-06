//! Bounded RGB histogram and HSB vectorscope of the grade, without diagnostic paint.
use crate::{Raster,CommandError};
use super::camera_raw::CameraRawSettings;
use serde::Serialize;
#[derive(Serialize)]
pub struct CameraRawScope {pub red:Vec<f64>,pub green:Vec<f64>,pub blue:Vec<f64>,pub vectorscope:Vec<f64>}
pub fn graded_scope(source:&Raster,settings:&CameraRawSettings)->Result<CameraRawScope,CommandError>{
    // Sample directly from storage: do not copy/halve a 100 MP layer on the UI thread.
    let step=source.width.max(source.height).div_ceil(512).max(1);
    let (w,h)=(source.width.div_ceil(step),source.height.div_ceil(step));
    let mut bytes=Vec::with_capacity(w as usize*h as usize*4);
    for y in (0..source.height).step_by(step as usize){for x in (0..source.width).step_by(step as usize){bytes.extend(source.pixel(x,y));}}
    let sampled=Raster::from_premultiplied(w,h,bytes);let mut scaled=settings.normalized();scaled.pixel_scale/=step as f64;
    let grade=scaled.apply(&sampled)?;Ok(make(&grade))
}
pub fn make(raster:&Raster)->CameraRawScope{
    let mut out=CameraRawScope{red:vec![0.;256],green:vec![0.;256],blue:vec![0.;256],vectorscope:vec![0.;4096]};
    for p in raster.bytes().chunks_exact(4){let alpha=p[3] as f64;if alpha==0.{continue;}let rgb=[p[0] as f64/alpha,p[1] as f64/alpha,p[2] as f64/alpha];let weight=alpha/255.;
        for (c,bins) in [&mut out.red,&mut out.green,&mut out.blue].into_iter().enumerate(){bins[(rgb[c]*255.).round().clamp(0.,255.) as usize]+=weight;}
        let max=rgb.into_iter().fold(0f64,f64::max);let min=rgb.into_iter().fold(1f64,f64::min);let chroma=max-min;if chroma<=1e-4||max<=1e-4{continue;}
        let hue=if max==rgb[0]{(rgb[1]-rgb[2])/chroma}else if max==rgb[1]{2.+(rgb[2]-rgb[0])/chroma}else{4.+(rgb[0]-rgb[1])/chroma};let angle=hue.rem_euclid(6.)/6.*std::f64::consts::TAU;let saturation=chroma/max;
        let x=((0.5+angle.cos()*saturation*0.48)*64.).clamp(0.,63.) as usize;let y=((0.5+angle.sin()*saturation*0.48)*64.).clamp(0.,63.) as usize;out.vectorscope[y*64+x]+=weight;
    }out
}
#[cfg(test)]mod tests{use super::*;
    #[test]fn transparent_pixels_are_skipped_and_partial_alpha_weights_the_same_grade(){let raster=Raster::from_straight(3,1,&[255,0,0,128,0,255,0,255,0,0,255,0]);let scope=make(&raster);assert!((scope.red[255]-128./255.).abs()<1e-9);assert_eq!(scope.green[255],1.);assert!((scope.vectorscope.iter().sum::<f64>()-(1.+128./255.)).abs()<1e-9);let neutral=graded_scope(&raster,&CameraRawSettings::default()).unwrap();assert_eq!(neutral.red,scope.red);}
}
