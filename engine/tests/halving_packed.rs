use compositor_engine::Raster;

fn reference(src: &[u8], width: usize, height: usize) -> Vec<u8> {
    let (w, h) = ((width / 2).max(1), (height / 2).max(1));
    let mut out = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for x in 0..w {
            for channel in 0..4 {
                let mut values = Vec::new();
                for sy in 2 * y..(2 * y + 2).min(height) {
                    for sx in 2 * x..(2 * x + 2).min(width) {
                        values.push(src[(sy * width + sx) * 4 + channel] as u32);
                    }
                }
                let n = values.len() as u32;
                out.push(((values.iter().sum::<u32>() + n / 2) / n) as u8);
            }
        }
    }
    out
}

#[test]
fn reductions_match_independent_box_averages_through_odd_and_thin_grids() {
    let sizes = [1, 2, 3, 4, 7, 16, 31, 32, 33, 65];
    let mut seed = 0x9e3779b9u32;
    for width in sizes {
        for height in sizes {
            let mut bytes = Vec::with_capacity(width * height * 4);
            for _ in 0..width * height {
                seed ^= seed << 13; seed ^= seed >> 17; seed ^= seed << 5;
                let alpha = (seed >> 24) as u8;
                for shift in [0, 8, 16] {
                    bytes.push(((seed >> shift) % (alpha as u32 + 1)) as u8);
                }
                bytes.push(alpha);
            }
            let mut raster = Raster::from_premultiplied(width as u32, height as u32, bytes.clone());
            let (mut w, mut h) = (width, height);
            while w > 1 && h > 1 {
                bytes = reference(&bytes, w, h);
                raster = raster.halved();
                w /= 2; h /= 2;
                assert_eq!(raster.bytes(), bytes, "{width}x{height} reduced to {w}x{h}");
            }
            let thin = raster.halved();
            assert_eq!(thin.bytes(), reference(&bytes, w, h));
        }
    }
}

#[test]
fn channel_lanes_keep_carries_and_half_up_rounding_separate() {
    for a in 0..=255u32 {
        for b in 0..=255u32 {
            let c = (a * 73 + b * 31) & 255;
            let d = a ^ b;
            let bytes = [a, b, c, 255, b, c, d, 255,
                c, d, a, 255, d, a, b, 255].map(|v| v as u8);
            let raster = Raster::from_premultiplied(2, 2, bytes.to_vec());
            assert_eq!(raster.halved().bytes(), reference(&bytes, 2, 2));
        }
    }
}
