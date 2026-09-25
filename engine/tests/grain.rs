use compositor_engine::*;

fn gray(w: u32, h: u32, alpha: u8) -> Raster {
    let v = (128u32 * alpha as u32 / 255) as u8;
    Raster::from_premultiplied(w, h, [v, v, v, alpha].repeat((w * h) as usize))
}
fn at(raster: &Raster, x: u32, y: u32) -> [u8; 4] { raster.pixel(x, y) }

#[test]
fn grain_is_fixed_in_document_space_and_leaves_transparency_alone() {
    let s = GrainSettings { amount: 60.0, size: 2.0, roughness: 40.0, seed: 7 };
    let whole = apply_grain(&gray(40, 40, 255), &s, Point { x: 0.0, y: 0.0 }, 1.0);
    let values: std::collections::HashSet<u8> = whole.bytes().chunks_exact(4).map(|p| p[0]).collect();
    assert!(values.len() > 5, "grain varies the brightness");
    assert!(whole.bytes().chunks_exact(4).all(|p| p[0] == p[1] && p[1] == p[2]), "the same change on every channel");
    // A 20 x 20 piece drawn at its place in the document gets that part of the same pattern.
    let part = apply_grain(&gray(20, 20, 255), &s, Point { x: 10.0, y: 10.0 }, 1.0);
    for y in 0..20 { for x in 0..20 { assert_eq!(at(&part, x, y), at(&whole, x + 10, y + 10), "({x}, {y})"); } }
    let reseeded = apply_grain(&gray(40, 40, 255), &GrainSettings { seed: 8, ..s }, Point { x: 0.0, y: 0.0 }, 1.0);
    assert_ne!(reseeded.bytes(), whole.bytes(), "another seed, another pattern");
    let none = apply_grain(&gray(4, 4, 255), &GrainSettings { amount: 0.0, ..s }, Point { x: 0.0, y: 0.0 }, 1.0);
    assert_eq!(none.bytes(), gray(4, 4, 255).bytes(), "no amount, no change");
    let clear = apply_grain(&gray(4, 4, 0), &s, Point { x: 0.0, y: 0.0 }, 1.0);
    assert!(clear.bytes().chunks_exact(4).all(|p| p[3] == 0 && p[0] == 0), "clear pixels stay clear");
}

#[test]
fn grain_is_strongest_in_the_midtones_and_bounded_by_its_amount() {
    let spread = |r: &Raster| -> i64 {
        let v: Vec<i64> = r.bytes().chunks_exact(4).map(|p| p[0] as i64).collect();
        v.iter().max().unwrap() - v.iter().min().unwrap()
    };
    let flat = |v: u8| Raster::from_premultiplied(24, 24, [v, v, v, 255].repeat(576));
    // Amount 20 keeps every delta well inside 0..255 for both tones (at most about 36 levels at
    // the midtone), so no clamp shapes the spreads: they differ only by grain_weight. On black the
    // negative deltas clamp away, which would halve the spread whatever the weight did.
    let gentle = GrainSettings { amount: 20.0, size: 1.5, roughness: 50.0, seed: 3 };
    let mid = spread(&apply_grain(&flat(128), &gentle, Point { x: 0.0, y: 0.0 }, 1.0));
    let dark = spread(&apply_grain(&flat(40), &gentle, Point { x: 0.0, y: 0.0 }, 1.0));
    let want = grain_weight(128.0 / 255.0) / grain_weight(40.0 / 255.0);
    assert!(mid > dark + 5 && ((mid as f32 / dark as f32) - want).abs() < 0.1,
        "midtones take more grain than a darker tone, by the weight ratio {want}: {mid} vs {dark}");
    // strength is amount/100 * 0.35 * 255, and the midtone weight peaks at 1.0.
    let s = GrainSettings { amount: 100.0, ..gentle };
    let full = spread(&apply_grain(&gray(24, 24, 255), &s, Point { x: 0.0, y: 0.0 }, 1.0));
    assert!(full > 20, "grain at full amount is clearly visible: {full}");
    assert!((grain_weight(0.5) - 1.0).abs() < 1e-6 && (grain_weight(0.0) - 0.4).abs() < 1e-6);
}

/// AdjustPixels.c:31-60 transcribed independently of grain.rs.
fn c_mix32(mut x: u32) -> u32 {
    x ^= x >> 16; x = x.wrapping_mul(0x7feb_352d); x ^= x >> 15; x = x.wrapping_mul(0x846c_a68b); x ^= x >> 16; x
}
fn c_lattice(ix: i64, iy: i64, seed: u32) -> f32 {
    let h = c_mix32((ix as u32).wrapping_mul(0x9E37_79B1) ^ c_mix32((iy as u32).wrapping_mul(0x85EB_CA77) ^ seed));
    (h & 0xFFFF) as f32 / 65535.0 + (h >> 16) as f32 / 65535.0 - 1.0
}
fn c_grain_field(u: f64, v: f64, scale: f64, seed: u32) -> f32 {
    let (cell_x, cell_y) = ((u / scale).floor(), (v / scale).floor());
    let (mut tx, mut ty) = ((u / scale - cell_x) as f32, (v / scale - cell_y) as f32);
    tx = tx * tx * (3.0 - 2.0 * tx);
    ty = ty * ty * (3.0 - 2.0 * ty);
    let (ix, iy) = (cell_x as i64, cell_y as i64);
    let (n00, n10) = (c_lattice(ix, iy, seed), c_lattice(ix + 1, iy, seed));
    let (n01, n11) = (c_lattice(ix, iy + 1, seed), c_lattice(ix + 1, iy + 1, seed));
    let top = n00 + (n10 - n00) * tx;
    let bottom = n01 + (n11 - n01) * tx;
    (top + (bottom - top) * ty) * 1.6
}

#[test]
fn the_fine_detail_is_the_same_smooth_field_at_about_a_third_of_the_size() {
    // Mac 1.2.6 (AdjustPixels.c:70-83): fine = grain_field(u, v, max(0.5, size * 0.35), mix32(seed ^ 0xA511E9B3)),
    // where the older kernel took an un-interpolated lattice value per whole pixel.
    for (size, roughness, seed) in [(1.5, 50.0, 0u32), (6.0, 100.0, 7), (0.5, 35.0, 99)] {
        let rough = (roughness / 100.0) as f32;
        let fine_seed = c_mix32(seed ^ 0xA511_E9B3);
        for i in 0..40 {
            let (u, v) = (i as f64 * 0.37 + 0.5, i as f64 * 0.61 + 0.5);
            let smooth = c_grain_field(u, v, size, seed);
            let fine = c_grain_field(u, v, f64::max(0.5, size * 0.35), fine_seed);
            assert_eq!(grain_noise(u, v, size, roughness, seed), smooth + (fine - smooth) * rough, "size {size} at ({u}, {v})");
        }
    }
}

#[test]
fn the_noise_field_is_smooth_and_repeatable() {
    let a = grain_noise(10.25, 4.75, 2.0, 40.0, 7);
    assert_eq!(a, grain_noise(10.25, 4.75, 2.0, 40.0, 7), "the same point is the same value");
    assert_ne!(a, grain_noise(10.25, 4.75, 2.0, 40.0, 8));
    assert!((-2.0..=2.0).contains(&a));
    // With no roughness the field varies smoothly inside a cell: two points a hundredth of a
    // pixel apart differ by almost nothing.
    let smooth_a = grain_noise(10.25, 4.75, 2.0, 0.0, 7);
    let smooth_b = grain_noise(10.26, 4.75, 2.0, 0.0, 7);
    assert!((smooth_a as f64 - smooth_b as f64).abs() < 0.05, "{smooth_a} vs {smooth_b}");
}
