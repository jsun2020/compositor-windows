//! The original v1.4.5 C kernels, compiled for Windows and WASM. Only their
//! allocator and math runtime boundary changes on WASM; algorithms are intact.
use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealingMode {
    #[serde(rename="Content-Aware")] ContentAware,
    #[serde(rename="Create Texture")] CreateTexture,
    #[serde(rename="Proximity Match")] ProximityMatch,
}
extern "C" {
    fn spot_heal(rgba: *mut u8, coverage: *const u8, width: usize, height: usize, stride: usize, opacity: f32, mode: i32, seed: u32) -> i32;
    fn content_fill(rgba: *mut u8, stride: usize, mask: *const u8, mask_stride: usize, width: i32, height: i32) -> i32;
}
pub fn heal(raster: &Raster, coverage: &GrayRaster, opacity: f32, mode: HealingMode, seed: u32) -> Result<Raster, CommandError> {
    if (raster.width,raster.height)!=(coverage.width,coverage.height) || !opacity.is_finite() || !(0.0..=1.0).contains(&opacity) {return Err(CommandError::Argument("invalid healing input".into()));}
    let mut out=raster.bytes().to_vec();
    let mode=match mode{HealingMode::ContentAware=>0,HealingMode::CreateTexture=>1,HealingMode::ProximityMatch=>2};
    let result=unsafe{spot_heal(out.as_mut_ptr(),coverage.bytes().as_ptr(),raster.width as usize,raster.height as usize,raster.width as usize*4,opacity,mode,seed)};
    if result<0 {return Err(CommandError::Refused("The healing edit ran out of memory".into()));}
    Ok(Raster::from_premultiplied(raster.width,raster.height,out))
}
pub const NO_FILL_SOURCE: &str = "Not enough unselected, opaque image pixels to synthesize a fill. Use a smaller selection with some surrounding image.";
pub fn fill(raster: &Raster, coverage: &GrayRaster) -> Result<Raster,CommandError> {
    if (raster.width,raster.height)!=(coverage.width,coverage.height) {return Err(CommandError::Argument("invalid content fill input".into()));}
    let mut out=raster.bytes().to_vec();
    let result=unsafe{content_fill(out.as_mut_ptr(),raster.width as usize*4,coverage.bytes().as_ptr(),coverage.width as usize,raster.width as i32,raster.height as i32)};
    if result==0{return Err(CommandError::Refused(NO_FILL_SOURCE.into()));}
    if result<0{return Err(CommandError::Refused("The content fill ran out of memory".into()));}
    Ok(Raster::from_premultiplied(raster.width,raster.height,out))
}

#[cfg(target_arch="wasm32")]
mod wasm_runtime {
    use std::{alloc::{alloc,alloc_zeroed,dealloc,Layout},ffi::c_void};
    unsafe fn allocation(n:usize,zero:bool)->*mut c_void {
        let Some(size)=n.max(1).checked_add(8) else{return std::ptr::null_mut();};
        let Ok(layout)=Layout::from_size_align(size,8) else{return std::ptr::null_mut();};
        let p=if zero{alloc_zeroed(layout)}else{alloc(layout)};
        if p.is_null(){return p.cast();}
        p.cast::<usize>().write(size);p.add(8).cast()
    }
    #[no_mangle] unsafe extern "C" fn compositor_malloc(n:usize)->*mut c_void{allocation(n,false)}
    #[no_mangle] unsafe extern "C" fn compositor_calloc(n:usize,s:usize)->*mut c_void{match n.checked_mul(s){Some(n)=>allocation(n,true),None=>std::ptr::null_mut()}}
    #[no_mangle] unsafe extern "C" fn compositor_free(p:*mut c_void){if !p.is_null(){let p=p.cast::<u8>().sub(8);let size=p.cast::<usize>().read();dealloc(p,Layout::from_size_align_unchecked(size,8));}}
    #[no_mangle] unsafe extern "C" fn compositor_memcpy(dst:*mut c_void,src:*const c_void,n:usize)->*mut c_void{std::ptr::copy_nonoverlapping(src.cast::<u8>(),dst.cast::<u8>(),n);dst}
    #[no_mangle] unsafe extern "C" fn compositor_memset(dst:*mut c_void,v:i32,n:usize)->*mut c_void{std::ptr::write_bytes(dst.cast::<u8>(),v as u8,n);dst}
    #[no_mangle] extern "C" fn compositor_sqrt(x:f64)->f64{x.sqrt()}
    #[no_mangle] extern "C" fn compositor_log(x:f64)->f64{x.ln()}
    #[no_mangle] extern "C" fn compositor_sin(x:f64)->f64{x.sin()}
    #[no_mangle] extern "C" fn compositor_cos(x:f64)->f64{x.cos()}
    #[no_mangle] extern "C" fn compositor_lround(x:f64)->i32{x.round() as i32}
    #[no_mangle] extern "C" fn compositor_labs(x:i32)->i32{x.abs()}
}
