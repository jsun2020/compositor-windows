//! Complete source-to-Glow regression, independent actual Mac 1.4.9 goldens.
use compositor_engine::{
    adjust::scanlines::{ScanlineColors, ScanlinesSettings},
    Raster,
};

#[test]
fn original_mac_glow_matches_across_reduction_scales() {
    let mut straight = Vec::new();
    for y in 0..96_u32 {
        for x in 0..128_u32 {
            let alpha = if y < 48 {
                255
            } else {
                ((x * 17 + y * 13) % 256) as u8
            };
            straight.extend([
                ((x * 31 + y * 7) % 256) as u8,
                ((x * 3 + y * 41) % 256) as u8,
                ((x * 19 + y * 23) % 256) as u8,
                alpha,
            ]);
        }
    }
    let source = Raster::from_straight(128, 96, &straight);
    // FNV1a64 of untouched Mac Scanlines.apply outputs, not a software model:
    // Mac149-glow-composite-20261015-075932/composite-spacingN-original.rgba.
    // Four scales include direct RGBA8 reduction, half mips and odd scale21.
    for (spacing, expected) in [
        (2, 0x843a13047cd28727_u64),
        (7, 0xd2753e43c61026d8),
        (28, 0x935874f5f6add23c),
        (32, 0xd453382541da323f),
    ] {
        let output = ScanlinesSettings {
            line_spacing: spacing as f64,
            colors: ScanlineColors::Original,
            glow: 32.,
            ..Default::default()
        }
        .apply(&source)
        .unwrap();
        let hash = output
            .bytes()
            .iter()
            .fold(14695981039346656037_u64, |h, &v| {
                (h ^ u64::from(v)).wrapping_mul(1099511628211)
            });
        assert_eq!(hash, expected, "spacing {spacing}");
        assert!(output
            .bytes()
            .chunks_exact(4)
            .zip(source.bytes().chunks_exact(4))
            .all(|(out, input)| out[3] == input[3] && out[..3].iter().all(|&v| v <= out[3])));
    }
}
