//! GPU transport validation uses synthetic tile bytes. Actual GPU/worker ABI
//! execution is covered separately by the browser cases.
use compositor_engine::warp::{WarpMode, WarpSettings, WarpStroke};
use compositor_engine::*;
use uuid::Uuid;

fn point(x: f64, y: f64) -> Point {
    Point { x, y }
}
fn same_appearance(a: &Document, b: &Document) {
    assert_eq!(
        (a.width, a.height, a.resolution),
        (b.width, b.height, b.resolution)
    );
    assert_eq!(a.selection, b.selection);
    assert_eq!(a.layers.len(), b.layers.len());
    for (a, b) in a.layers.iter().zip(&b.layers) {
        // Install/undo deliberately assign fresh cache revisions. Compare the
        // complete persisted record and all bytes, not revision bookkeeping.
        assert_eq!(a.record(), b.record());
        assert_eq!(a.pixels, b.pixels);
        assert_eq!(a.mask, b.mask);
    }
}
fn setup() -> (Engine, Uuid, Uuid) {
    let mut doc = Document::new(20, 16);
    let mut bytes = [0, 0, 0, 255].repeat(320);
    for y in 0..16 {
        bytes[(y * 20 + 3) * 4..(y * 20 + 3) * 4 + 4].copy_from_slice(&[255; 4]);
    }
    let mut l = Layer::with_pixels(
        "Stripe",
        Raster::from_premultiplied(20, 16, bytes),
        point(0.0, 0.0),
    );
    l.opacity = 0.25;
    l.blend_mode = BlendMode::Multiply;
    l.mask = Some(Mask {
        pixels: GrayRaster::from_bytes(20, 16, vec![0; 320]),
        enabled: true,
        placement: None,
        linked: Some(true),
    });
    let id = l.id;
    doc.layers.push(l);
    doc.active_layer_id = Some(id);
    let mut e = Engine::new();
    let doc = e.insert_document(doc);
    (e, doc, id)
}
fn spec(mode: WarpMode) -> WarpSpec {
    WarpSpec {
        mode,
        diameter: 2.0,
        hardness: 0.5,
        strength: 0.5,
        points: vec![point(3.0, 8.0), point(6.0, 8.0)],
    }
}
fn tiles(source: &Raster, spec: &WarpSpec) -> Vec<(WarpTileRect, Vec<u8>)> {
    let mut stroke = WarpStroke::new(
        source.width as usize,
        source.height as usize,
        source.bytes(),
        spec.mode,
        WarpSettings {
            diameter: spec.diameter,
            hardness: spec.hardness as f32,
            strength: spec.strength as f32,
        },
    )
    .unwrap();
    for p in &spec.points {
        stroke.append_dabs([p.x, p.y]).unwrap();
    }
    let pixels = stroke.pixels();
    let mut out = vec![];
    // Send a small tile containing every changed pixel; unchanged tile pixels
    // must still be selected/clipped using the original layer snapshot.
    for x in [0, 5] {
        let r = WarpTileRect {
            x,
            y: 5,
            width: 5,
            height: 7,
        };
        let mut b = vec![];
        for y in r.y..r.y + r.height {
            let i = ((y * source.width + x) * 4) as usize;
            b.extend_from_slice(&pixels[i..i + 20]);
        }
        out.push((r, b));
    }
    out
}

#[test]
fn source_is_raw_and_result_equals_reference_with_selection_and_style() {
    for mode in [WarpMode::Smudge, WarpMode::Liquify] {
        for selected in [false, true] {
            let (mut e, doc, id) = setup();
            if selected {
                e.execute(
                    doc,
                    Command::SelectShape {
                        kind: SelectionShape::Rectangle,
                        points: vec![
                            point(4.0, 0.0),
                            point(5.0, 0.0),
                            point(5.0, 16.0),
                            point(4.0, 16.0),
                        ],
                        mode: SelectionMode::Replace,
                        antialiased: false,
                    },
                )
                .unwrap();
            }
            let before = e.document(doc).unwrap().clone();
            let depth = e.state(doc).unwrap().undo_depth;
            let (input, pixels, mask, points) = e.job_input(doc, id).unwrap();
            let source = run_warp_source_job(
                &input,
                pixels.clone(),
                mask.clone(),
                points.as_deref(),
                false,
            )
            .unwrap();
            assert_eq!(source.pixel(3, 8), [255; 4]); // zero mask and 25% opacity excluded
            let warp = spec(mode);
            let (output, result, new_mask, display) = run_warp_result_job(
                &input,
                pixels,
                mask,
                points.as_deref(),
                false,
                &warp,
                tiles(&source, &warp),
                0.2,
            )
            .unwrap();
            assert!(output.witness.is_some());
            assert!(new_mask.as_ref().unwrap().bytes().iter().all(|b| *b == 0));
            let mut oracle = Engine::new();
            let other = oracle.insert_document(before.clone());
            oracle
                .execute(
                    other,
                    Command::WarpStroke {
                        id,
                        mask: false,
                        warp,
                    },
                )
                .unwrap();
            e.install_job(doc, id, input.stamp, output, result, new_mask, display)
                .unwrap();
            same_appearance(e.document(doc).unwrap(), oracle.document(other).unwrap());
            assert_eq!(e.state(doc).unwrap().undo_depth, depth + 1);
            e.undo(doc).unwrap();
            same_appearance(e.document(doc).unwrap(), &before);
            e.redo(doc).unwrap();
            same_appearance(e.document(doc).unwrap(), oracle.document(other).unwrap());
        }
    }
}

#[test]
fn every_stale_dimension_rejects_the_gpu_result() {
    for change in 0..5 {
        let (mut e, doc, id) = setup();
        let (input, pixels, mask, points) = e.job_input(doc, id).unwrap();
        let warp = spec(WarpMode::Smudge);
        let source = run_warp_source_job(
            &input,
            pixels.clone(),
            mask.clone(),
            points.as_deref(),
            false,
        )
        .unwrap();
        let (out, px, m, display) = run_warp_result_job(
            &input,
            pixels,
            mask,
            points.as_deref(),
            false,
            &warp,
            tiles(&source, &warp),
            1.0,
        )
        .unwrap();
        let command = match change {
            0 => Command::InvertPixels { id, mask: false },
            1 => Command::NudgeLayers {
                ids: vec![id],
                dx: 1.0,
                dy: 0.0,
            },
            2 => Command::CanvasSize {
                width: 21,
                height: 16,
                anchor: 0,
                fill: None,
            },
            3 => Command::SelectAll,
            _ => Command::InvertPixels { id, mask: true },
        };
        e.execute(doc, command).unwrap();
        let before = e.document(doc).unwrap().clone();
        let depth = e.state(doc).unwrap().undo_depth;
        assert!(e
            .install_job(doc, id, input.stamp, out, px, m, display)
            .unwrap_err()
            .to_string()
            .contains(LAYER_CHANGED));
        assert!(e.document(doc).unwrap().same_content(&before));
        assert_eq!(e.state(doc).unwrap().undo_depth, depth);
    }
}

#[test]
fn malformed_readback_refuses_before_install_and_worker_can_continue() {
    let (e, doc, id) = setup();
    let (input, pixels, mask, points) = e.job_input(doc, id).unwrap();
    let warp = spec(WarpMode::Liquify);
    let r = WarpTileRect {
        x: 0,
        y: 0,
        width: 1,
        height: 1,
    };
    for bad in [
        vec![(r, vec![0; 3])],
        vec![(r, vec![255, 0, 0, 0])],
        vec![(r, vec![0; 4]), (r, vec![0; 4])],
        vec![(WarpTileRect { x: u32::MAX, ..r }, vec![0; 4])],
        vec![(WarpTileRect { width: 0, ..r }, vec![])],
    ] {
        assert!(run_warp_result_job(
            &input,
            pixels.clone(),
            mask.clone(),
            points.as_deref(),
            false,
            &warp,
            bad,
            1.0
        )
        .is_err());
    }
    let source = run_warp_source_job(
        &input,
        pixels.clone(),
        mask.clone(),
        points.as_deref(),
        false,
    )
    .unwrap();
    assert!(run_warp_result_job(
        &input,
        pixels,
        mask,
        points.as_deref(),
        false,
        &warp,
        tiles(&source, &warp),
        1.0
    )
    .is_ok());
    assert_eq!(e.state(doc).unwrap().undo_depth, 0);
}

#[test]
fn mask_target_and_pixel_free_pickup_are_refused() {
    let (e, doc, id) = setup();
    let (mut input, pixels, mask, points) = e.job_input(doc, id).unwrap();
    assert!(run_warp_source_job(
        &input,
        pixels.clone(),
        mask.clone(),
        points.as_deref(),
        true
    )
    .unwrap_err()
    .to_string()
    .contains(ops::warp::MASK_REFUSAL));
    assert!(run_warp_result_job(
        &input,
        pixels,
        mask.clone(),
        points.as_deref(),
        true,
        &spec(WarpMode::Smudge),
        vec![],
        1.0
    )
    .unwrap_err()
    .to_string()
    .contains(ops::warp::MASK_REFUSAL));
    input.pixels = None;
    let mut pickup = spec(WarpMode::Smudge);
    pickup.points.truncate(1);
    assert!(run_warp_result_job(
        &input,
        None,
        mask,
        points.as_deref(),
        false,
        &pickup,
        vec![],
        1.0
    )
    .is_err());
}

#[test]
fn pickup_preserves_metadata_and_redo_even_if_readback_bytes_differ() {
    let (mut e, doc, id) = setup();
    e.execute(doc, Command::InvertPixels { id, mask: false })
        .unwrap();
    e.undo(doc).unwrap();
    let before = e.document(doc).unwrap().clone();
    let (input, pixels, mask, points) = e.job_input(doc, id).unwrap();
    let mut pickup = spec(WarpMode::Smudge);
    pickup.points = vec![point(3.0, 8.0), point(3.5, 8.0)];
    let (out, px, m, display) = run_warp_result_job(
        &input,
        pixels,
        mask,
        points.as_deref(),
        false,
        &pickup,
        vec![(
            WarpTileRect {
                x: 4,
                y: 8,
                width: 1,
                height: 1,
            },
            vec![255; 4],
        )],
        1.0,
    )
    .unwrap();
    assert!(px.is_none());
    e.install_job(doc, id, input.stamp, out, px, m, display)
        .unwrap();
    assert!(e.document(doc).unwrap().same_content(&before));
    assert!(e.state(doc).unwrap().can_redo);
    assert_eq!(e.state(doc).unwrap().undo_depth, 0);
}

#[test]
fn large_canvas_uses_transport_without_the_cpu_reference_cap() {
    let (e, doc, id) = setup();
    let mut big = e.document(doc).unwrap().clone();
    big.width = 2049;
    big.height = 2048;
    let mut e = Engine::new();
    let doc = e.insert_document(big);
    let warp = spec(WarpMode::Smudge);
    assert!(e
        .execute(
            doc,
            Command::WarpStroke {
                id,
                mask: false,
                warp: warp.clone()
            }
        )
        .unwrap_err()
        .to_string()
        .contains(ops::warp::REFERENCE_LIMIT));
    let (input, pixels, mask, points) = e.job_input(doc, id).unwrap();
    let source = run_warp_source_job(
        &input,
        pixels.clone(),
        mask.clone(),
        points.as_deref(),
        false,
    )
    .unwrap();
    assert_eq!((source.width, source.height), (2049, 2048));
    assert_eq!(source.pixel(3, 8), [255; 4]);
    let (out, px, m, display) = run_warp_result_job(
        &input,
        pixels,
        mask,
        points.as_deref(),
        false,
        &warp,
        vec![(
            WarpTileRect {
                x: 4,
                y: 8,
                width: 1,
                height: 1,
            },
            vec![128, 128, 128, 255],
        )],
        1.0,
    )
    .unwrap();
    e.install_job(doc, id, input.stamp, out, px, m, display)
        .unwrap();
    assert_eq!(
        e.document(doc)
            .unwrap()
            .layer(id)
            .unwrap()
            .pixels
            .as_ref()
            .unwrap()
            .pixel(4, 8),
        [128, 128, 128, 255]
    );
}
