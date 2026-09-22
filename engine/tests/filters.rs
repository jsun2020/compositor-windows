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
    // An even streak: the dot's alpha is spread over about `distance` pixels, not tapered to a point.
    let total: i64 = (0..41).map(|x| alpha(&horizontal, x, 20)).sum();
    assert!((total - 255).abs() <= 8, "energy is preserved: {total}");
    assert!(alpha(&horizontal, 20, 20) < 40, "no spike at the centre");
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
