use compositor_engine::*;

fn solid(r: u8, g: u8, b: u8, a: u8) -> Raster {
    let (r, g, b) = if a == 255 { (r, g, b) } else { ((r as u32 * a as u32 / 255) as u8, (g as u32 * a as u32 / 255) as u8, (b as u32 * a as u32 / 255) as u8) };
    Raster::from_premultiplied(2, 2, [r, g, b, a].repeat(4))
}
/// Straight (unpremultiplied) bytes of the first pixel.
fn straight(raster: &Raster) -> [u32; 4] {
    let p = raster.pixel(0, 0);
    let a = p[3] as u32;
    if a == 0 { return [0, 0, 0, 0]; }
    [(p[0] as u32 * 255 + a / 2) / a, (p[1] as u32 * 255 + a / 2) / a, (p[2] as u32 * 255 + a / 2) / a, a]
}

#[test]
fn exposure_works_in_linear_light_with_offset_and_gamma() {
    let gray = solid(128, 128, 128, 255);
    assert_eq!(apply_tables(&gray, &exposure_table(&ExposureSettings::default())).bytes(), gray.bytes(), "defaults change nothing");
    let brighter = straight(&apply_tables(&gray, &exposure_table(&ExposureSettings { exposure: 1.0, ..Default::default() })));
    assert!((brighter[0] as i64 - 176).abs() <= 2, "+1 stop doubles linear light: {brighter:?}");
    assert_eq!(brighter[0], brighter[2]);
    let lifted = straight(&apply_tables(&gray, &exposure_table(&ExposureSettings { gamma: 2.0, ..Default::default() })));
    assert!((lifted[0] as i64 - 181).abs() <= 2, "gamma 2 takes the square root of linear light: {lifted:?}");
    let offset = straight(&apply_tables(&solid(0, 0, 0, 255), &exposure_table(&ExposureSettings { offset: 0.1, ..Default::default() })));
    assert!((offset[0] as i64 - 89).abs() <= 2, "offset adds linear light: {offset:?}");
    let translucent = apply_tables(&solid(128, 128, 128, 128), &exposure_table(&ExposureSettings { exposure: 1.0, ..Default::default() }));
    assert_eq!(translucent.pixel(0, 0)[3], 128, "alpha kept");
}

#[test]
fn gradient_map_colors_by_brightness_and_reverses() {
    let mut s = GradientMapSettings { shadows: AdjustmentColor { red: 1.0, green: 0.0, blue: 0.0 }, highlights: AdjustmentColor { red: 0.0, green: 0.0, blue: 1.0 }, reversed: false };
    let table = gradient_map_table(&s);
    assert_eq!(straight(&apply_gradient_map(&solid(0, 0, 0, 255), &table)), [255, 0, 0, 255]);
    assert_eq!(straight(&apply_gradient_map(&solid(255, 255, 255, 255), &table)), [0, 0, 255, 255]);
    let middle = straight(&apply_gradient_map(&solid(128, 128, 128, 255), &table));
    assert!((middle[0] as i64 - 127).abs() <= 2 && (middle[2] as i64 - 128).abs() <= 2 && middle[1] == 0, "{middle:?}");
    let translucent = straight(&apply_gradient_map(&solid(255, 255, 255, 128), &table));
    assert!(translucent[2] >= 250 && translucent[0] <= 5 && translucent[3] == 128, "{translucent:?}");
    s.reversed = true;
    assert_eq!(straight(&apply_gradient_map(&solid(0, 0, 0, 255), &gradient_map_table(&s))), [0, 0, 255, 255]);
}

#[test]
fn curves_identity_is_exact_and_a_flat_curve_whitens_while_keeping_alpha() {
    let source = Raster::from_premultiplied(2, 2, vec![102,179,26,255, 51,89,13,128, 13,22,3,32, 0,0,0,0]);
    assert_eq!(apply_tables(&source, &curves_tables(&CurvesSettings::default())).bytes(), source.bytes());
    let mut s = CurvesSettings::default();
    s.channels[0] = vec![CurvePoint { x: 0.0, y: 255.0 }, CurvePoint { x: 255.0, y: 255.0 }];
    let out = apply_tables(&source, &curves_tables(&s));
    assert_eq!(out.bytes(), &[255,255,255,255, 128,128,128,128, 32,32,32,32, 0,0,0,0], "white at every input, premultiplied by the original alpha");
}

#[test]
fn a_curve_point_moves_the_tone_it_names() {
    let mut s = CurvesSettings::default();
    s.channels[0] = vec![CurvePoint { x: 0.0, y: 0.0 }, CurvePoint { x: 128.0, y: 190.0 }, CurvePoint { x: 255.0, y: 255.0 }];
    let out = straight(&apply_tables(&solid(128, 128, 128, 255), &curves_tables(&s)));
    assert!((out[0] as i64 - 190).abs() <= 1, "{out:?}");
    let mut red_only = CurvesSettings::default();
    red_only.channels[LevelsChannel::Red.index()] = vec![CurvePoint { x: 0.0, y: 0.0 }, CurvePoint { x: 255.0, y: 128.0 }];
    let halved = straight(&apply_tables(&solid(200, 200, 200, 255), &curves_tables(&red_only)));
    assert!(halved[0] < halved[1] && halved[1] == halved[2], "the red channel alone: {halved:?}");
}

#[test]
fn invert_keeps_transparency_and_round_trips() {
    let source = Raster::from_premultiplied(2, 2, vec![255,0,0,255, 64,32,0,128, 0,0,0,0, 10,20,30,40]);
    let inverted = invert_raster(&source);
    assert_eq!(&inverted.bytes()[0..8], &[0,255,255,255, 64,96,128,128], "premultiplied colour becomes alpha minus colour");
    assert_eq!(&inverted.bytes()[8..12], &[0,0,0,0], "clear pixels stay clear");
    assert_eq!(invert_raster(&inverted).bytes(), source.bytes());
    let gray = GrayRaster::from_bytes(2, 1, vec![0, 200]);
    assert_eq!(invert_gray(&gray).bytes(), &[255, 55]);
}
