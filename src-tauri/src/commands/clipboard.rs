//! Win32 image clipboard. PNG retains alpha; DIBV5 supports Paint and other
//! applications that request a standard bitmap format. Global handles are freed
//! here only before SetClipboardData transfers ownership to Windows.
use tauri::ipc::{InvokeBody, Request, Response};

const MAX_BYTES: usize = 512 * 1024 * 1024;

fn pixel_origin(headers: &tauri::http::HeaderMap) -> Result<[f64; 2], String> {
    // HTTP Origin belongs to the browser, which supplies the WebView origin.
    let origin: [f64; 2] = serde_json::from_str(headers.get("compositor-pixel-origin").and_then(|v| v.to_str().ok()).ok_or("missing clipboard origin")?).map_err(|_| "Invalid clipboard origin")?;
    if origin.iter().any(|v| !v.is_finite() || v.abs() > 1_000_000.0) { return Err("Invalid clipboard origin".into()); }
    Ok(origin)
}

#[cfg(windows)]
mod win {
    use super::MAX_BYTES;
    use std::{ffi::c_void, ptr, time::{Duration, Instant}};
    type Handle = *mut c_void;
    #[link(name = "user32")]
    extern "system" {
        fn OpenClipboard(window: Handle) -> i32;
        fn CloseClipboard() -> i32;
        fn EmptyClipboard() -> i32;
        fn GetClipboardData(format: u32) -> Handle;
        fn SetClipboardData(format: u32, handle: Handle) -> Handle;
        fn RegisterClipboardFormatW(name: *const u16) -> u32;
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GlobalAlloc(flags: u32, size: usize) -> Handle;
        fn GlobalLock(handle: Handle) -> Handle;
        fn GlobalUnlock(handle: Handle) -> i32;
        fn GlobalSize(handle: Handle) -> usize;
        fn GlobalFree(handle: Handle) -> Handle;
        fn GetLastError() -> u32;
    }
    fn format(name: &str) -> Result<u32, String> {
        let wide: Vec<_> = name.encode_utf16().chain(Some(0)).collect();
        let value = unsafe { RegisterClipboardFormatW(wide.as_ptr()) };
        if value == 0 { Err("Cannot register the image clipboard format".into()) } else { Ok(value) }
    }
    struct Open;
    impl Open {
        fn new(window: Handle) -> Result<Self, String> {
            // Clipboard listeners can briefly own the lock immediately after
            // Copy. ERROR_ACCESS_DENIED also describes that normal contention;
            // it does not establish a non-interactive desktop. Wait only for
            // acquisition, before any data access or mutation, on this blocking
            // worker. A persistent denial still fails within a bounded interval.
            let deadline = Instant::now() + Duration::from_millis(250);
            loop {
                if unsafe { OpenClipboard(window) } != 0 { return Ok(Self); }
                let error = unsafe { GetLastError() };
                let now = Instant::now();
                if !matches!(error, 0 | 5 | 170) || now >= deadline {
                    return Err("Windows clipboard is busy or unavailable. Please try again.".into());
                }
                std::thread::sleep(Duration::from_millis(10).min(deadline - now));
            }
        }
    }
    impl Drop for Open { fn drop(&mut self) { unsafe { CloseClipboard(); } } }
    struct Memory(Handle);
    impl Memory {
        fn new(bytes: &[u8]) -> Result<Self, String> {
            let handle = unsafe { GlobalAlloc(0x0002, bytes.len()) };
            if handle.is_null() { return Err("Cannot allocate clipboard memory".into()); }
            let mem = Self(handle);
            let p = unsafe { GlobalLock(handle) };
            if p.is_null() { return Err("Cannot lock clipboard memory".into()); }
            unsafe { ptr::copy_nonoverlapping(bytes.as_ptr(), p as *mut u8, bytes.len()); GlobalUnlock(handle); }
            Ok(mem)
        }
        fn give(mut self, format: u32) -> Result<(), String> {
            if unsafe { SetClipboardData(format, self.0) }.is_null() { return Err("Cannot write the image to the clipboard".into()); }
            self.0 = ptr::null_mut(); Ok(())
        }
    }
    impl Drop for Memory { fn drop(&mut self) { if !self.0.is_null() { unsafe { GlobalFree(self.0); } } } }
    fn read(format: u32) -> Result<Option<Vec<u8>>, String> {
        let handle = unsafe { GetClipboardData(format) };
        if handle.is_null() { return Ok(None); }
        let size = unsafe { GlobalSize(handle) };
        if size == 0 || size > MAX_BYTES { return Err("The clipboard image is too large or unreadable".into()); }
        let p = unsafe { GlobalLock(handle) };
        if p.is_null() { return Err("Cannot read the clipboard image".into()); }
        let out = unsafe { std::slice::from_raw_parts(p as *const u8, size).to_vec() };
        unsafe { GlobalUnlock(handle); }
        Ok(Some(out))
    }
    pub fn write(window: isize, png: &[u8], origin: [f64; 2], token: Option<String>) -> Result<(), String> {
        let raster = compositor_engine::decode_image(png).map_err(|e| e.to_string())?.raster;
        let dib = super::dib_v5(&raster);
        // Allocate every payload before emptying the clipboard. A failure in
        // decoding or allocation leaves the previous clipboard intact.
        let png_mem = Memory::new(png)?;
        let dib_mem = Memory::new(&dib)?;
        let origin_bytes = [origin[0].to_le_bytes(), origin[1].to_le_bytes()].concat();
        let origin_mem = Memory::new(&origin_bytes)?;
        let token_mem = token.as_ref().map(|s|Memory::new(s.as_bytes())).transpose()?;
        let png_format = format("PNG")?;
        let origin_format = format("Compositor.PixelOrigin.v1")?;
        let token_format = format("Compositor.LayerCopy.v1")?;
        let _open = Open::new(window as Handle)?;
        if unsafe { EmptyClipboard() } == 0 { return Err("Cannot clear the clipboard".into()); }
        png_mem.give(png_format)?;
        // Once PNG succeeded the image is safely available for Cut. Auxiliary
        // format failure must not turn a successful write into lost pixels.
        let _ = dib_mem.give(17);
        let _ = origin_mem.give(origin_format);
        if let Some(m)=token_mem {let _=m.give(token_format);}
        Ok(())
    }
    pub fn image(window: isize) -> Result<Vec<u8>, String> {
        let png_format = format("PNG")?;
        let origin_format = format("Compositor.PixelOrigin.v1")?;
        let token_format = format("Compositor.LayerCopy.v1")?;
        let _open = Open::new(window as Handle)?;
        let png = read(png_format)?;
        let (image, origin) = match png {
            Some(image) => (image, read(origin_format)?.filter(|b| b.len() >= 16)),
            None => {
                let dib = read(17)?.or(read(8)?).ok_or("The clipboard does not contain an image")?;
                (super::dib_to_bmp(&dib)?, None)
            }
        };
        // One byte marks an internal origin; two little-endian f64s follow.
        // Image data travels as a raw IPC response, never a JSON number array.
        let mut out = Vec::with_capacity(image.len() + 17);
        out.push(u8::from(origin.is_some()));
        out.extend_from_slice(origin.as_ref().map_or(&[0; 16][..], |b| &b[..16]));
        let token=if origin.is_some(){read(token_format)?.filter(|b|b.len()>=36)}else{None};
        out.extend_from_slice(token.as_ref().map_or(&[0;36][..],|b|&b[..36]));
        out.extend_from_slice(&image); Ok(out)
    }
}

fn dib_v5(r: &compositor_engine::Raster) -> Vec<u8> {
    let mut out = vec![0; 124];
    fn put(out: &mut [u8], at: usize, value: u32) { out[at..at + 4].copy_from_slice(&value.to_le_bytes()); }
    put(&mut out, 0, 124); put(&mut out, 4, r.width);
    put(&mut out, 8, (-(r.height as i32)) as u32); // top-down
    out[12..14].copy_from_slice(&1u16.to_le_bytes()); out[14..16].copy_from_slice(&32u16.to_le_bytes());
    put(&mut out, 16, 3); put(&mut out, 20, r.width * r.height * 4);
    for (at, value) in [(40, 0x00ff0000), (44, 0x0000ff00), (48, 0x000000ff), (52, 0xff000000), (56, 0x73524742), (108, 4)] { put(&mut out, at, value); }
    for p in r.to_straight().chunks_exact(4) { out.extend_from_slice(&[p[2], p[1], p[0], p[3]]); }
    out
}

fn dib_to_bmp(dib: &[u8]) -> Result<Vec<u8>, String> {
    fn u32_at(b: &[u8], n: usize) -> u32 { u32::from_le_bytes(b[n..n + 4].try_into().unwrap()) }
    if dib.len() < 40 || dib.len() > MAX_BYTES { return Err("Unsupported clipboard bitmap".into()); }
    let header = u32_at(dib, 0) as usize;
    if ![40, 108, 124].contains(&header) || dib.len() < header { return Err("Unsupported clipboard bitmap header".into()); }
    let bits = u16::from_le_bytes(dib[14..16].try_into().unwrap());
    let compression = u32_at(dib, 16);
    if ![24, 32].contains(&bits) || ![0, 3, 6].contains(&compression) { return Err("Unsupported clipboard bitmap format".into()); }
    let masks = if header == 40 { match compression { 3 => 12, 6 => 16, _ => 0 } } else { 0 };
    let palette = (u32_at(dib, 32) as usize).checked_mul(4).ok_or("Invalid bitmap palette")?;
    let mut offset = header.checked_add(masks).and_then(|n| n.checked_add(palette)).ok_or("Invalid bitmap offset")?;
    if offset > dib.len() { return Err("Truncated clipboard bitmap".into()); }
    // Windows can synthesize CF_DIBV5 from CF_BITMAP with an additional RGB
    // mask table in bmiColors, even though the same masks are in the V5 header.
    // A BMP file instead identifies its pixel start explicitly. Recognize that
    // table by its exact masks and the complete uncompressed pixel span; do not
    // skip bytes in ordinary V4/V5 bitmaps with only embedded masks.
    if header >= 108 && compression == 3 && palette == 0 {
        let width = u32_at(dib, 4) as usize;
        let height = (u32_at(dib, 8) as i32).unsigned_abs() as usize;
        let span = width.checked_mul(bits as usize).and_then(|n|n.checked_add(31)).map(|n|n/32*4).and_then(|stride|stride.checked_mul(height)).ok_or("Invalid bitmap size")?;
        if span > 0 && offset.checked_add(12).and_then(|n|n.checked_add(span)) == Some(dib.len()) && dib[offset..offset+12] == dib[40..52] { offset += 12; }
    }
    let mut out = Vec::with_capacity(dib.len() + 14);
    out.extend_from_slice(b"BM"); out.extend_from_slice(&((dib.len() + 14) as u32).to_le_bytes());
    out.extend_from_slice(&[0; 4]); out.extend_from_slice(&((offset + 14) as u32).to_le_bytes()); out.extend_from_slice(dib);
    Ok(out)
}

#[tauri::command]
pub async fn write_clipboard_image(window: tauri::WebviewWindow, request: Request<'_>) -> Result<(), String> {
    let bytes = match request.body() { InvokeBody::Raw(b) if b.len() <= MAX_BYTES => b.clone(), _ => return Err("Invalid clipboard image payload".into()) };
    let origin = pixel_origin(request.headers())?;
    let token=request.headers().get("layer-token").and_then(|v|v.to_str().ok()).map(str::to_owned);
    if token.as_ref().is_some_and(|t|t.len()!=36||!t.bytes().all(|c|c.is_ascii_hexdigit()||c==b'-')){return Err("Invalid clipboard token".into());}
    #[cfg(windows)] {
        let hwnd = window.hwnd().map_err(|e| e.to_string())?.0 as isize;
        tauri::async_runtime::spawn_blocking(move || win::write(hwnd, &bytes, origin, token)).await.map_err(|e| e.to_string())?
    }
    #[cfg(not(windows))] { let _ = (window, bytes); Err("System image clipboard is available on Windows".into()) }
}

#[tauri::command]
pub async fn read_clipboard_image(window: tauri::WebviewWindow) -> Result<Response, String> {
    #[cfg(windows)] {
        let hwnd = window.hwnd().map_err(|e| e.to_string())?.0 as isize;
        let bytes = tauri::async_runtime::spawn_blocking(move || win::image(hwnd)).await.map_err(|e| e.to_string())??;
        Ok(Response::new(bytes))
    }
    #[cfg(not(windows))] { let _ = window; Err("System image clipboard is available on Windows".into()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pixel_coordinates_do_not_use_the_browser_origin_header() {
        let mut headers = tauri::http::HeaderMap::new();
        headers.insert("origin", "http://tauri.localhost".parse().unwrap());
        assert!(pixel_origin(&headers).is_err());
        headers.insert("compositor-pixel-origin", "[318,239]".parse().unwrap());
        assert_eq!(pixel_origin(&headers).unwrap(), [318.0, 239.0]);
        for value in ["[1000001,0]", "[1]", "[null,0]", "http://tauri.localhost"] {
            headers.insert("compositor-pixel-origin", value.parse().unwrap());
            assert!(pixel_origin(&headers).is_err());
        }
    }
    #[test]
    fn dib_preserves_alpha_and_channel_order() {
        let r = compositor_engine::Raster::from_straight(2, 1, &[255, 0, 0, 128, 0, 40, 255, 255]);
        let dib = dib_v5(&r);
        assert_eq!(&dib[124..], &[0, 0, 255, 128, 255, 40, 0, 255]);
        let bmp = dib_to_bmp(&dib).unwrap();
        let decoded = compositor_engine::decode_image(&bmp).unwrap().raster;
        assert_eq!(decoded.bytes(), r.bytes());
    }
    #[test]
    fn dib_v5_preserves_both_row_directions_and_alpha() {
        let r = compositor_engine::Raster::from_straight(2, 2, &[255,0,0,128, 0,255,0,255, 0,0,255,255, 255,255,255,64]);
        let mut dib = dib_v5(&r);
        assert_eq!(compositor_engine::decode_image(&dib_to_bmp(&dib).unwrap()).unwrap().raster.bytes(),r.bytes());
        dib[8..12].copy_from_slice(&2i32.to_le_bytes());
        let rows=dib[124..].to_vec();dib[124..132].copy_from_slice(&rows[8..]);dib[132..].copy_from_slice(&rows[..8]);
        assert_eq!(compositor_engine::decode_image(&dib_to_bmp(&dib).unwrap()).unwrap().raster.bytes(),r.bytes());
    }
    #[test]
    fn windows_synthesized_v5_mask_table_is_not_read_as_a_pixel_row() {
        let r = compositor_engine::Raster::from_straight(3,2,&[255,0,0,255, 0,255,0,255, 0,0,255,255, 255,255,255,255, 128,0,128,255, 0,0,0,255]);
        let mut dib=dib_v5(&r);dib[8..12].copy_from_slice(&2i32.to_le_bytes());dib[52..56].fill(0);
        let rows=dib[124..].to_vec();dib.truncate(124);let masks=dib[40..52].to_vec();
        dib.extend_from_slice(&masks);dib.extend_from_slice(&rows[12..]);dib.extend_from_slice(&rows[..12]);
        let bmp=dib_to_bmp(&dib).unwrap();assert_eq!(u32::from_le_bytes(bmp[10..14].try_into().unwrap()),150);
        assert_eq!(compositor_engine::decode_image(&bmp).unwrap().raster.bytes(),r.bytes());
    }
    #[test]
    fn rejects_truncated_and_unsupported_headers() {
        assert!(dib_to_bmp(&[0; 39]).is_err());
        let mut dib = vec![0; 40]; dib[..4].copy_from_slice(&124u32.to_le_bytes());
        assert!(dib_to_bmp(&dib).is_err());
        dib[..4].copy_from_slice(&40u32.to_le_bytes()); dib[14..16].copy_from_slice(&8u16.to_le_bytes());
        assert!(dib_to_bmp(&dib).is_err());
    }
}
