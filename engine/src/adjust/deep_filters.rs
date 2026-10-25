//! Spatial filters sample high precision channels directly; no RGBA8 intermediate.
use crate::{Raster,PixelDepth,DeepPixels};
use super::filters::{CanvasSides,edge_index};
pub(super) fn write(out:&mut [u16],p:[f32;4],depth:PixelDepth) {
    let alpha=p[3].clamp(0.,1.);
    for c in 0..4 {let v=if c==3 {alpha}else{p[c].clamp(0.,if depth==PixelDepth::Hdr {65504.}else{alpha})};
        out[c]=if depth==PixelDepth::Hdr {half::f16::from_f32(v).to_bits()}else{(v*65535.).round() as u16};}
}
pub fn gaussian(r:&Raster,sigma:f64)->Raster {gaussian_alpha(r,sigma,false)}
pub fn gaussian_alpha(r:&Raster,sigma:f64,preserve:bool)->Raster {gaussian_edges(r,sigma,preserve,CanvasSides::default())}
pub fn gaussian_edges(r:&Raster,sigma:f64,preserve:bool,sides:CanvasSides)->Raster {
    if sigma<=0. {return r.clone();}
    let(w,h)=(r.width as i64,r.height as i64);let radius=(sigma*3.).ceil() as i64;
    let kernel:Vec<f32>=(-radius..=radius).map(|i|(-(i*i) as f64/(2.*sigma*sigma)).exp() as f32).collect();
    let sum:f32=kernel.iter().sum();let band=(2*radius+1).min(h);
    let mut ring=vec![0.;(band*w*4) as usize];let mut samples=vec![0;if r.depth()==PixelDepth::Eight {0}else{(w*h*4)as usize}];let mut bytes=vec![0;if r.depth()==PixelDepth::Eight {(w*h*4)as usize}else{0}];let mut next=0;
    for y in 0..h {
        while next<h && next<=y+radius {
            let slot=((next%band)*w*4) as usize;
            for x in 0..w {let mut acc=[0.;4];
                for(k,weight)in kernel.iter().enumerate(){let Some(xx)=edge_index(x+k as i64-radius,w,sides.left,sides.right)else{continue;};
                    let p=r.pixel_f32(xx as u32,next as u32);for c in 0..4{acc[c]+=p[c]*weight;}}
                for c in 0..4 {ring[slot+x as usize*4+c]=acc[c]/sum;}
            }next+=1;
        }
        for x in 0..w {let mut acc=[0.;4];
            for(k,weight)in kernel.iter().enumerate(){let Some(yy)=edge_index(y+k as i64-radius,h,sides.top,sides.bottom)else{continue;};
                let at=((yy%band)*w*4+x*4) as usize;for c in 0..4{acc[c]+=ring[at+c]*weight;}}
            let mut p=acc.map(|v|v/sum);
            if preserve {let original=r.pixel_f32(x as u32,y as u32);if p[3]>0. {for c in 0..3{p[c]=p[c]/p[3]*original[3];}}else{p=original;}p[3]=original[3];}
            let at=((y*w+x)*4)as usize;
            if r.depth()==PixelDepth::Eight {for c in 0..4{bytes[at+c]=(p[c].clamp(0.,p[3])*255.).round()as u8;}}
            else{write(&mut samples[at..][..4],p,r.depth());}
        }
    }
    if r.depth()==PixelDepth::Eight {Raster::from_premultiplied(r.width,r.height,bytes)}else{Raster::from_deep(r.width,r.height,DeepPixels{depth:r.depth(),samples})}
}
pub fn motion(r:&Raster,angle:f64,distance:f64)->Raster {motion_edges(r,angle,distance,CanvasSides::default())}
pub fn motion_edges(r:&Raster,angle:f64,distance:f64,sides:CanvasSides)->Raster {
    let(stencil,sum)=super::filters::motion_stencil(angle,distance);if stencil.is_empty(){return r.clone();}
    let(w,h)=(r.width as i64,r.height as i64);let mut samples=vec![0;if r.depth()==PixelDepth::Eight{0}else{(w*h*4) as usize}];let mut bytes=vec![0;if r.depth()==PixelDepth::Eight{(w*h*4) as usize}else{0}];
    for y in 0..h {for x in 0..w {let mut acc=[0.;4];
        for &(dx,dy,weight)in &stencil {let(Some(xx),Some(yy))=(edge_index(x+dx,w,sides.left,sides.right),edge_index(y+dy,h,sides.top,sides.bottom))else{continue;};
            let p=r.pixel_f32(xx as u32,yy as u32);for c in 0..4 {acc[c]+=p[c]*weight;}}
        let at=((y*w+x)*4)as usize;let p=acc.map(|v|v/sum);if r.depth()==PixelDepth::Eight{for c in 0..4{bytes[at+c]=(p[c].clamp(0.,p[3])*255.).round()as u8;}}else{write(&mut samples[at..][..4],p,r.depth());}
    }}
    if r.depth()==PixelDepth::Eight{Raster::from_premultiplied(r.width,r.height,bytes)}else{Raster::from_deep(r.width,r.height,DeepPixels{depth:r.depth(),samples})}
}
pub fn distort(r:&Raster,k:f64)->Raster {
    if k==0. {return r.clone();}
    let(w,h)=(r.width,r.height);let(cx,cy)=(w as f64/2.,h as f64/2.);let diagonal=cx*cx+cy*cy;let mut samples=vec![0;w as usize*h as usize*4];
    for y in 0..h {for x in 0..w {let(dx,dy)=(x as f64+0.5-cx,y as f64+0.5-cy);let scale=1.-k*(dx*dx+dy*dy)/diagonal;
        let(fx,fy)=(cx+dx*scale-0.5,cy+dy*scale-0.5);let(ix,iy)=(fx.floor() as i64,fy.floor() as i64);let(tx,ty)=((fx-ix as f64) as f32,(fy-iy as f64) as f32);let mut p=[0.;4];
        for(xx,yy,weight)in [(ix,iy,(1.-tx)*(1.-ty)),(ix+1,iy,tx*(1.-ty)),(ix,iy+1,(1.-tx)*ty),(ix+1,iy+1,tx*ty)] {
            if xx<0||yy<0||xx>=w as i64||yy>=h as i64 {continue;}let q=r.pixel_f32(xx as u32,yy as u32);for c in 0..4 {p[c]+=q[c]*weight;}}
        write(&mut samples[((y*w+x)*4) as usize..][..4],p,r.depth());
    }}
    Raster::from_deep(w,h,DeepPixels{depth:r.depth(),samples})
}
