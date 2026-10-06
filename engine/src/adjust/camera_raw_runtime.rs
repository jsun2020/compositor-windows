//! Allocation accounting for the upstream void Camera Raw C kernels.
use std::{
    alloc::{alloc, dealloc, Layout},
    cell::Cell,
    ffi::c_void,
};
thread_local! {
    static USED:Cell<usize>=const{Cell::new(0)};
    static FAILED:Cell<bool>=const{Cell::new(false)};
}
// Kernel scratch only; image/history keep their existing independent budgets.
const SCRATCH_LIMIT: usize = 1536 * 1024 * 1024;
pub(super) fn begin() {
    FAILED.with(|f| f.set(false));
}
pub(super) fn failed() -> bool {
    FAILED.with(Cell::get)
}
#[no_mangle]
unsafe extern "C" fn phase7_malloc(n: usize) -> *mut c_void {
    let Some(size) = n.max(1).checked_add(16) else {
        FAILED.with(|f| f.set(true));
        return std::ptr::null_mut();
    };
    let allowed = USED.with(|u| {
        u.get()
            .checked_add(size)
            .is_some_and(|v| v <= SCRATCH_LIMIT)
    });
    let Ok(layout) = Layout::from_size_align(size, 16) else {
        FAILED.with(|f| f.set(true));
        return std::ptr::null_mut();
    };
    if !allowed {
        FAILED.with(|f| f.set(true));
        return std::ptr::null_mut();
    }
    let p = alloc(layout);
    if p.is_null() {
        FAILED.with(|f| f.set(true));
        return p.cast();
    }
    p.cast::<usize>().write(size);
    USED.with(|u| u.set(u.get() + size));
    p.add(16).cast()
}
#[no_mangle]
unsafe extern "C" fn phase7_free(p: *mut c_void) {
    if p.is_null() {
        return;
    }
    let base = p.cast::<u8>().sub(16);
    let size = base.cast::<usize>().read();
    USED.with(|u| u.set(u.get() - size));
    dealloc(base, Layout::from_size_align_unchecked(size, 16));
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scratch_limit_failure_is_recorded_and_accounting_recovers(){
        begin();let before=USED.with(Cell::get);
        unsafe{assert!(phase7_malloc(SCRATCH_LIMIT).is_null());assert!(failed());begin();let p=phase7_malloc(23);assert!(!p.is_null());assert_eq!(p as usize%16,0);assert!(!failed());phase7_free(p);phase7_free(std::ptr::null_mut());}
        assert_eq!(USED.with(Cell::get),before);
    }
}
#[cfg(target_arch = "wasm32")]
mod math {
    #[no_mangle]
    extern "C" fn compositor_pow(x: f64, y: f64) -> f64 {
        x.powf(y)
    }
    #[no_mangle]
    extern "C" fn compositor_exp2(x: f64) -> f64 {
        x.exp2()
    }
    #[no_mangle]
    extern "C" fn compositor_fmod(x: f64, y: f64) -> f64 {
        x % y
    }
    #[no_mangle]
    extern "C" fn compositor_tanh(x: f64) -> f64 {
        x.tanh()
    }
    #[no_mangle]
    extern "C" fn compositor_hypot(x: f64, y: f64) -> f64 {
        x.hypot(y)
    }
}
