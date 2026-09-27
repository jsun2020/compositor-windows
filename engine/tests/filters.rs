use compositor_engine::*;

/// `width` x `height`, opaque white in the left `solid` columns, transparent elsewhere.
fn half(width: u32, height: u32, solid: u32) -> Raster {
    let mut data = vec![0u8; (width * height * 4) as usize];
    for y in 0..height { for x in 0..solid {
        let i = ((y * width + x) * 4) as usize;
        data[i..i + 4].copy_from_slice(&[255, 255, 255, 255]);
    }}
    Raster::from_premultiplied(width, height, data)
}
fn alpha(raster: &Raster, x: u32, y: u32) -> i64 { raster.pixel(x, y)[3] as i64 }

#[test]
fn a_gaussian_blur_softens_a_hard_edge_and_fades_into_empty_space() {
    let blurred = gaussian_blur(&half(40, 20, 20), 3.0);
    assert!(alpha(&blurred, 20, 10) > 20 && alpha(&blurred, 20, 10) < 235, "the hard edge is now soft");
    assert!(alpha(&blurred, 24, 10) > 0, "it spreads into the empty half");
    assert!(alpha(&blurred, 38, 10) == 0, "but not six sigma away");
    assert!(alpha(&blurred, 0, 10) > 100 && alpha(&blurred, 0, 10) < 160, "the raster's own border fades: nothing lies beyond it");
    assert_eq!(gaussian_blur(&half(8, 8, 4), 0.0).bytes(), half(8, 8, 4).bytes(), "no sigma, no change");
}

#[test]
fn motion_blur_streaks_along_its_angle_counterclockwise_from_horizontal() {
    // One opaque dot in the middle of a transparent raster.
    let mut data = vec![0u8; 41 * 41 * 4];
    let i = ((20 * 41 + 20) * 4) as usize;
    data[i..i + 4].copy_from_slice(&[255, 255, 255, 255]);
    let dot = Raster::from_premultiplied(41, 41, data);
    let horizontal = motion_blur(&dot, 0.0, 16.0);
    assert!(alpha(&horizontal, 24, 20) > 0 && alpha(&horizontal, 16, 20) > 0 && alpha(&horizontal, 20, 24) == 0);
    let vertical = motion_blur(&dot, 90.0, 16.0);
    assert!(alpha(&vertical, 20, 24) > 0 && alpha(&vertical, 20, 16) > 0 && alpha(&vertical, 24, 20) == 0);
    // 45 degrees runs up-right and down-left on screen, never up-left.
    let diagonal = motion_blur(&dot, 45.0, 16.0);
    assert!(alpha(&diagonal, 23, 17) > 0 && alpha(&diagonal, 17, 23) > 0 && alpha(&diagonal, 17, 17) == 0);
    // CIMotionBlur's taper (probe results): along the row the dot's alpha follows a Gaussian of
    // sigma 16 / sqrt(12), cut at ceil(3 sigma) = 14, each column its tap's share of the weights.
    // An even 16-px streak would give 16 columns of 16 each.
    let sigma = 16.0 / 12f64.sqrt();
    let weight = |t: i64| (-((t * t) as f64) / (2.0 * sigma * sigma)).exp();
    let total: f64 = (-14..=14).map(weight).sum();
    for x in 0..41i64 {
        let want = if (x - 20).abs() <= 14 { 255.0 * weight(x - 20) / total } else { 0.0 };
        assert!((alpha(&horizontal, x as u32, 20) as f64 - want).abs() <= 1.0, "column {x}: {} vs {want:.2}", alpha(&horizontal, x as u32, 20));
    }
}

#[test]
fn add_noise_changes_color_but_never_alpha_and_monochromatic_keeps_grays() {
    let gray = Raster::from_premultiplied(32, 8, {
        let mut d = vec![0u8; 32 * 8 * 4];
        for y in 0..8 { for x in 0..16 { let i = ((y * 32 + x) * 4) as usize; d[i..i + 4].copy_from_slice(&[128, 128, 128, 255]); } }
        d
    });
    let color = add_noise(&gray, 10.0, false, false, 7);
    assert_eq!(color.bytes(), add_noise(&gray, 10.0, false, false, 7).bytes(), "the same seed gives the same grain");
    let opaque: Vec<[u8; 4]> = (0..8).flat_map(|y| (0..16).map(move |x| (x, y))).map(|(x, y)| color.pixel(x, y)).collect();
    assert!(opaque.iter().all(|p| p[3] == 255 && (112..=144).contains(&(p[0] as i64))));
    assert!(opaque.iter().map(|p| p[0]).collect::<std::collections::HashSet<_>>().len() > 5);
    assert!(opaque.iter().any(|p| p[0] != p[1]), "colour noise differs per channel");
    assert!((16..32).all(|x| { let p = color.pixel(x, 0); p[3] == 0 && p[0] == 0 }), "clear pixels stay clear");
    let mono = add_noise(&gray, 10.0, true, true, 7);
    assert!((0..16).all(|x| { let p = mono.pixel(x, 0); p[0] == p[1] && p[1] == p[2] && p[3] == 255 }));
}

/// NoisePixels.c:5-49 transcribed independently of filters.rs: what noise_add_at writes into one
/// channel of the pixel at field position (px, py).
fn c_noise_channel(p: [u8; 4], c: usize, px: u32, py: u32, amount: f32, gaussian: bool, mono: bool, seed: u32) -> u8 {
    let hash = |mut x: u32| { x ^= x >> 16; x = x.wrapping_mul(0x7feb_352d); x ^= x >> 15; x = x.wrapping_mul(0x846c_a68b); x ^= x >> 16; x };
    let unit = |key: u32| (hash(key) >> 8) as f32 * (1.0 / 16_777_216.0);
    let spread = amount / 100.0 * 127.5;
    let base = hash(seed ^ hash(px.wrapping_mul(0x9e37_79b9) ^ hash(py.wrapping_mul(0x85eb_ca6b))));
    let key = if mono { base } else { base.wrapping_add((c as u32).wrapping_mul(0x9e37_79b9)) };
    let n = if gaussian {
        (-2.0 * (1.0 - unit(key)).ln()).sqrt() * (6.2831853 * unit(key ^ 0x68e3_1da4)).cos() * spread * (2.0 / 3.0)
    } else { (unit(key) * 2.0 - 1.0) * spread };
    let alpha = p[3] as f32;
    let value = (p[c] as f32 * 255.0 / alpha + n).clamp(0.0, 255.0);
    (value * alpha / 255.0).round() as u8
}

#[test]
fn add_noise_hashes_each_pixel_by_its_position_as_mac_1_2_6_does() {
    // A translucent 7 x 3 raster: under the older index hash, (x, y) would take the noise of
    // x + 7 * y, which differs from the position hash everywhere but the first row's first pixel.
    let source = Raster::from_premultiplied(7, 3, [90u8, 60, 30, 200].repeat(21));
    for (gaussian, mono, seed) in [(false, false, 7u32), (true, true, 1234)] {
        let noisy = add_noise(&source, 35.0, gaussian, mono, seed);
        for y in 0..3 { for x in 0..7 { for c in 0..3 {
            assert_eq!(noisy.pixel(x, y)[c], c_noise_channel(source.pixel(x, y), c, x, y, 35.0, gaussian, mono, seed), "({x}, {y}) channel {c}");
        }}}
    }
}

#[test]
fn add_noise_at_an_origin_is_that_part_of_the_same_field() {
    let big = Raster::from_premultiplied(9, 6, [120u8, 120, 120, 255].repeat(54));
    let whole = add_noise(&big, 20.0, false, false, 42);
    let shifted = add_noise_at(&big.cropped(4, 2, 5, 4), 20.0, false, false, 42, 4, 2);
    assert_eq!(shifted.bytes(), whole.cropped(4, 2, 5, 4).bytes());
}

#[test]
fn remove_distortion_bends_about_the_center_and_only_pincushion_opens_the_corners() {
    // Four quadrants of distinct opaque colours.
    let mut data = vec![0u8; 40 * 30 * 4];
    for y in 0..30u32 { for x in 0..40u32 {
        let index = (if y < 15 { 0 } else { 2 }) + if x < 20 { 0 } else { 1 };
        let i = ((y * 40 + x) * 4) as usize;
        data[i..i + 4].copy_from_slice(&[(index as f64 / 3.0 * 255.0) as u8, 128, (255.0 - index as f64 / 3.0 * 255.0) as u8, 255]);
    }}
    let source = Raster::from_premultiplied(40, 30, data);
    assert_eq!(lens_distort(&source, 0.0).bytes(), source.bytes(), "no distortion, no change");
    let barrel = lens_distort(&source, 100.0 / 100.0 * 0.35);
    assert!(alpha(&barrel, 0, 0) == 255 && alpha(&barrel, 39, 29) == 255, "straightening barrel stretches outward, nothing opens up");
    let pincushion = lens_distort(&source, -100.0 / 100.0 * 0.35);
    assert!(alpha(&pincushion, 0, 0) == 0 && alpha(&pincushion, 39, 29) == 0, "straightening pincushion pulls the edges in");
    assert_eq!(pincushion.pixel(20, 15), source.pixel(20, 15), "the middle stays put");
}

#[test]
fn filter_params_carry_their_name_margin_and_preview_scaling() {
    let blur = FilterParams::GaussianBlur { radius: 4.0 };
    assert_eq!(blur.name(), "Gaussian Blur");
    assert_eq!(blur.margin(), 14.0);
    assert_eq!(blur.scaled(0.5), FilterParams::GaussianBlur { radius: 2.0 });
    assert!(blur.spreads());
    let motion = FilterParams::MotionBlur { angle: 30.0, distance: 20.0 };
    assert_eq!(motion.margin(), 12.0);
    assert!(motion.spreads());
    let noise = FilterParams::AddNoise { amount: 10.0, gaussian: false, monochromatic: false, seed: 1 };
    assert!(!noise.spreads() && noise.margin() == 0.0);
    assert_eq!(noise.scaled(0.5), noise, "noise and lens correction do not scale with a preview");
    assert!(FilterParams::LensCorrection { distortion: 0.0 }.is_identity());
    assert_eq!(FilterParams::GaussianBlur { radius: 500.0 }.normalized(), FilterParams::GaussianBlur { radius: 250.0 });
    let json = serde_json::to_string(&motion).unwrap();
    assert_eq!(json, r#"{"filter":"MotionBlur","angle":30.0,"distance":20.0}"#);
    assert_eq!(serde_json::from_str::<FilterParams>(&json).unwrap(), motion);
}

/// Premultiplied bytes from a fixed linear congruential generator: a third clear, a third
/// translucent, so every rounding and the colour-under-alpha clamp are exercised.
fn noisy(width: u32, height: u32, mut s: u32) -> Raster {
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for _ in 0..width * height {
        let mut next = || { s = s.wrapping_mul(1_103_515_245).wrapping_add(12_345) & 0x7fff_ffff; (s >> 16) & 0xff };
        let (r, g, b, pick) = (next(), next(), next(), next());
        let a = match pick % 3 { 0 => 0, 1 => 255, _ => 40 + pick % 200 };
        data.extend_from_slice(&[(r * a / 255) as u8, (g * a / 255) as u8, (b * a / 255) as u8, a as u8]);
    }
    Raster::from_premultiplied(width, height, data)
}

/// The full-frame Gaussian this crate ran before its horizontal pass was banded (16 bytes of f32
/// per pixel), kept as the reference the banded one must equal to the bit.
fn full_frame_gaussian(raster: &Raster, sigma: f64) -> Raster {
    if !(sigma > 0.0) || raster.width == 0 || raster.height == 0 { return raster.clone(); }
    let radius = (sigma * 3.0).ceil() as i64;
    let kernel: Vec<f32> = (-radius..=radius).map(|i| (-(i * i) as f64 / (2.0 * sigma * sigma)).exp() as f32).collect();
    let sum: f32 = kernel.iter().sum();
    let (w, h) = (raster.width as i64, raster.height as i64);
    let src = raster.bytes();
    let mut tmp = vec![0f32; (w * h * 4) as usize];
    for y in 0..h { for x in 0..w {
        let mut acc = [0f32; 4];
        for (k, weight) in kernel.iter().enumerate() {
            let sx = x + k as i64 - radius;
            if sx < 0 || sx >= w { continue; }
            let i = ((y * w + sx) * 4) as usize;
            for c in 0..4 { acc[c] += src[i + c] as f32 * weight; }
        }
        let i = ((y * w + x) * 4) as usize;
        for c in 0..4 { tmp[i + c] = acc[c] / sum; }
    }}
    let mut out = vec![0u8; (w * h * 4) as usize];
    for y in 0..h { for x in 0..w {
        let mut acc = [0f32; 4];
        for (k, weight) in kernel.iter().enumerate() {
            let sy = y + k as i64 - radius;
            if sy < 0 || sy >= h { continue; }
            let i = ((sy * w + x) * 4) as usize;
            for c in 0..4 { acc[c] += tmp[i + c] * weight; }
        }
        let i = ((y * w + x) * 4) as usize;
        let alpha = (acc[3] / sum).round().clamp(0.0, 255.0);
        out[i + 3] = alpha as u8;
        for c in 0..3 { out[i + c] = (acc[c] / sum).round().clamp(0.0, alpha) as u8; }
    }}
    Raster::from_premultiplied(raster.width, raster.height, out)
}

/// The halved path as it ran before it wrote over its input: `level` halvings, the kernel, then a
/// fresh full-size raster sampled bilinearly from the reduced one.
fn full_frame_halved(raster: &Raster, level: u32, blur: impl Fn(&Raster) -> Raster) -> Raster {
    let mut small = raster.clone();
    for _ in 0..level { small = small.halved(); }
    let small = blur(&small);
    let f = (1u32 << level) as f64;
    let (width, height) = (raster.width, raster.height);
    let mut data = vec![0u8; (width as usize) * (height as usize) * 4];
    for y in 0..height { for x in 0..width {
        let s = sample(&small, (x as f64 + 0.5) / f, (y as f64 + 0.5) / f, false);
        let i = ((y * width + x) * 4) as usize;
        let alpha = (s[3] * 255.0).round().clamp(0.0, 255.0);
        data[i + 3] = alpha as u8;
        for c in 0..3 { data[i + c] = (s[c] * 255.0).round().clamp(0.0, alpha) as u8; }
    }}
    Raster::from_premultiplied(width, height, data)
}

#[test]
fn the_banded_gaussian_equals_the_full_frame_one_to_the_bit() {
    // Radii 1, 3, 8, 9, 10 and 24 taps each side; heights either side of the band (2 * radius + 1
    // rows), a single row, and several bands deep; widths oblong and odd, and a single column.
    for (n, sigma) in [0.3, 1.0, 2.5, 3.0, 10.0 / 3.0, 7.7].into_iter().enumerate() {
        let radius = (sigma * 3.0_f64).ceil() as u32;
        let band = 2 * radius + 1;
        for height in [1, 2, radius, band - 1, band, band + 1, 3 * band + 2] {
            for width in [1, 13, 2 * band + 3] {
                let source = noisy(width, height, 7 + n as u32 * 31 + height * 5 + width);
                assert_eq!(gaussian_blur(&source, sigma).bytes(), full_frame_gaussian(&source, sigma).bytes(), "sigma {sigma}, {width} x {height}");
            }
        }
    }
}

#[test]
fn a_blur_layer_written_over_its_input_equals_the_full_frame_path_to_the_bit() {
    let source = noisy(151, 97, 99);
    // Level 0 (reach 30), 1 (reach 60) and 2 (reach 120) of `spatial_level`.
    assert_eq!(blur_for_layer(source.clone(), 10.0).bytes(), full_frame_gaussian(&source, 10.0).bytes(), "level 0");
    for (sigma, level) in [(20.0, 1), (40.0, 2)] {
        assert_eq!(spatial_level(sigma * 3.0), level);
        let expected = full_frame_halved(&source, level, |r| full_frame_gaussian(r, sigma / (1u32 << level) as f64));
        assert_eq!(blur_for_layer(source.clone(), sigma).bytes(), expected.bytes(), "Gaussian at level {level}");
    }
    // A Motion Blur reaches three sigmas and halves past the same 48: 100 px (86.6) is level 1,
    // 250 px (216.5) level 3.
    for (distance, level) in [(100.0, 1), (250.0, 3)] {
        assert_eq!(spatial_level(motion_reach(distance)), level);
        let expected = full_frame_halved(&source, level, |r| motion_blur(r, 30.0, distance / (1u32 << level) as f64));
        assert_eq!(streak_for_layer(source.clone(), 30.0, distance).bytes(), expected.bytes(), "Motion Blur at level {level}");
    }
}

/// The Motion Blur as CIMotionBlur's kernel reads, one tap at a time: `2 ceil(3 sigma) + 1` taps
/// along (cos a, -sin a), each a bilinear sample (zero beyond the raster) weighted
/// exp(-t^2 / 2 sigma^2), sigma = distance / sqrt(12). What the GPU's FRAG_MOTION runs, and the
/// reference the row-wise stencil must equal.
fn per_tap_motion(raster: &Raster, angle: f64, distance: f64) -> Raster {
    let sigma = distance / 12f64.sqrt();
    let radius = (sigma * 3.0).ceil() as i64;
    let (dx, dy) = (angle.to_radians().cos(), -angle.to_radians().sin());
    let fetch = |x: i64, y: i64| -> [f32; 4] {
        if x < 0 || y < 0 || x >= raster.width as i64 || y >= raster.height as i64 { return [0.0; 4]; }
        raster.pixel(x as u32, y as u32).map(|v| v as f32)
    };
    let kernel: Vec<f32> = (-radius..=radius).map(|t| (-((t * t) as f64) / (2.0 * sigma * sigma)).exp() as f32).collect();
    let sum: f32 = kernel.iter().sum();
    let mut out = vec![0u8; (raster.width * raster.height * 4) as usize];
    for y in 0..raster.height { for x in 0..raster.width {
        let mut acc = [0f32; 4];
        for (k, weight) in kernel.iter().enumerate() {
            let t = (k as i64 - radius) as f64;
            let (fx, fy) = (x as f64 + dx * t, y as f64 + dy * t);
            let (x0, y0) = (fx.floor(), fy.floor());
            let (tx, ty) = ((fx - x0) as f32, (fy - y0) as f32);
            let (a, b) = (fetch(x0 as i64, y0 as i64), fetch(x0 as i64 + 1, y0 as i64));
            let (c, d) = (fetch(x0 as i64, y0 as i64 + 1), fetch(x0 as i64 + 1, y0 as i64 + 1));
            for i in 0..4 { acc[i] += ((a[i] * (1.0 - tx) + b[i] * tx) * (1.0 - ty) + (c[i] * (1.0 - tx) + d[i] * tx) * ty) * weight; }
        }
        let i = ((y * raster.width + x) * 4) as usize;
        let alpha = (acc[3] / sum).round().clamp(0.0, 255.0);
        out[i + 3] = alpha as u8;
        for c in 0..3 { out[i + c] = (acc[c] / sum).round().clamp(0.0, alpha) as u8; }
    }}
    Raster::from_premultiplied(raster.width, raster.height, out)
}

#[test]
fn the_row_wise_motion_stencil_equals_one_tap_at_a_time_within_float_ordering() {
    // The stencil regroups the same products in another order. Measured on p4a-scratch
    // (2026-09-27): 2 of 23668 bytes one level apart at 30 degrees / 24 px (a sum landing on .5),
    // every other case identical. Angles on and off the axes, lengths from a no-op to a long reach.
    let source = noisy(97, 61, 7);
    for (angle, distance) in [(30.0, 24.0), (0.0, 13.0), (90.0, 7.0), (-60.0, 40.0), (45.0, 1.0), (12.5, 33.0)] {
        let (a, b) = (motion_blur(&source, angle, distance), per_tap_motion(&source, angle, distance));
        let worst = a.bytes().iter().zip(b.bytes()).map(|(x, y)| x.abs_diff(*y)).max().unwrap();
        let count = a.bytes().iter().zip(b.bytes()).filter(|(x, y)| x != y).count();
        assert!(worst <= 1 && count <= 2, "{angle} degrees, {distance} px: worst {worst} over {count} bytes");
    }
}