//! Enumerate installed font families without launching a shell or loading font files.
#[tauri::command]
pub fn list_fonts() -> Result<Vec<String>,String> {
    #[cfg(windows)] { windows_fonts::families() }
    #[cfg(not(windows))] { Ok(vec!["Arial".into(),"Georgia".into(),"Helvetica".into(),"Times New Roman".into()]) }
}
#[cfg(windows)]
mod windows_fonts {
    use std::{collections::BTreeSet,ffi::c_void};
    #[repr(C)]
    struct LogFont {height:i32,width:i32,escapement:i32,orientation:i32,weight:i32,italic:u8,underline:u8,strikeout:u8,charset:u8,out_precision:u8,clip_precision:u8,quality:u8,pitch_family:u8,face:[u16;32]}
    type Hdc=*mut c_void;
    #[link(name="user32")]
    extern "system" {fn GetDC(window:*mut c_void)->Hdc;fn ReleaseDC(window:*mut c_void,dc:Hdc)->i32;}
    #[link(name="gdi32")]
    extern "system" {fn EnumFontFamiliesExW(dc:Hdc,font:*const LogFont,callback:Option<unsafe extern "system" fn(*const LogFont,*const c_void,u32,isize)->i32>,parameter:isize,flags:u32)->i32;}
    unsafe extern "system" fn collect(font:*const LogFont,_metrics:*const c_void,_kind:u32,parameter:isize)->i32 {
        if font.is_null(){return 1;}
        let families=&mut *(parameter as *mut BTreeSet<String>);let face=&(*font).face;
        let length=face.iter().position(|v|*v==0).unwrap_or(face.len());let name=String::from_utf16_lossy(&face[..length]);
        if !name.is_empty()&&!name.starts_with('@'){families.insert(name);}
        i32::from(families.len()<4096)
    }
    pub(super) fn families()->Result<Vec<String>,String>{
        let mut families=BTreeSet::new();
        unsafe {
            let dc=GetDC(std::ptr::null_mut());if dc.is_null(){return Err("Installed fonts could not be read".into());}
            let mut font:LogFont=std::mem::zeroed();font.charset=1; // DEFAULT_CHARSET: enumerate all scripts.
            EnumFontFamiliesExW(dc,&font,Some(collect),&mut families as *mut BTreeSet<String> as isize,0);
            ReleaseDC(std::ptr::null_mut(),dc);
        }
        Ok(families.into_iter().collect())
    }
}
