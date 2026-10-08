mod support { include!("support/paint_canvas_probe.rs"); }

#[test]
fn canvas_bounds_specialization_preserves_complete_original_outputs() {
    assert_eq!(support::outputs().as_slice(),
        include_bytes!("fixtures/kernel-specialization/paint-canvas.rgba"));
}
