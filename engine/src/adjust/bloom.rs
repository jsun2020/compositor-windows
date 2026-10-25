//! Bloom adds the positive part of a Gaussian's difference from the source.
//! Uniform areas retain their tone; bright pixels spill into darker neighbors.
use crate::{DeepPixels,PixelDepth,Raster};
use super::deep_filters;
pub fn bloom(r:&Raster,amount:f64,radius:f64)->Raster {
    if amount<=0. {return r.clone();}
    let blurred=super::filters::gaussian_blur(r,radius);let gain=(amount/50.)as f32;
    let count=r.width as usize*r.height as usize*4;
    let mut bytes=vec![0;if r.depth()==PixelDepth::Eight {count}else{0}];
    let mut samples=vec![0;if r.depth()==PixelDepth::Eight {0}else{count}];
    for y in 0..r.height {for x in 0..r.width {
        let a=r.pixel_f32(x,y);let b=blurred.pixel_f32(x,y);
        let p=std::array::from_fn(|c|a[c]+(b[c]-a[c]).max(0.)*gain);
        let at=(y as usize*r.width as usize+x as usize)*4;
        if r.depth()==PixelDepth::Eight{let alpha=p[3].clamp(0.,1.);for c in 0..4{bytes[at+c]=(p[c].clamp(0.,alpha)*255.).round()as u8;}}
        else{deep_filters::write(&mut samples[at..at+4],p,r.depth());}
    }}
    drop(blurred);
    if r.depth()==PixelDepth::Eight {Raster::from_premultiplied(r.width,r.height,bytes)}else{Raster::from_deep(r.width,r.height,DeepPixels{depth:r.depth(),samples})}
}
