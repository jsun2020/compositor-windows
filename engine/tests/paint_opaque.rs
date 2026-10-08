#[path = "support/paint_opaque_probe.rs"]
mod probe;

#[test]
fn opaque_paints_match_complete_pre_optimization_outputs() {
    assert_eq!(probe::outputs().as_slice(), include_bytes!("fixtures/kernel-specialization/paint-opaque.rgba"));
}
