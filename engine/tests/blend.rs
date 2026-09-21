use compositor_engine::*;

fn near(a: f32, b: f32) -> bool { (a - b).abs() < 0.02 }

#[test]
fn separable_modes_match_known_values() {
    // Backdrop 0.4 grey, source 0.8 grey (the macOS LayerAppearanceTests table).
    for (mode, expected) in [
        (BlendMode::Normal, 0.8), (BlendMode::Multiply, 0.32), (BlendMode::Screen, 0.88), (BlendMode::Overlay, 0.64),
        (BlendMode::Darken, 0.4), (BlendMode::Lighten, 0.8), (BlendMode::Difference, 0.4),
        (BlendMode::ColorDodge, 1.0), (BlendMode::ColorBurn, 0.25),
    ] {
        let out = compose([0.4, 0.4, 0.4, 1.0], [0.8, 0.8, 0.8, 1.0], mode);
        assert!(near(out[0], expected), "{mode:?}: {} vs {expected}", out[0]);
        assert!(near(out[3], 1.0));
    }
}

#[test]
fn non_separable_modes_follow_pdf_definitions() {
    let backdrop = [0.8, 0.2, 0.2]; // reddish
    let source = [0.2, 0.2, 0.8];   // bluish
    let hue = blend_rgb(BlendMode::Hue, backdrop, source);
    assert!(hue[2] > hue[0], "hue takes the source's hue (blue)");
    let lum = blend_rgb(BlendMode::Luminosity, backdrop, source);
    let l = |c: [f32; 3]| 0.3 * c[0] + 0.59 * c[1] + 0.11 * c[2];
    assert!(near(l(lum), l(source)), "luminosity takes the source's lum");
    assert!(lum[0] > lum[2], "but keeps the backdrop's hue");
    let sat = blend_rgb(BlendMode::Saturation, backdrop, [0.5, 0.5, 0.5]);
    assert!(near(sat[0], sat[1]) && near(sat[1], sat[2]), "zero saturation source greys the backdrop");
    let color = blend_rgb(BlendMode::Color, [0.5, 0.5, 0.5], source);
    assert!(color[2] > color[0] && near(l(color), 0.5));
}

#[test]
fn compose_is_source_over_for_normal_and_respects_alpha() {
    let out = compose([0.4, 0.0, 0.0, 0.4], [0.0, 0.0, 0.5, 0.5], BlendMode::Normal);
    assert!(near(out[3], 0.7));
    assert!(near(out[0], 0.2) && near(out[2], 0.5));
    // Multiply over a transparent backdrop is just the source.
    let out = compose([0.0; 4], [0.3, 0.3, 0.3, 0.6], BlendMode::Multiply);
    assert!(near(out[0], 0.3) && near(out[3], 0.6));
    // A translucent source multiplies only in proportion to its alpha.
    let out = compose([0.5, 0.5, 0.5, 1.0], [0.25, 0.25, 0.25, 0.5], BlendMode::Multiply);
    assert!(near(out[0], 0.5 * 0.5 + 0.5 * 0.5 * 0.5), "{}", out[0]);
    let mut bytes = [128u8, 128, 128, 255];
    compose_u8(&mut bytes, [0.25, 0.25, 0.25, 0.5], BlendMode::Multiply);
    assert!((bytes[0] as i32 - 96).abs() <= 1 && bytes[3] == 255);
}

#[test]
fn boundary_conditions_of_dodge_burn_and_compose() {
    // ColorDodge boundary: backdrop 0 should give 0, source 1.0 should give 1.0
    assert!(near(separable(BlendMode::ColorDodge, 0.0, 0.5), 0.0));
    assert!(near(separable(BlendMode::ColorDodge, 0.3, 1.0), 1.0));
    // ColorBurn boundary: backdrop 1.0 should give 1.0, source 0.0 should give 0.0
    assert!(near(separable(BlendMode::ColorBurn, 1.0, 0.5), 1.0));
    assert!(near(separable(BlendMode::ColorBurn, 0.3, 0.0), 0.0));
    // Transparent source should return unchanged destination
    let dst = [0.2, 0.3, 0.4, 0.9];
    let out = compose(dst, [0.0, 0.0, 0.0, 0.0], BlendMode::Multiply);
    assert!(near(out[0], dst[0]) && near(out[1], dst[1]) && near(out[2], dst[2]) && near(out[3], dst[3]));
    // Source-over onto transparent backdrop is just the source
    let out = compose([0.0; 4], [0.1, 0.2, 0.3, 0.5], BlendMode::ColorBurn);
    assert!(near(out[0], 0.1) && near(out[1], 0.2) && near(out[2], 0.3) && near(out[3], 0.5));
    // Saturation on a grey backdrop should stay grey (exercises set_sat's zero-range guard)
    let sat = blend_rgb(BlendMode::Saturation, [0.5, 0.5, 0.5], [0.9, 0.1, 0.1]);
    assert!(near(sat[0], sat[1]) && near(sat[1], sat[2]));
}
