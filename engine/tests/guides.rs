use compositor_engine::*;

fn doc_with_guides(w: u32, h: u32) -> Document {
    let mut doc = Document::new(w, h);
    doc.guides = vec![
        Guide { id: uuid::Uuid::new_v4(), axis: GuideAxis::Vertical, position: 20.0 },
        Guide { id: uuid::Uuid::new_v4(), axis: GuideAxis::Horizontal, position: 10.0 },
    ];
    doc
}
fn positions(doc: &Document) -> (f64, f64) {
    let v = doc.guides.iter().find(|g| g.axis == GuideAxis::Vertical).unwrap().position;
    let h = doc.guides.iter().find(|g| g.axis == GuideAxis::Horizontal).unwrap().position;
    (v, h)
}

#[test]
fn canvas_size_moves_guides_with_the_content() {
    // Anchor 8 is bottom-right: 40 extra columns on the left, 30 extra rows on the top.
    let out = ops::canvas_size::canvas_size(&doc_with_guides(100, 50),
        ops::canvas_size::CanvasSizeOptions { width: 140, height: 80, anchor: 8, fill: None, content_offset: None }).unwrap();
    assert_eq!(positions(&out), (60.0, 40.0));
}

#[test]
fn crop_moves_guides_by_its_offset() {
    let out = ops::canvas_size::canvas_size(&doc_with_guides(100, 50),
        ops::canvas_size::CanvasSizeOptions { width: 60, height: 30, anchor: 4, fill: None, content_offset: Some(Point { x: -15.0, y: -4.0 }) }).unwrap();
    assert_eq!(positions(&out), (5.0, 6.0));
}

#[test]
fn image_size_scales_each_axis_by_its_own_factor() {
    let out = ops::image_size::image_size(&doc_with_guides(100, 50),
        ops::image_size::ImageSizeOptions { width: 300, height: 100, resolution: 72.0, sampling: Sampling::Smooth }).unwrap();
    assert_eq!(positions(&out), (60.0, 20.0), "x3 across, x2 down");
}

#[test]
fn flip_canvas_mirrors_only_the_perpendicular_guides() {
    let mut doc = doc_with_guides(100, 40);
    ops::flip::flip_canvas(&mut doc, true);
    assert_eq!(positions(&doc), (80.0, 10.0));
    ops::flip::flip_canvas(&mut doc, false);
    assert_eq!(positions(&doc), (80.0, 30.0));
}

#[test]
fn a_guide_beyond_the_macs_saveable_range_refuses_an_enlarging_image_size() {
    // M7: a guide already legal (up to 1e6) plus an enlarging resize can push it past the
    // range manifest.rs validate enforces at save, after which this build (no guide UI, only
    // undo) could never save the project again. Refuse instead, as the ops already do for a
    // moved layer transform that would go out of range.
    let mut doc = Document::new(100, 50);
    doc.guides = vec![Guide { id: uuid::Uuid::new_v4(), axis: GuideAxis::Vertical, position: 900_000.0 }];
    let before = doc.clone();
    let err = ops::image_size::image_size(&doc, ops::image_size::ImageSizeOptions {
        width: 200, height: 50, resolution: 72.0, sampling: Sampling::Smooth }).unwrap_err();
    assert_eq!(err, ProjectError::TooLarge);
    assert_eq!(doc, before, "a refused resize leaves the document untouched");
}

#[test]
fn a_guide_beyond_the_macs_saveable_range_refuses_an_enlarging_canvas_size() {
    let mut doc = Document::new(100, 50);
    doc.guides = vec![Guide { id: uuid::Uuid::new_v4(), axis: GuideAxis::Vertical, position: 999_990.0 }];
    // Anchor 8 (bottom-right) adds the 40 new columns on the left: every vertical guide moves 40 right, past 1,000,000.
    let err = ops::canvas_size::canvas_size(&doc, ops::canvas_size::CanvasSizeOptions { width: 140, height: 50, anchor: 8, fill: None, content_offset: None }).unwrap_err();
    assert_eq!(err, ProjectError::TooLarge);
    // Anchor 0 adds them on the right: the guide stays put and the resize goes through.
    assert!(ops::canvas_size::canvas_size(&doc, ops::canvas_size::CanvasSizeOptions { width: 140, height: 50, anchor: 0, fill: None, content_offset: None }).is_ok());
}
