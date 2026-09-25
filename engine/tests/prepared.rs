use compositor_engine::*;

/// A 16-step gray ramp plus three primaries, opaque.
fn ramp() -> Raster {
    let mut data = Vec::new();
    for i in 0..16u32 { let v = (i * 17) as u8; data.extend_from_slice(&[v, v, v, 255]); }
    data.extend_from_slice(&[255, 0, 0, 255]); data.extend_from_slice(&[0, 255, 0, 255]); data.extend_from_slice(&[0, 0, 255, 255]);
    data.extend_from_slice(&[40, 90, 200, 255]);
    Raster::from_premultiplied(20, 1, data)
}

fn each_kind() -> Vec<LayerAdjustment> {
    let mut levels = LayerAdjustment::new(AdjustmentKind::Levels);
    levels.levels.ranges[0] = LevelRange { black: 20.0, gamma: 1.4, white: 230.0, output_black: 10.0, output_white: 250.0 };
    let mut curves = LayerAdjustment::new(AdjustmentKind::Curves);
    curves.curves.channels[0] = vec![CurvePoint { x: 0.0, y: 10.0 }, CurvePoint { x: 128.0, y: 190.0 }, CurvePoint { x: 255.0, y: 245.0 }];
    let mut exposure = LayerAdjustment::new(AdjustmentKind::Exposure);
    exposure.exposure_settings = Some(ExposureSettings { exposure: 0.8, offset: 0.05, gamma: 1.3 });
    let mut gradient = LayerAdjustment::new(AdjustmentKind::GradientMap);
    gradient.gradient_map_settings = Some(GradientMapSettings { shadows: AdjustmentColor { red: 0.1, green: 0.0, blue: 0.4 }, highlights: AdjustmentColor { red: 1.0, green: 0.9, blue: 0.2 }, reversed: false });
    let mut hsv = LayerAdjustment::new(AdjustmentKind::Hsv);
    let mut settings = HueSaturationSettings::new(35.0, 20.0, -10.0, false, ColorRange::Master);
    settings.adjustments.insert(ColorRange::Blues, RangeAdjustment { hue: -20.0, saturation: 40.0, lightness: 0.0 });
    hsv.hsv_settings = Some(settings);
    let mut grain = LayerAdjustment::new(AdjustmentKind::Grain);
    grain.grain_settings = Some(GrainSettings { amount: 50.0, size: 2.0, roughness: 30.0, seed: 11 });
    let mut bw = LayerAdjustment::new(AdjustmentKind::BlackWhite);
    bw.black_white_settings = Some(BlackWhiteSettings { reds: 115.0, yellows: -40.0, greens: 70.0, cyans: 180.0, blues: -90.0, magentas: 20.0, tint: true, tint_hue: 205.0, tint_saturation: 45.0 });
    let mut balance = LayerAdjustment::new(AdjustmentKind::ColorBalance);
    balance.color_balance_settings = Some(ColorBalanceSettings { mid_cyan_red: -35.0, highlight_yellow_blue: -50.0, shadow_magenta_green: -20.0, ..ColorBalanceSettings::default() });
    let mut noise = LayerAdjustment::new(AdjustmentKind::AddNoise);
    noise.noise_amount = Some(40.0); noise.noise_seed = Some(9);
    vec![levels, curves, exposure, gradient, hsv, grain, LayerAdjustment::new(AdjustmentKind::Invert), bw, balance, noise]
}

#[test]
fn the_single_color_path_matches_the_whole_raster_kernel() {
    let source = ramp();
    for adjustment in each_kind() {
        let whole = apply_adjustment(&source, &adjustment, Point { x: 0.0, y: 0.0 }, 1.0, None);
        let prepared = PreparedAdjustment::prepare(&adjustment);
        for i in 0..source.width {
            let p = source.pixel(i, 0);
            let rgb = [p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0];
            let at = Point { x: i as f64 + 0.5, y: 0.5 };
            let out = prepared.color(rgb, at);
            let expected = whole.pixel(i, 0);
            for c in 0..3 {
                let got = (out[c] * 255.0).round() as i64;
                assert!((got - expected[c] as i64).abs() <= 1, "{:?} pixel {i} channel {c}: {got} vs {}", adjustment.kind, expected[c]);
            }
        }
    }
}

#[test]
fn an_identity_adjustment_is_an_exact_no_op_and_coverage_blends() {
    let source = ramp();
    for kind in [AdjustmentKind::Levels, AdjustmentKind::Curves, AdjustmentKind::Exposure, AdjustmentKind::Hsv] {
        let a = LayerAdjustment::new(kind);
        assert_eq!(apply_adjustment(&source, &a, Point { x: 0.0, y: 0.0 }, 1.0, None).bytes(), source.bytes(), "{kind:?}");
    }
    let mut white = LayerAdjustment::new(AdjustmentKind::Curves);
    white.curves.channels[0] = vec![CurvePoint { x: 0.0, y: 255.0 }, CurvePoint { x: 255.0, y: 255.0 }];
    let adjusted = apply_adjustment(&source, &white, Point { x: 0.0, y: 0.0 }, 1.0, None);
    let coverage = GrayRaster::from_bytes(20, 1, (0..20).map(|i| if i < 10 { 255 } else { 0 }).collect());
    let blended = apply_adjustment(&source, &white, Point { x: 0.0, y: 0.0 }, 1.0, Some(&coverage));
    assert_eq!(blended.pixel(0, 0), adjusted.pixel(0, 0), "full coverage takes the adjusted colour");
    assert_eq!(blended.pixel(19, 0), source.pixel(19, 0), "no coverage leaves the original");
    let half = GrayRaster::from_bytes(20, 1, vec![128; 20]);
    let mixed = apply_adjustment(&source, &white, Point { x: 0.0, y: 0.0 }, 1.0, Some(&half));
    let expected = (source.pixel(5, 0)[0] as i64 + adjusted.pixel(5, 0)[0] as i64) / 2;
    assert!((mixed.pixel(5, 0)[0] as i64 - expected).abs() <= 1);
}

#[test]
fn grain_reads_the_document_point_so_two_tiles_agree() {
    let mut grain = LayerAdjustment::new(AdjustmentKind::Grain);
    grain.grain_settings = Some(GrainSettings { amount: 80.0, size: 3.0, roughness: 20.0, seed: 5 });
    let prepared = PreparedAdjustment::prepare(&grain);
    let rgb = [0.5, 0.5, 0.5];
    let a = prepared.color(rgb, Point { x: 12.5, y: 7.5 });
    assert_eq!(a, prepared.color(rgb, Point { x: 12.5, y: 7.5 }));
    assert_ne!(a, prepared.color(rgb, Point { x: 40.5, y: 7.5 }));
}
