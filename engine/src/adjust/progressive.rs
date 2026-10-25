//! Variable-radius Gaussian blur. The radius field is in the layer's pixel grid;
//! document handles are mapped by FilterParams before entering this module.
use crate::{DeepPixels, PixelDepth, Point, Raster};
use super::{deep_filters, filters::CanvasSides};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all="camelCase")]
pub struct ProgressiveBlurSettings {
    pub from: f64, pub to: f64,
    pub start: Option<Point>, pub end: Option<Point>,
    pub radial: bool, pub inverted: bool,
}
impl Default for ProgressiveBlurSettings {
    fn default()->Self {Self{from:0.,to:20.,start:None,end:None,radial:false,inverted:false}}
}
impl ProgressiveBlurSettings {
    pub fn normalized(&self)->Self {
        let mut s=self.clone();
        s.from=if s.from.is_finite(){s.from.clamp(0.,250.)}else{0.};
        s.to=if s.to.is_finite(){s.to.clamp(0.,250.)}else{20.};
        s.start=s.start.filter(|p|p.x.is_finite()&&p.y.is_finite());
        s.end=s.end.filter(|p|p.x.is_finite()&&p.y.is_finite());s
    }
    fn radius(&self,x:f64,y:f64,w:u32,h:u32)->f64 {
        let a=self.start.unwrap_or(Point{x:w as f64/2.,y:0.});
        let mut b=self.end.unwrap_or(Point{x:w as f64/2.,y:h as f64});
        if (b.x-a.x).hypot(b.y-a.y)<1. {b=Point{x:a.x,y:a.y+1.};}
        let(dx,dy)=(b.x-a.x,b.y-a.y);let length=dx.hypot(dy);
        let t=if self.radial {(x-a.x).hypot(y-a.y)/length}else{((x-a.x)*dx+(y-a.y)*dy)/(length*length)}.clamp(0.,1.);
        let t=if self.inverted{1.-t}else{t};self.from+(self.to-self.from)*t
    }
    /// Adjacent Gaussian scales are interpolated in variance. Only two levels are
    /// live at once; above sigma 4 the source halves so kernel cost stays bounded.
    /// Core Image's masked-variable kernel is proprietary: this is the Windows
    /// Gaussian implementation, not a claim of bit-identical Core Image output.
    pub fn apply(&self,r:&Raster,sides:CanvasSides)->Raster {
        self.normalized().apply_pixels(r,sides)
    }
    pub(crate) fn apply_pixels(&self,r:&Raster,sides:CanvasSides)->Raster {
        let s=self;let maximum=s.from.max(s.to);
        if maximum==0. {return r.clone();}
        let mut radii=vec![0.,0.5];
        while *radii.last().unwrap()<maximum {radii.push(radii.last().unwrap()*std::f64::consts::SQRT_2);}
        let mut bytes=vec![0;if r.depth()==PixelDepth::Eight {r.width as usize*r.height as usize*4}else{0}];
        let mut samples=vec![0;if r.depth()==PixelDepth::Eight {0}else{r.width as usize*r.height as usize*4}];
        let mut previous=r.clone();let mut previous_scale=1.;let mut reduced=r.clone();let mut scale=1.;
        for pair in radii.windows(2) {
            let (lo,hi)=(pair[0],pair[1]);
            while hi/scale>4. && (reduced.width>1&&reduced.height>1) {reduced=reduced.reduced_deep(1);scale*=2.;}
            let next=deep_filters::gaussian_edges(&reduced,hi/scale,false,sides);
            for y in 0..r.height {for x in 0..r.width {
                let radius=s.radius(x as f64+0.5,y as f64+0.5,r.width,r.height);
                if radius>hi || (lo>0.&&radius<=lo) {continue;}
                let mix=((radius*radius-lo*lo)/(hi*hi-lo*lo)).clamp(0.,1.)as f32;
                let a=sample(&previous,(x as f64+0.5)/previous_scale,(y as f64+0.5)/previous_scale);
                let b=sample(&next,(x as f64+0.5)/scale,(y as f64+0.5)/scale);
                let p=std::array::from_fn(|c|a[c]*(1.-mix)+b[c]*mix);let at=(y as usize*r.width as usize+x as usize)*4;
                if r.depth()==PixelDepth::Eight {for c in 0..4 {bytes[at+c]=(p[c].clamp(0.,p[3])*255.).round()as u8;}}
                else {deep_filters::write(&mut samples[at..at+4],p,r.depth());}
            }}
            previous=next;previous_scale=scale;
        }
        drop(previous);drop(reduced);
        if r.depth()==PixelDepth::Eight {Raster::from_premultiplied(r.width,r.height,bytes)}else{Raster::from_deep(r.width,r.height,DeepPixels{depth:r.depth(),samples})}
    }
}
fn sample(r:&Raster,x:f64,y:f64)->[f32;4] {
    let (x,y)=(x-0.5,y-0.5);let(ix,iy)=(x.floor()as i64,y.floor()as i64);let(tx,ty)=((x-ix as f64)as f32,(y-iy as f64)as f32);
    let mut p=[0.;4];
    for(xx,yy,k)in [(ix,iy,(1.-tx)*(1.-ty)),(ix+1,iy,tx*(1.-ty)),(ix,iy+1,(1.-tx)*ty),(ix+1,iy+1,tx*ty)] {
        let q=r.pixel_f32(xx.clamp(0,r.width as i64-1)as u32,yy.clamp(0,r.height as i64-1)as u32);for c in 0..4{p[c]+=q[c]*k;}
    }p
}
