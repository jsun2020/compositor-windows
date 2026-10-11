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
    // Long runs share one checked palette entry. Copying the initialized word
    // pattern uses bulk memory copies instead of storing each equal pixel.
    // Short or changing indices retain the word-store path.
    if depth == 1 { unsafe { decode::<1>(destination, &words, colors, indices)?; } }
    else { unsafe { decode::<2>(destination, &words, colors, indices)?; } }
    // SAFETY: every byte in the appended interval was written above, within
    // the checked reservation. The previous initialized prefix is untouched.
    unsafe { target.set_len(next); }
    Ok(())
}

/// The caller proved the complete destination interval fits reserved storage.
/// DEPTH is 1 or 2; every index is checked before a table read. Run detection
/// reads only complete eight-byte blocks and complete indices from the input.
unsafe fn decode<const DEPTH: usize>(destination: *mut u8, words: &[u32; MAX_PALETTE], colors: usize, indices: &[u8]) -> Result<(), &'static str> {
    let mut at = 0;
    while at < indices.len() {
        let index = if DEPTH == 1 { indices[at] as usize }
            else { u16::from_le_bytes([indices[at],indices[at+1]]) as usize };
        if index >= colors { return Err("pixel palette index out of range"); }
        let word = words[index];
        // A pair of equal indices is a cheap rejection for changing palettes.
        if indices.len()-at >= 32 && indices[at+DEPTH] == indices[at]
            && (DEPTH == 1 || indices[at+DEPTH+1] == indices[at+1]) {
            let pattern = if DEPTH == 1 { [indices[at];8] }
                else { [indices[at],indices[at+1],indices[at],indices[at+1],indices[at],indices[at+1],indices[at],indices[at+1]] };
            let repeated = u64::from_ne_bytes(pattern);
            // SAFETY: four blocks are inside the checked 32-byte interval.
            let same = (0..4).all(|block| unsafe { indices.as_ptr().add(at+block*8).cast::<u64>().read_unaligned() } == repeated);
            if same {
                let mut end = at+32;
                while indices.len()-end >= 8 && unsafe { indices.as_ptr().add(end).cast::<u64>().read_unaligned() } == repeated { end += 8; }
                while indices.len()-end >= DEPTH && indices[end] == indices[at]
                    && (DEPTH == 1 || indices[end+1] == indices[at+1]) { end += DEPTH; }
                let length = (end-at)/DEPTH*4;
                // SAFETY: these complete indices are within the checked output
                // interval and all equal the already validated table entry.
                unsafe { repeat_word(destination.add(at/DEPTH*4),word,length); }
                at = end;
                continue;
            }
        }
        // Keep changing indices in the original tight word-store loop. Probe
        // again after a bounded block so a later long run can still use copies.
        let length = (indices.len()-at).min(4096*DEPTH);
        // SAFETY: the complete index block maps into the checked reservation.
        unsafe { decode_words::<DEPTH>(destination.add(at/DEPTH*4),words,colors,&indices[at..at+length])?; }
        at += length;
    }
    Ok(())
}


/// Same checked table and reserved output contract as decode. The caller keeps
/// each block aligned to the index depth; palette size is bounded by MAX_PALETTE.
unsafe fn decode_words<const DEPTH: usize>(destination: *mut u8, words: &[u32; MAX_PALETTE], colors: usize, indices: &[u8]) -> Result<(), &'static str> {
    if DEPTH == 1 {
        for (i, &index) in indices.iter().enumerate() {
            let n=index as usize;
            if n>=colors {return Err("pixel palette index out of range");}
            unsafe {destination.add(i*4).cast::<u32>().write_unaligned(*words.get_unchecked(n));}
        }
    } else {
        for (i, index) in indices.chunks_exact(2).enumerate() {
            let n=u16::from_le_bytes([index[0],index[1]]) as usize;
            if n>=colors {return Err("pixel palette index out of range");}
            unsafe {destination.add(i*4).cast::<u32>().write_unaligned(*words.get_unchecked(n));}
        }
    }
    Ok(())
}

/// length is a nonzero whole-pixel interval in the caller's reservation.
/// Sources are initialized and disjoint from the destination of each copy.
unsafe fn repeat_word(destination: *mut u8, word: u32, length: usize) {
    #[cfg(target_arch = "wasm32")]
    if length >= 16 * 1024 && destination as usize % 4 == 0 {
        use wasm_bindgen::JsCast;
        // The caller checked every index in this equal run and proved the
        // complete destination interval fits the reservation. Fill the actual
        // WASM storage directly; no Rust reference exposes uninitialized bytes.
        let pattern = js_sys::Uint8Array::new_with_length(4);
        for (i, byte) in word.to_ne_bytes().into_iter().enumerate() { pattern.set_index(i as u32, byte); }
        let pixel = js_sys::Uint32Array::new(&pattern.buffer()).get_index(0);
        let memory: js_sys::WebAssembly::Memory = wasm_bindgen::memory().unchecked_into();
        // No WASM allocation occurs after taking the view. Native byte order is
        // derived from the RGBA pattern instead of assuming JS host endianness.
        let output = js_sys::Uint32Array::new_with_byte_offset_and_length(
            &memory.buffer(), destination as u32, (length / 4) as u32);
        output.fill(pixel, 0, (length / 4) as u32);
        return;
    }
    unsafe { destination.cast::<u32>().write_unaligned(word); }
    let mut written = 4;
    while written < length {
        let count = written.min(length-written);
        unsafe { std::ptr::copy_nonoverlapping(destination,destination.add(written),count); }
        written += count;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn narrow_bands_preserve_bytes_across_short_run_and_chunk_boundaries() {
        for colors in [256, 300] {
            let palette: Vec<u8> = (0..colors).flat_map(|i| [(i%256) as u8, (i/256) as u8, 73, 255]).collect();
            let depth = if colors <= 256 { 1 } else { 2 };
            let mut indices = Vec::new();
            let mut expected = vec![9; 4];
            for row in 0..7 {
                for (band, count) in [7,8,9,15,16,17,31,32,33,65].into_iter().enumerate() {
                    let color = (colors-1-row*11-band) % colors;
                    for _ in 0..count {
                        if depth == 1 { indices.push(color as u8); }
                        else { indices.extend_from_slice(&(color as u16).to_le_bytes()); }
                        expected.extend_from_slice(&palette[color*4..color*4+4]);
                    }
                }
            }
            let mut output = Vec::with_capacity(expected.len());
            output.extend_from_slice(&[9; 4]);
            for chunk in indices.chunks(257*depth) { append(&mut output, expected.len(), &palette, chunk).unwrap(); }
            assert_eq!(output, expected);
        }
    }
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

    #[test] fn long_and_short_runs_preserve_bytes_in_both_index_depths() {
        for colors in [4,300] {
            let palette:Vec<u8>=(0..colors).flat_map(|i|[(i%256) as u8,(i/256) as u8,17,255]).collect();
            let runs=[(colors-1,2051),(0,7),(1,32),(2,3),(colors-1,4097)];
            let mut indices=Vec::new();let mut expected=Vec::new();
            for (index,count) in runs {
                for _ in 0..count {
                    if colors<=256 {indices.push(index as u8);} else {indices.extend_from_slice(&(index as u16).to_le_bytes());}
                    expected.extend_from_slice(&palette[index*4..index*4+4]);
                }
            }
            let depth=if colors<=256 {1}else{2};let mut out=Vec::with_capacity(expected.len());
            for chunk in indices.chunks(513*depth) {append(&mut out,expected.len(),&palette,chunk).unwrap();}
            assert_eq!(out,expected);
        }
    }
    #[test] fn an_invalid_index_after_a_long_run_never_advances_the_initialized_length() {
        for colors in [2,257] {
            let palette=vec![13;colors*4];let depth=if colors<=256 {1}else{2};
            let mut indices=vec![0;2051*depth];
            if depth==1 {indices.push(colors as u8);} else {indices.extend_from_slice(&(colors as u16).to_le_bytes());}
            let limit=4+2052*4;let mut out=Vec::with_capacity(limit);out.extend_from_slice(&[9;4]);
            assert!(append(&mut out,limit,&palette,&indices).is_err());assert_eq!(out,[9;4]);
        }
    }

    #[test] fn changing_blocks_and_later_runs_preserve_bytes_and_reject_a_raw_block_tail() {
        for colors in [4,300] {
            let palette:Vec<u8>=(0..colors).flat_map(|i|[(i%256) as u8,(i/256) as u8,17,255]).collect();
            let depth=if colors<=256 {1}else{2};let mut indices=Vec::new();let mut expected=Vec::new();
            for i in 0..65549 {
                let n=if (4097..45069).contains(&i) {colors-1}else{i%colors};
                if depth==1 {indices.push(n as u8);}else{indices.extend_from_slice(&(n as u16).to_le_bytes());}
                expected.extend_from_slice(&palette[n*4..n*4+4]);
            }
            let mut out=Vec::with_capacity(expected.len());append(&mut out,expected.len(),&palette,&indices).unwrap();assert_eq!(out,expected);
            if depth==1 {*indices.last_mut().unwrap()=colors as u8;}else{let last=indices.len()-2;indices[last..].copy_from_slice(&(colors as u16).to_le_bytes());}
            out.clear();out.extend_from_slice(&[9;4]);out.reserve_exact(expected.len());
            assert!(append(&mut out,expected.len()+4,&palette,&indices).is_err());assert_eq!(out,[9;4]);
        }
    }
}
