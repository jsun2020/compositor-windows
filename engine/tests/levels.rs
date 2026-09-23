use compositor_engine::*;

/// The last pixel is fully transparent with non-zero colour bytes, so only the kernel's alpha
/// guard (skip alpha 0) keeps them: without it they unpremultiply to infinity and come back 0.
fn ramp() -> Raster { Raster::from_premultiplied(6, 1, vec![0,0,0,255, 64,64,64,255, 128,128,128,255, 255,255,255,255, 64,32,0,128, 90,40,10,0]) }

#[test]
fn input_clipping_gamma_output_inversion_and_alpha() {
    let source = ramp();
    let mut s = LevelsSettings::default();
    s.ranges[0] = LevelRange { black: 64.0, gamma: 1.0, white: 128.0, ..LevelRange::default() };
    let clipped = apply_tables(&source, &levels_tables(&s));
    assert_eq!(&clipped.bytes()[0..12], &[0,0,0,255, 0,0,0,255, 255,255,255,255]);
    s.ranges[0] = LevelRange { gamma: 2.0, ..LevelRange::default() };
    let b = apply_tables(&source, &levels_tables(&s)).bytes().to_vec();
    assert!((b[4] as i32 - 128).abs() <= 1 && (b[8] as i32 - 181).abs() <= 1);
    // Straight (127.5, 63.75, 0) through sqrt, re-premultiplied at alpha 128.
    assert_eq!(&b[16..20], &[91, 64, 0, 128], "unpremultiplied, mapped, re-premultiplied");
    assert_eq!(&b[20..24], &[90, 40, 10, 0], "a transparent pixel is skipped, not mapped");
    s.ranges[0] = LevelRange { output_black: 255.0, output_white: 0.0, ..LevelRange::default() };
    let inv = apply_tables(&source, &levels_tables(&s)).bytes().to_vec();
    assert!(inv[0] == 255 && inv[12] == 0);
    assert_eq!(&inv[16..20], &[64, 96, 128, 128], "one unpremultiply inside the kernel, as LevelsTests expects");
}

#[test]
fn a_result_past_white_is_clamped_to_alpha_and_one_below_black_to_zero() {
    // A premultiplied channel can never exceed its alpha. The Levels tables stay within 0..1, so
    // only tables from outside (apply_tables is public) reach the clamp: 2.0 and -1.0 everywhere.
    let source = Raster::from_premultiplied(1, 1, vec![64, 32, 16, 128]);
    let over = apply_tables(&source, &[2.0f32; 768]);
    assert_eq!(over.bytes(), &[128, 128, 128, 128], "clamped to alpha, not saturated to 255");
    let under = apply_tables(&source, &[-1.0f32; 768]);
    assert_eq!(under.bytes(), &[0, 0, 0, 128]);
}

#[test]
fn identity_tables_are_an_exact_no_op() {
    let source = ramp();
    let out = apply_tables(&source, &levels_tables(&LevelsSettings::default()));
    assert_eq!(out.bytes(), source.bytes());
}

#[test]
fn histogram_excludes_transparency_and_weights_coverage() {
    let source = Raster::from_premultiplied(3, 1, vec![255,0,0,255, 0,128,0,128, 0,0,0,0]);
    let bins = histogram(&source, None);
    assert!(bins[1][255] == 1.0 && (bins[2][255] - 128.0 / 255.0).abs() < 1e-5);
    let total: f64 = bins[0].iter().sum();
    assert!((total - (1.0 + 128.0 / 255.0)).abs() < 1e-5);
    let cov = GrayRaster::from_bytes(3, 1, vec![255, 0, 0]);
    let selected = histogram(&source, Some(&cov));
    assert!(selected[1][255] == 1.0 && selected[2][255] == 0.0);
}

#[test]
fn auto_algorithms_and_eyedropper_calibration() {
    let mut bins = vec![vec![0.0; 256]; 4];
    for c in 1..=3 { bins[c][20 * c] = 100.0; bins[c][200 + c * 10] = 100.0; }
    let linked = LevelsAuto::Contrast.settings(&bins);
    assert!(linked.ranges[0].black == 20.0 && linked.ranges[0].white == 230.0);
    let color = LevelsAuto::Color.settings(&bins);
    assert!(color.ranges[1].black == 20.0 && color.ranges[3].black == 60.0 && color.ranges[0] == LevelRange::default());
    // Neutral sets each channel's gamma so its mean tone lands on mid gray. A third tone a quarter
    // of the way up each channel's range skews the mean below 0.5; two symmetric tones alone would
    // give gamma 1, the same as doing nothing.
    let mut skewed = bins.clone();
    let mut want = [0.0; 4];
    for c in 1..=3 {
        let (low, high) = ((20 * c) as f64, (200 + c * 10) as f64);
        let quarter = (low + (high - low) / 4.0).round();
        skewed[c][quarter as usize] = 100.0;
        let mean = (0.0 + 1.0 + (quarter - low) / (high - low)) / 3.0;
        want[c] = mean.ln() / 0.5f64.ln();
    }
    let neutral = LevelsAuto::Neutral.settings(&skewed);
    for c in 1..=3 {
        assert!(want[c] > 1.2, "the fixture really is skewed: {}", want[c]);
        assert!((neutral.ranges[c].gamma - want[c]).abs() < 1e-9, "channel {c}: {} vs {}", neutral.ranges[c].gamma, want[c]);
    }
    let empty = vec![vec![0.0; 256]; 4];
    for mode in [LevelsAuto::Contrast, LevelsAuto::Color, LevelsAuto::Neutral] { assert!(mode.settings(&empty).is_identity()); }
    let rgb = [0.25, 0.4, 0.6];
    for mode in [LevelsSample::Black, LevelsSample::Gray, LevelsSample::White] {
        let s = LevelsSettings::default().sampling(rgb, mode);
        let target = match mode { LevelsSample::Black => 0.0, LevelsSample::White => 1.0, LevelsSample::Gray => 0.5 };
        for (i, ch) in [LevelsChannel::Red, LevelsChannel::Green, LevelsChannel::Blue].iter().enumerate() {
            assert!((s.apply(rgb[i], *ch) - target).abs() < 1e-4, "{mode:?} {ch:?}");
        }
    }
}
