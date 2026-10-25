//! WIC keeps JPEG/YCCK/TIFF ink values and applies the embedded or explicitly chosen ICC.
use std::{os::windows::ffi::OsStrExt, path::Path};
use windows::{
    core::{Interface, PCWSTR},
    Win32::{Foundation::GENERIC_READ, Graphics::Imaging::*, System::Com::*},
};
struct Apartment;
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe { CoUninitialize() }
    }
}
pub struct Image {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub profile: Vec<u8>,
    pub dpi: f64,
}
pub fn decode(
    path: &Path,
    fallback: impl FnOnce() -> Result<Option<Vec<u8>>, String>,
    srgb: &[u8],
) -> Result<Option<Image>, String> {
    fn error(e: windows::core::Error) -> String {
        format!("Windows ICC image decoder: {e}")
    }
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED)
            .ok()
            .map_err(error)?;
        let _apartment = Apartment;
        let factory: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)
                .map_err(error)?;
        let path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let decoder = factory
            .CreateDecoderFromFilename(
                PCWSTR(path.as_ptr()),
                None,
                GENERIC_READ,
                WICDecodeMetadataCacheOnDemand,
            )
            .map_err(error)?;
        let frame = decoder.GetFrame(0).map_err(error)?;
        let format = frame.GetPixelFormat().map_err(error)?;
        if ![
            GUID_WICPixelFormat32bppCMYK,
            GUID_WICPixelFormat64bppCMYK,
            GUID_WICPixelFormat40bppCMYKAlpha,
            GUID_WICPixelFormat80bppCMYKAlpha,
        ]
        .contains(&format)
        {
            return Ok(None);
        }
        let (mut width, mut height) = (0, 0);
        frame.GetSize(&mut width, &mut height).map_err(error)?;
        if width == 0
            || height == 0
            || width > 30000
            || height > 30000
            || width as u64 * height as u64 > 100_000_000
        {
            return Err("CMYK image exceeds 30,000 pixels per side or 100 MP".into());
        }
        // An empty Rust slice has a non-null dangling pointer. WIC requires NULL
        // for a count-only query; otherwise it may report insufficient buffer.
        let mut count = 0;
        (frame.vtable().GetColorContexts)(frame.as_raw(), 0, std::ptr::null_mut(), &mut count)
            .ok()
            .map_err(|e| format!("Query embedded ICC count: {e}"))?;
        if count > 64 {
            return Err("Too many embedded image color profiles".into());
        }
        let mut contexts = Vec::new();
        for _ in 0..count {
            contexts.push(Some(factory.CreateColorContext().map_err(error)?));
        }
        if count > 0 {
            frame
                .GetColorContexts(&mut contexts, &mut count)
                .map_err(|e| format!("Read embedded ICC contexts: {e}"))?;
        }
        if count as usize > contexts.len() {
            return Err("Embedded ICC context count changed".into());
        }
        contexts.truncate(count as usize);
        let mut profile = None;
        for context in contexts.into_iter().flatten() {
            if context.GetType().map_err(error)? != WICColorContextProfile {
                continue;
            }
            let data = profile_bytes(&context)?;
            if data.len() >= 20 && &data[16..20] == b"CMYK" {
                profile = Some(data);
                break;
            }
        }
        let profile=match profile{Some(profile)=>Some(profile),None=>fallback()?}.ok_or("This CMYK image has no embedded ICC profile. Choose its print profile in View > CMYK Proof before importing.")?;
        let source = factory.CreateColorContext().map_err(error)?;
        source.InitializeFromMemory(&profile).map_err(error)?;
        let destination = factory.CreateColorContext().map_err(error)?;
        destination.InitializeFromMemory(srgb).map_err(error)?;
        let transform = factory.CreateColorTransformer().map_err(error)?;
        transform
            .Initialize(&frame, &source, &destination, &GUID_WICPixelFormat32bppRGBA)
            .map_err(error)?;
        let mut rgba = vec![0; width as usize * height as usize * 4];
        transform
            .CopyPixels(std::ptr::null(), width * 4, &mut rgba)
            .map_err(error)?;
        let (mut x, mut y) = (72., 72.);
        let _ = frame.GetResolution(&mut x, &mut y);
        let dpi = if x.is_finite() && (1.0..=9600.).contains(&x) {
            x
        } else {
            72.
        };
        Ok(Some(Image {
            rgba,
            width,
            height,
            profile,
            dpi,
        }))
    }
}

// The public windows-rs slice wrapper cannot express the documented NULL probe.
fn profile_bytes(context: &IWICColorContext) -> Result<Vec<u8>, String> {
    unsafe {
        let mut size = 0;
        (context.vtable().GetProfileBytes)(context.as_raw(), 0, std::ptr::null_mut(), &mut size)
            .ok()
            .map_err(|e| format!("Query embedded ICC byte size: {e}"))?;
        if size == 0 || size > 16 * 1024 * 1024 {
            return Err("Embedded ICC profile must be between 1 byte and 16 MiB".into());
        }
        let mut data = vec![0; size as usize];
        context
            .GetProfileBytes(&mut data, &mut size)
            .map_err(|e| format!("Read embedded ICC bytes: {e}"))?;
        if size as usize > data.len() {
            return Err("Embedded ICC byte size changed".into());
        }
        data.truncate(size as usize);
        Ok(data)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn profile_size_probe_and_read_preserve_exact_bytes() {
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED).ok().unwrap();
            let _apartment = Apartment;
            let factory: IWICImagingFactory =
                CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap();
            let context = factory.CreateColorContext().unwrap();
            let original = moxcms::ColorProfile::new_srgb().encode().unwrap();
            context.InitializeFromMemory(&original).unwrap();
            assert_eq!(profile_bytes(&context).unwrap(), original);
        }
    }
}
