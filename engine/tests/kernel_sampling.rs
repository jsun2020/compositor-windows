mod support {
    include!("support/kernel_probe_inputs.rs");
}

// Synthetic complete outputs frozen with the original release engine before
// specializing the paint loop and preparing spatial interpolation columns.
// These cover channel rounding, partial coverage, transparent endpoints,
// rotated/flipped/clipped placement, odd dimensions and reduced edge clamping.
#[test]
fn paint_layout_specialization_preserves_complete_original_outputs() {
    assert_eq!(
        support::paint_outputs().as_slice(),
        include_bytes!("fixtures/kernel-specialization/paint.rgba")
    );
}

#[test]
fn spatial_enlargement_preserves_complete_original_outputs() {
    assert_eq!(
        support::spatial_outputs().as_slice(),
        include_bytes!("fixtures/kernel-specialization/spatial.rgba")
    );
}
