//! RGB Split from Compositor v1.5.7 MetalDispersion.swift. A spectral gather, with
//! symmetric smear taps, rather than three independently displaced RGB images.
use crate::{Raster,Point};
use serde::{Serialize,Deserialize};
#[derive(Clone,Debug,PartialEq,Serialize,Deserialize)]
#[serde(rename_all="camelCase",default)]
pub struct RgbSplitSettings {pub distance:f64,pub blur:f64,pub angle:f64,pub radial:bool,pub center:Option<Point>,pub reach:f64}
impl Default for RgbSplitSettings {fn default()->Self{Self{distance:10.,blur:0.,angle:0.,radial:false,center:None,reach:1000.}}}
impl RgbSplitSettings {
    pub fn normalized(&self)->Self {
        let clamp=|v:f64,lo,hi,default|if v.is_finite(){v.clamp(lo,hi)}else{default};
        Self{distance:clamp(self.distance,0.,200.,10.),blur:clamp(self.blur,0.,400.,0.),angle:clamp(self.angle,-180.,180.,0.),radial:self.radial,
            center:self.center.filter(|p|p.x.is_finite()&&p.y.is_finite()&&p.x.abs()<=1_000_000.&&p.y.abs()<=1_000_000.),reach:clamp(self.reach,1.,100_000.,1000.)}
    }
    pub fn apply(&self,r:&Raster)->Raster {self.normalized().apply_pixels_at(r,Point{x:0.,y:0.})}
    pub fn apply_at(&self,r:&Raster,origin:Point)->Raster {self.normalized().apply_pixels_at(r,origin)}
    pub(crate) fn apply_pixels_at(&self,r:&Raster,origin:Point)->Raster {
        if r.deep_pixels().is_some(){return r.map_deep::<std::convert::Infallible>(false,|p|Ok(self.apply_pixels_at(p,origin))).unwrap();}
        self.apply_8_at(r,None,origin)
    }
    fn apply_8(&self,r:&Raster,jitters:Option<&[f32]>)->Raster {self.normalized().apply_8_at(r,jitters,Point{x:0.,y:0.})}
    fn apply_8_at(&self,r:&Raster,jitters:Option<&[f32]>,origin:Point)->Raster {
        let s=self;if s.distance==0.&&s.blur==0.{return r.clone();}
        let center=s.center.unwrap_or(Point{x:r.width as f64/2.,y:r.height as f64/2.});
        let n=if s.distance>0.{((s.distance*if s.radial{2.}else{1.}) as u32).clamp(3,32)}else{1};
        let m=if s.blur>0.{((s.blur*if s.radial{2.}else{1.}/2.) as u32).clamp(2,24)}else{1};
        let angle=s.angle.to_radians();let direction=[angle.cos() as f32,-angle.sin() as f32];
        let smooth=|a:f32,b:f32,x:f32|{let t=((x-a)/(b-a)).clamp(0.,1.);t*t*(3.-2.*t)};
        let mut bytes=vec![0;r.bytes().len()];
        for y in 0..r.height {for x in 0..r.width {
            let at=[x as f32+0.5,y as f32+0.5];let axis=if s.radial{[(at[0]-center.x as f32)/s.reach as f32,(at[1]-center.y as f32)/s.reach as f32]}else{direction};
            let noise=(y as f32+origin.y as f32).mul_add(78.233,(x as f32+origin.x as f32)*12.9898).sin()*43758.5453;let jitter=jitters.map_or(noise-noise.floor(),|j|j[(y*r.width+x) as usize]);
            let(mut color,mut alpha,mut total)=([0f32;3],[0f32;3],[0f32;3]);
            for i in 0..n {let t=if n==1{0.}else{(i as f32+jitter)/n as f32*2.-1.};
                let weight=[smooth(-0.3,1.,t),1.-smooth(0.,1.,t.abs())+0.05,smooth(-0.3,1.,-t)];
                for j in 0..m {let u=if m==1{0.}else{(j as f32+jitter)/m as f32-0.5};
                    let q=[at[0]-axis[0]*(s.distance as f32*t+s.blur as f32*u),at[1]-axis[1]*(s.distance as f32*t+s.blur as f32*u)];
                    let p=if q[0]<0.||q[1]<0.||q[0]>r.width as f32||q[1]>r.height as f32{[0.;4]}else{crate::compositor::sample(r,q[0] as f64,q[1] as f64,false)};
                    for c in 0..3{color[c]+=weight[c]*p[c];alpha[c]+=weight[c]*p[3];total[c]+=weight[c];}
                }
            }
            let a=(0..3).map(|c|alpha[c]/total[c]).fold(0.,f32::max);let index=((y*r.width+x)*4) as usize;
            for c in 0..3 {bytes[index+c]=((color[c]/total[c]).min(a)*255.).round().clamp(0.,255.) as u8;}bytes[index+3]=(a*255.).round().clamp(0.,255.) as u8;
        }}
        Raster::from_premultiplied(r.width,r.height,bytes)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_metal_tap_positions_match_the_upstream_spectral_kernel() {
        let r=Raster::from_premultiplied(64,48,include_bytes!("../../tests/fixtures/mac157/rgb-input.rgba").to_vec());
        let jitter:Vec<_>=include_bytes!("../../tests/fixtures/mac157/rgb-jitter.f32").chunks_exact(4).map(|b|f32::from_le_bytes(b.try_into().unwrap())).collect();
        for(distance,blur,angle,radial,expected)in [
            (8.,0.,0.,false,include_bytes!("../../tests/fixtures/mac157/rgb-linear.rgba").as_slice()),
            (0.,24.,0.,false,include_bytes!("../../tests/fixtures/mac157/rgb-smear.rgba").as_slice()),
            (12.,8.,0.,true,include_bytes!("../../tests/fixtures/mac157/rgb-radial.rgba").as_slice()),
            (10.,12.,45.,false,include_bytes!("../../tests/fixtures/mac157/rgb-angled.rgba").as_slice())] {
            let s=RgbSplitSettings{distance,blur,angle,radial,center:Some(Point{x:32.,y:24.}),reach:32.};let result=s.apply_8(&r,Some(&jitter));
            let max=result.bytes().iter().zip(expected).map(|(a,b)|a.abs_diff(*b)).max().unwrap();
            assert!(max<=2,"Metal spectral taps differ by {max} levels");
        }
    }
}
