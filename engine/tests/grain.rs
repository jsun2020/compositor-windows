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
    let s = GrainSettings { amount: 100.0, size: 1.5, roughness: 50.0, seed: 3 };
    let mid = apply_grain(&gray(24, 24, 255), &s, Point { x: 0.0, y: 0.0 }, 1.0);
    let black = apply_grain(&Raster::from_premultiplied(24, 24, [0, 0, 0, 255].repeat(576)), &s, Point { x: 0.0, y: 0.0 }, 1.0);
    let spread = |r: &Raster| -> i64 {
        let v: Vec<i64> = r.bytes().chunks_exact(4).map(|p| p[0] as i64).collect();
        v.iter().max().unwrap() - v.iter().min().unwrap()
    };
    assert!(spread(&mid) > spread(&black), "midtones take more grain than the shadows: {} vs {}", spread(&mid), spread(&black));
    // strength is amount/100 * 0.35 * 255, and the midtone weight peaks at 1.0.
    assert!(spread(&mid) > 20, "grain at full amount is clearly visible: {}", spread(&mid));
    assert!((grain_weight(0.5) - 1.0).abs() < 1e-6 && (grain_weight(0.0) - 0.4).abs() < 1e-6);
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
