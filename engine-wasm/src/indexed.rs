pub(crate) const MAX_PALETTE: usize = 1024;

/// Decode exact stored RGBA bytes directly into reserved output storage. Index
/// planes use one byte through 256 colours and little-endian u16 above that.
pub(crate) fn append(target: &mut Vec<u8>, expected: usize, palette: &[u8], indices: &[u8]) -> Result<(), &'static str> {
    if palette.is_empty() || palette.len() % 4 != 0 || palette.len() > MAX_PALETTE * 4 { return Err("invalid pixel palette"); }
    let colors = palette.len() / 4;
    let depth = if colors <= 256 { 1 } else { 2 };
    if indices.len() % depth != 0 || target.len() % 4 != 0 { return Err("incomplete palette index"); }
    let length = (indices.len() / depth).checked_mul(4).ok_or("staged buffer overflow")?;
    let next = target.len().checked_add(length).filter(|&n| n <= expected && n <= target.capacity()).ok_or("staged buffer overflow")?;
    // No uninitialized byte is read or exposed. A rejected index leaves the
    // length unchanged; the caller cancels the complete staging reservation.
    let mut words = [0u32; MAX_PALETTE];
    for (out, color) in words.iter_mut().zip(palette.chunks_exact(4)) {
        *out = u32::from_ne_bytes([color[0],color[1],color[2],color[3]]);
    }
    // SAFETY: `length` fits the reserved spare capacity, checked above. Each
    // decoded index is checked before reading the bounded table; unaligned
    // word stores preserve all four bytes even on a big-endian native host.
    let destination = unsafe { target.as_mut_ptr().add(target.len()) };
    if depth == 1 {
        for (i, &index) in indices.iter().enumerate() {
            let n = index as usize;
            if n >= colors { return Err("pixel palette index out of range"); }
            unsafe { destination.add(i*4).cast::<u32>().write_unaligned(*words.get_unchecked(n)); }
        }
    } else {
        for (i, index) in indices.chunks_exact(2).enumerate() {
            let n = u16::from_le_bytes([index[0],index[1]]) as usize;
            if n >= colors { return Err("pixel palette index out of range"); }
            unsafe { destination.add(i*4).cast::<u32>().write_unaligned(*words.get_unchecked(n)); }
        }
    }
    // SAFETY: every byte in the appended interval was written above, within
    // the checked reservation. The previous initialized prefix is untouched.
    unsafe { target.set_len(next); }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn byte_indices_preserve_all_channels_and_chunk_boundaries() {
        let palette=[17,31,43,97, 0,0,0,0, 91,37,13,255];
        let mut out=Vec::with_capacity(24);
        append(&mut out,24,&palette,&[2,0,1]).unwrap(); append(&mut out,24,&palette,&[0,2,1]).unwrap();
        assert_eq!(out,[91,37,13,255,17,31,43,97,0,0,0,0,17,31,43,97,91,37,13,255,0,0,0,0]);
    }
    #[test] fn wide_indices_use_little_endian_without_truncating_color_numbers() {
        let palette:Vec<u8>=(0..MAX_PALETTE).flat_map(|i|[(i%256) as u8,(i/256) as u8,17,255]).collect();
        let mut out=Vec::with_capacity(16);
        append(&mut out,16,&palette,&[0,0, 255,0, 0,1, 255,3]).unwrap();
        assert_eq!(out,[0,0,17,255,255,0,17,255,0,1,17,255,255,3,17,255]);
    }
    #[test] fn malformed_input_never_exposes_partial_output() {
        let mut out=Vec::with_capacity(32);out.extend_from_slice(&[9;4]);
        for palette in [vec![],vec![0;3],vec![0;(MAX_PALETTE+1)*4]] { assert!(append(&mut out,32,&palette,&[0]).is_err()); assert_eq!(out,[9;4]); }
        assert!(append(&mut out,32,&[1,2,3,4],&[0,1]).is_err());assert_eq!(out,[9;4]);
        let wide=vec![0;257*4];assert!(append(&mut out,32,&wide,&[0]).is_err());assert_eq!(out,[9;4]);
    }
    #[test] fn reservation_and_expected_size_are_checked_before_writing() {
        let mut out=Vec::with_capacity(8);out.extend_from_slice(&[9;4]);
        assert!(append(&mut out,4,&[1,2,3,4],&[0]).is_err());assert_eq!(out,[9;4]);
        assert!(append(&mut out,16,&[1,2,3,4],&[0,0]).is_err());assert_eq!(out,[9;4]);
        append(&mut out,8,&[1,2,3,4],&[0]).unwrap();assert_eq!(out,[9,9,9,9,1,2,3,4]);
    }
}
