//! Immutable high precision pixels alongside the dithered RGBA8 tool surface.
//! DeepPixels.c is the MIT-licensed Compositor v1.5.7 implementation.
use crate::Raster;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PixelDepth { #[default] Eight, Sixteen, Hdr }
#[derive(Clone, Debug, PartialEq)]
pub struct DeepPixels { pub depth: PixelDepth, pub samples: Vec<u16> }
extern "C" {
    fn dither_quantize16(source: *const u16, out: *mut u8, width: usize, height: usize, stride: usize);
    fn deep_take_painted(deep: *mut u16, deep_stride: usize, painted: *const u8, base: *const u8, stride: usize, width: usize, height: usize);
    fn deep_take_painted_hdr(deep: *mut u16, deep_stride: usize, painted: *const u8, base: *const u8, stride: usize, width: usize, height: usize);
    fn deep_split(deep: *const u16, deep_stride: usize, out: *mut u8, stride: usize, width: usize, height: usize, step: i32, channel: i32);
    fn deep_merge(deep: *const u16, deep_stride: usize, low: *const u8, near: *const *const u8, mid: *const *const u8, far: *const *const u8, stride: usize, out: *mut u16, out_stride: usize, width: usize, height: usize, mixes: i32);
    fn deep_hdr_normalize(hdr: *const u16, hdr_stride: usize, out: *mut u16, out_stride: usize, scales: *mut f32, width: usize, height: usize);
    fn deep_hdr_restore(normal: *const u16, normal_stride: usize, scales: *const f32, hdr: *mut u16, hdr_stride: usize, width: usize, height: usize);
    fn deep_hdr_exposure(hdr: *mut u16, stride: usize, width: usize, height: usize, scale: f32, offset: f32, inverse_gamma: f32);
}
impl DeepPixels {
    pub fn preview(&self, width: u32, height: u32) -> Vec<u8> {
        let mut out=vec![0; width as usize*height as usize*4];
        let normal;
        let samples=if self.depth==PixelDepth::Hdr {
            normal=self.samples.iter().map(|v| (half::f16::from_bits(*v).to_f32().clamp(0.,1.)*65535.).round() as u16).collect::<Vec<_>>();
            &normal
        } else { &self.samples };
        unsafe { dither_quantize16(samples.as_ptr(),out.as_mut_ptr(),width as usize,height as usize,width as usize*4); }
        out
    }
    pub fn straight_u16(&self) -> Vec<u16> {
        assert_eq!(self.depth,PixelDepth::Sixteen);
        let mut out=self.samples.clone();
        for pixel in out.chunks_exact_mut(4) {
            let a=pixel[3] as u64;
            if a>0 && a<65535 { for c in 0..3 { pixel[c]=((pixel[c] as u64*65535+a/2)/a).min(65535) as u16; } }
        }
        out
    }
    pub fn straight_f32(&self) -> Vec<f32> {
        let mut out:Vec<f32>=self.samples.iter().map(|v| if self.depth==PixelDepth::Hdr {half::f16::from_bits(*v).to_f32()} else {*v as f32/65535.}).collect();
        for p in out.chunks_exact_mut(4) { let a=p[3]; if a>0. {for c in 0..3 {p[c]/=a;}} }
        out
    }
}
impl Raster {
    /// Transfer samples without rounding through the displayed RGBA8 copy. Windows and WASM are little endian.
    pub fn transfer_ptr(&self) -> *const u8 { self.deep_pixels().map_or(self.bytes().as_ptr(), |p|p.samples.as_ptr().cast()) }
    pub fn transfer_len(&self) -> usize { self.deep_pixels().map_or(self.bytes().len(), |p|p.samples.len()*2) }

    pub fn from_rgba16(width: u32, height: u32, mut straight: Vec<u16>) -> Self {
        assert_eq!(straight.len(),width as usize*height as usize*4);
        for p in straight.chunks_exact_mut(4) {
            let a=p[3] as u64;
            for c in 0..3 {p[c]=((p[c] as u64*a+32767)/65535) as u16;}
        }
        Self::from_deep(width,height,DeepPixels{depth:PixelDepth::Sixteen,samples:straight})
    }
    pub fn from_rgba_float(width: u32, height: u32, straight: Vec<f32>) -> Self {
        assert_eq!(straight.len(),width as usize*height as usize*4);
        // Imported EXR uses extended sRGB, matching the layer's own canonical space.
        let hdr=straight.chunks_exact(4).any(|p|p[..3].iter().any(|v|v.is_finite()&&*v>1.001));
        if !hdr {return Self::from_rgba16(width,height,straight.iter().map(|v|(v.clamp(0.,1.)*65535.).round() as u16).collect());}
        let mut samples=Vec::with_capacity(straight.len());
        for p in straight.chunks_exact(4) {
            let a=if p[3].is_finite(){p[3].clamp(0.,1.)}else{0.};
            for v in &p[..3] {samples.push(half::f16::from_f32(if v.is_finite(){(v*a).clamp(0.,65504.)}else{0.}).to_bits());}
            samples.push(half::f16::from_f32(a).to_bits());
        }
        Self::from_deep(width,height,DeepPixels{depth:PixelDepth::Hdr,samples})
    }
    pub fn to_depth(&self, depth: PixelDepth) -> Self {
        if self.depth()==depth {return self.clone();}
        match depth {
            PixelDepth::Eight => Self::from_premultiplied(self.width,self.height,self.bytes().to_vec()),
            PixelDepth::Sixteen => {
                let samples=match self.deep_pixels() {
                    Some(p) => p.samples.iter().map(|v|(half::f16::from_bits(*v).to_f32().clamp(0.,1.)*65535.).round() as u16).collect(),
                    None => self.bytes().iter().map(|v|*v as u16*257).collect(),
                };
                Self::from_deep(self.width,self.height,DeepPixels{depth,samples})
            }
            PixelDepth::Hdr => {
                let samples=match self.deep_pixels() {
                    Some(p)=>p.samples.iter().map(|v|half::f16::from_f32(*v as f32/65535.).to_bits()).collect(),
                    None=>self.bytes().iter().map(|v|half::f16::from_f32(*v as f32/255.).to_bits()).collect(),
                };
                Self::from_deep(self.width,self.height,DeepPixels{depth,samples})
            }
        }
    }
    /// Place an unchanged pixel grid on a larger transparent surface without requantizing it.
    pub fn padded(&self, width: u32, height: u32, x: u32, y: u32) -> Self {
        assert!(x as u64+self.width as u64<=width as u64 && y as u64+self.height as u64<=height as u64);
        if (width,height,x,y)==(self.width,self.height,0,0) { return self.clone(); }
        if let Some(deep)=self.deep_pixels() {
            let mut samples=vec![0;width as usize*height as usize*4];
            let mut preview=vec![0;samples.len()];
            for row in 0..self.height as usize {
                let from=row*self.width as usize*4;let to=((row+y as usize)*width as usize+x as usize)*4;
                samples[to..to+self.width as usize*4].copy_from_slice(&deep.samples[from..from+self.width as usize*4]);
                preview[to..to+self.width as usize*4].copy_from_slice(&self.bytes()[from..from+self.width as usize*4]);
            }
            Self::from_deep_preview(width,height,DeepPixels{depth:deep.depth,samples},preview)
        } else {
            let mut bytes=vec![0;width as usize*height as usize*4];
            for row in 0..self.height as usize {
                let from=row*self.width as usize*4;let to=((row+y as usize)*width as usize+x as usize)*4;
                bytes[to..to+self.width as usize*4].copy_from_slice(&self.bytes()[from..from+self.width as usize*4]);
            }
            Self::from_premultiplied(width,height,bytes)
        }
    }
    /// Keep untouched high precision samples when an RGBA8 brush changes only part of a layer.
    pub fn with_painted(&self, painted: &Raster) -> Self {
        let Some(deep)=self.deep_pixels() else {return painted.clone();};
        if (self.width,self.height)!=(painted.width,painted.height) || painted.deep_pixels().is_some() {return painted.clone();}
        let mut result=deep.clone();let(w,h)=(self.width as usize,self.height as usize);
        unsafe { if deep.depth==PixelDepth::Hdr {deep_take_painted_hdr(result.samples.as_mut_ptr(),w*8,painted.bytes().as_ptr(),self.bytes().as_ptr(),w*4,w,h);} else {deep_take_painted(result.samples.as_mut_ptr(),w*8,painted.bytes().as_ptr(),self.bytes().as_ptr(),w*4,w,h);} }
        Self::from_deep(self.width,self.height,result)
    }
    /// Upstream four/ten-pass precision reconstruction for non-spatial 8-bit operations.
    pub fn map_deep<E>(&self, mixes: bool, mut operation: impl FnMut(&Raster)->Result<Raster,E>) -> Result<Self,E> {
        let Some(deep)=self.deep_pixels() else {return operation(self);};
        let (w,h)=(self.width as usize,self.height as usize);let len=w*h*4;
        let mut normalized=deep.samples.clone();let mut scales=vec![1.;w*h];
        if deep.depth==PixelDepth::Hdr {unsafe{deep_hdr_normalize(deep.samples.as_ptr(),w*8,normalized.as_mut_ptr(),w*8,scales.as_mut_ptr(),w,h);}}
        let mut run=|step,channel|->Result<Raster,E>{
            let mut bytes=vec![0;len];unsafe{deep_split(normalized.as_ptr(),w*8,bytes.as_mut_ptr(),w*4,w,h,step,channel);}
            operation(&Raster::from_premultiplied(self.width,self.height,bytes))
        };
        let low=run(0,-1)?;
        let mut results=Vec::new();
        for step in [1,2,4] {for c in 0..if mixes {3}else{1} {results.push(run(step,if mixes{c}else{-1})?);}}
        assert!((low.width,low.height)==(self.width,self.height) && results.iter().all(|r|(r.width,r.height)==(self.width,self.height)), "deep reconstruction requires a same-size operation");
        let n=if mixes{3}else{1};
        let pointers=|offset: usize|std::array::from_fn::<_,3,_>(|c|results[offset+if mixes{c}else{0}].bytes().as_ptr());
        let (near,mid,far)=(pointers(0),pointers(n),pointers(n*2));let mut samples=vec![0;len];
        unsafe{deep_merge(normalized.as_ptr(),w*8,low.bytes().as_ptr(),near.as_ptr(),mid.as_ptr(),far.as_ptr(),w*4,samples.as_mut_ptr(),w*8,w,h,i32::from(mixes));}
        if deep.depth==PixelDepth::Hdr {let mut hdr=vec![0;len];unsafe{deep_hdr_restore(samples.as_ptr(),w*8,scales.as_ptr(),hdr.as_mut_ptr(),w*8,w,h);}samples=hdr;}
        Ok(Self::from_deep(self.width,self.height,DeepPixels{depth:deep.depth,samples}))
    }
    pub fn hdr_exposure(&self, exposure: f64, offset: f64, gamma: f64) -> Self {
        let mut pixels=self.deep_pixels().expect("HDR pixels").clone();
        assert_eq!(pixels.depth,PixelDepth::Hdr);
        unsafe{deep_hdr_exposure(pixels.samples.as_mut_ptr(),self.width as usize*8,self.width as usize,self.height as usize,2f64.powf(exposure) as f32,offset as f32,(1./gamma) as f32);}
        Self::from_deep(self.width,self.height,pixels)
    }
}

impl Raster {
    pub fn pixel_f32(&self, x:u32, y:u32)->[f32;4] {
        let at=(y as usize*self.width as usize+x as usize)*4;
        match self.deep_pixels() {
            Some(p) if p.depth==PixelDepth::Hdr=>std::array::from_fn(|c|half::f16::from_bits(p.samples[at+c]).to_f32()),
            Some(p)=>std::array::from_fn(|c|p.samples[at+c] as f32/65535.),
            None=>self.pixel(x,y).map(|v|v as f32/255.),
        }
    }
    /// Streaming high-precision output: retain only the final 8-byte samples per pixel.
    pub(crate) fn from_deep_fn(width:u32,height:u32,depth:PixelDepth,mut pixel:impl FnMut(u32,u32)->[f32;4])->Self {
        assert_ne!(depth,PixelDepth::Eight);let mut samples=Vec::with_capacity(width as usize*height as usize*4);
        for y in 0..height{for x in 0..width{let p=pixel(x,y);let alpha=p[3].clamp(0.,1.);for(c,v)in p.into_iter().enumerate(){let limit=if c==3{1.}else if depth==PixelDepth::Hdr{65504.}else{alpha};let v=if v.is_finite(){v.clamp(0.,limit)}else{0.};samples.push(if depth==PixelDepth::Hdr{half::f16::from_f32(v).to_bits()}else{(v*65535.).round()as u16});}}}
        Self::from_deep(width,height,DeepPixels{depth,samples})
    }
    pub fn from_premultiplied_f32(width:u32,height:u32,values:Vec<f32>,depth:PixelDepth)->Self {
        assert_eq!(values.len(),width as usize*height as usize*4);
        assert_ne!(depth,PixelDepth::Eight);
        let mut samples=Vec::with_capacity(values.len());
        for p in values.chunks_exact(4) {
            let alpha=p[3].clamp(0.,1.);
            for (c,v) in p.iter().enumerate() {
                let limit=if c==3 {1.} else if depth==PixelDepth::Hdr {65504.} else {alpha};
                let v=if v.is_finite(){v.clamp(0.,limit)}else{0.};
                samples.push(if depth==PixelDepth::Hdr {half::f16::from_f32(v).to_bits()}else{(v*65535.).round() as u16});
            }
        }
        Self::from_deep(width,height,DeepPixels{depth,samples})
    }
    /// High precision prefiltering for export and destructive resampling. Display caches stay RGBA8.
    pub fn reduced_deep(&self,level:u32)->Self {
        if self.depth()==PixelDepth::Eight {return self.reduced(level);}
        let mut result=self.clone();
        for _ in 0..level {
            let(w,h)=result.half_size();let mut data=vec![0.;w as usize*h as usize*4];
            for y in 0..h {for x in 0..w {
                let mut n=0.;let mut sum=[0.;4];
                for yy in y*2..(y*2+2).min(result.height) {for xx in x*2..(x*2+2).min(result.width) {
                    let p=result.pixel_f32(xx,yy);for c in 0..4 {sum[c]+=p[c];}n+=1.;
                }}
                for c in 0..4 {data[((y*w+x)*4) as usize+c]=sum[c]/n;}
            }}
            result=Self::from_premultiplied_f32(w,h,data,self.depth());
        }
        result
    }
}
