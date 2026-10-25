use windows::{core::{PCWSTR,PWSTR},Win32::UI::ColorSystem::*};
use std::{path::PathBuf,ffi::c_void};
struct Profile(isize);impl Drop for Profile{fn drop(&mut self){unsafe{let _=CloseColorProfile(Some(self.0));}}}
struct Transform(isize);impl Drop for Transform{fn drop(&mut self){unsafe{let _=DeleteColorTransform(self.0);}}}
fn error(label:&str)->String{format!("{label}: {}",std::io::Error::last_os_error())}
fn open(bytes:&[u8])->Result<Profile,String>{
    let p=PROFILE{dwType:PROFILE_MEMBUFFER,pProfileData:bytes.as_ptr() as *mut c_void,cbDataSize:bytes.len() as u32};
    let h=unsafe{OpenColorProfileW(&p,PROFILE_READ,1,3)};if h==0{Err(error("Windows cannot open this ICC profile"))}else{Ok(Profile(h))}
}
fn transform(from:&Profile,to:&Profile)->Result<Transform,String>{
    let h=unsafe{CreateMultiProfileTransform(&[from.0,to.0],&[INTENT_RELATIVE_COLORIMETRIC,INTENT_RELATIVE_COLORIMETRIC],BEST_MODE,0)};
    if h==0{Err(error("Windows cannot build this ICC transform"))}else{Ok(Transform(h))}
}
pub fn srgb_bytes()->Result<Vec<u8>,String>{
    let mut buf=vec![0u16;32768];let mut bytes=(buf.len()*2)as u32;
    if !unsafe{GetStandardColorSpaceProfileW(PCWSTR::null(),LCS_sRGB.0 as u32,Some(PWSTR(buf.as_mut_ptr())),&mut bytes)}.as_bool(){return Err(error("Cannot find the Windows sRGB profile"));}
    let n=buf.iter().position(|v|*v==0).ok_or("Invalid sRGB profile path")?;
    std::fs::read(String::from_utf16_lossy(&buf[..n])).map_err(|e|e.to_string())
}
fn srgb()->Result<Profile,String>{open(&srgb_bytes()?)}
pub fn directory()->Result<PathBuf,String>{
    let mut buf=vec![0u16;32768];let mut bytes=(buf.len()*2)as u32;
    if !unsafe{GetColorDirectoryW(PCWSTR::null(),Some(PWSTR(buf.as_mut_ptr())),&mut bytes)}.as_bool(){return Err(error("Cannot find installed ICC profiles"));}
    let n=buf.iter().position(|v|*v==0).ok_or("Invalid ICC directory")?;Ok(PathBuf::from(String::from_utf16_lossy(&buf[..n])))
}
fn translate(t:&Transform,input:&[u8],output:&mut[u8],w:u32,h:u32,from:BMFORMAT,to:BMFORMAT,channels_in:u32,channels_out:u32)->Result<(),String>{
    if input.len()!=w as usize*h as usize*channels_in as usize||output.len()!=w as usize*h as usize*channels_out as usize{return Err("Invalid color conversion buffer".into());}
    if !unsafe{TranslateBitmapBits(t.0,input.as_ptr().cast(),from,w,h,w*channels_in,output.as_mut_ptr().cast(),to,w*channels_out,None,None)}.as_bool(){return Err(error("Windows ICC conversion failed"));}Ok(())
}
pub fn to_cmyk(icc:&[u8],rgba:&mut[u8],w:u32,h:u32)->Result<(),String>{
    if rgba.len()!=w as usize*h as usize*4{return Err("Invalid RGB raster".into());}
    let rgb=srgb()?;let ink=open(icc)?;let t=transform(&rgb,&ink)?;
    // Names describe the numeric word, not byte order: xBGR is RGBA, KYMC is CMYK.
    // Keep the full raster in place and use a bounded scratch band for the transform.
    let mut output=vec![0;w as usize*64*4];
    for band in rgba.chunks_mut(w as usize*64*4){let rows=(band.len()/4/w as usize)as u32;let out=&mut output[..band.len()];translate(&t,band,out,w,rows,BM_xBGRQUADS,BM_KYMCQUADS,4,4)?;band.copy_from_slice(out);}Ok(())
}
pub fn proof(icc:&[u8])->Result<Vec<u8>,String>{
    let rgb=srgb()?;let ink=open(icc)?;let forward=transform(&rgb,&ink)?;let reverse=transform(&ink,&rgb)?;
    let mut source=Vec::with_capacity(33*33*33*3);
    for b in 0..33{for g in 0..33{for r in 0..33{source.extend([r,g,b].map(|v|((v as f64/32.)*255.).round()as u8));}}}
    let mut inks=vec![0;33*33*33*4];let mut proof=vec![0;source.len()];
    translate(&forward,&source,&mut inks,33,33*33,BM_BGRTRIPLETS,BM_KYMCQUADS,3,4)?;
    translate(&reverse,&inks,&mut proof,33,33*33,BM_KYMCQUADS,BM_BGRTRIPLETS,4,3)?;Ok(proof)
}
pub struct InkTransform {transform:Transform,_rgb:Profile,_ink:Profile}
impl InkTransform {
    pub fn new(icc:&[u8])->Result<Self,String>{let rgb=srgb()?;let ink=open(icc)?;let t=transform(&ink,&rgb)?;Ok(Self{transform:t,_rgb:rgb,_ink:ink})}
    pub fn apply(&self,ink:&mut[u8],w:u32,h:u32)->Result<(),String>{
        if w==0||h==0||ink.len()!=w as usize*h as usize*4{return Err("Invalid CMYK raster".into());}
        let mut output=vec![0;w as usize*64*4];
        for band in ink.chunks_mut(w as usize*64*4){let rows=(band.len()/4/w as usize)as u32;let out=&mut output[..band.len()];translate(&self.transform,band,out,w,rows,BM_KYMCQUADS,BM_xBGRQUADS,4,4)?;for p in out.chunks_exact_mut(4){p[3]=255;}band.copy_from_slice(out);}Ok(())
    }
}
