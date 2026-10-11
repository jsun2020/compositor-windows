/// Lossless encoding is offered only when every stored premultiplied pixel agrees.
pub(crate) fn color(bytes: &[u8]) -> Option<[u8; 4]> {
    if bytes.is_empty() || bytes.len() % 4 != 0 { return None; }
    let first: [u8; 4] = bytes[..4].try_into().ok()?;
    bytes.chunks_exact(4).all(|p| p == first).then_some(first)
}

/// Append into already reserved storage without first zeroing it. The browser
/// fills aligned words directly; other targets retain initialized doubling.
pub(crate) fn append(target: &mut Vec<u8>, expected: usize, pixel: [u8; 4], length: usize) -> Result<(), &'static str> {
    let start = target.len();
    let next = start.checked_add(length).filter(|&n| n <= expected && n <= target.capacity())
        .ok_or("staged buffer overflow")?;
    if start % 4 != 0 || length % 4 != 0 { return Err("incomplete uniform pixel"); }
    if length == 0 { return Ok(()); }
    // SAFETY: reservation contains [start,next). The browser fills the complete
    // interval before exposing its length. The fallback copies the first pixel,
    // then only initialized prefixes into disjoint spare capacity.
    unsafe {
        let begin = target.as_mut_ptr().add(start);
        #[cfg(target_arch = "wasm32")]
        if begin as usize % 4 == 0 {
            use wasm_bindgen::JsCast;
            // Construct the word from byte storage so JS native endianness
            // cannot reorder RGBA. No Rust reference covers uninitialized data.
            let pattern = js_sys::Uint8Array::new_with_length(4);
            for (i, &byte) in pixel.iter().enumerate() { pattern.set_index(i as u32, byte); }
            let word = js_sys::Uint32Array::new(&pattern.buffer()).get_index(0);
            let memory: js_sys::WebAssembly::Memory = wasm_bindgen::memory().unchecked_into();
            // The interval is checked above and aligned. No WASM allocation or
            // callback occurs between taking this memory view and filling it.
            // Native TypedArray.fill writes each word once, instead of reading
            // and copying growing prefixes through the WASM memcpy loop.
            let words = js_sys::Uint32Array::new_with_byte_offset_and_length(
                &memory.buffer(), begin as u32, (length / 4) as u32);
            words.fill(word, 0, (length / 4) as u32);
            target.set_len(next);
            return Ok(());
        }
        std::ptr::copy_nonoverlapping(pixel.as_ptr(), begin, 4);
        let mut written = 4;
        while written < length {
            let count = written.min(length - written);
            std::ptr::copy_nonoverlapping(begin, begin.add(written), count);
            written += count;
        }
        target.set_len(next);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_encoding_checks_every_channel_and_final_pixel() {
        let mut bytes = [17, 31, 43, 97].repeat(1031);
        assert_eq!(color(&bytes), Some([17, 31, 43, 97]));
        for c in 0..4 { let at=bytes.len()-4+c; bytes[at]^=1; assert_eq!(color(&bytes),None); bytes[at]^=1; }
        assert_eq!(color(&[]),None); assert_eq!(color(&bytes[..bytes.len()-1]),None);
    }
    #[test]
    fn expansion_preserves_all_bytes_across_non_power_of_two_chunks() {
        let expected=4*(1024*1024+13);let mut out=Vec::with_capacity(expected);
        for n in [4, 4*1024*1024, 48] { append(&mut out,expected,[17,31,43,97],n).unwrap(); }
        assert_eq!(out,[17,31,43,97].repeat(expected/4));
        let before=out.clone();assert!(append(&mut out,expected,[0;4],4).is_err());assert_eq!(out,before);
    }
    #[test]
    fn partial_pixels_overflow_and_insufficient_capacity_leave_storage_unchanged() {
        let mut out=Vec::with_capacity(16);out.extend_from_slice(&[9;4]);
        for (limit,n) in [(16,3),(16,usize::MAX),(8,8),(32,20)] {
            assert!(append(&mut out,limit,[1,2,3,4],n).is_err()); assert_eq!(out,[9;4]);
        }
        append(&mut out,16,[1,2,3,4],0).unwrap();assert_eq!(out,[9;4]);
    }
}
