//! Independent document/job checks for Phase 6, beyond the local-plane oracle.
use compositor_engine::warp::WarpMode;
use compositor_engine::*;
use uuid::Uuid;

fn p(x: f64, y: f64) -> Point {
    Point { x, y }
}
fn setup() -> (Engine, Uuid, Uuid) {
    let mut doc = Document::new(20, 16);
    let mut bytes = [0, 0, 0, 255].repeat(20 * 16);
    for y in 0..16 {
        bytes[(y * 20 + 3) * 4..(y * 20 + 3) * 4 + 4].copy_from_slice(&[255; 4]);
    }
    let layer = Layer::with_pixels(
        "Stripe",
        Raster::from_premultiplied(20, 16, bytes),
        p(0.0, 0.0),
    );
    let id = layer.id;
    doc.layers.push(layer);
    doc.active_layer_id = Some(id);
    let mut engine = Engine::new();
    let handle = engine.insert_document(doc);
    (engine, handle, id)
}
fn spec(mode: WarpMode) -> WarpSpec {
    WarpSpec {
        mode,
        diameter: 2.0,
        hardness: 0.5,
        strength: 0.5,
        points: vec![p(3.0, 8.0), p(6.0, 8.0)],
    }
}
fn command(id: Uuid, mode: WarpMode) -> Command {
    Command::WarpStroke {
        id,
        mask: false,
        warp: spec(mode),
    }
}
fn pixels(e: &Engine, doc: Uuid, id: Uuid) -> Raster {
    e.document(doc)
        .unwrap()
        .layer(id)
        .unwrap()
        .pixels
        .clone()
        .unwrap()
}
fn selected(e: &mut Engine, doc: Uuid, left: f64, right: f64) {
    e.execute(
        doc,
        Command::SelectShape {
            kind: SelectionShape::Rectangle,
            points: vec![p(left, 0.0), p(right, 0.0), p(right, 16.0), p(left, 16.0)],
            mode: SelectionMode::Replace,
            antialiased: false,
        },
    )
    .unwrap();
}

#[test]
fn smudge_replaces_one_fading_trail_as_one_undo_redo_step() {
    let (mut e, doc, id) = setup();
    let before = pixels(&e, doc, id);
    let dirty = e.execute(doc, command(id, WarpMode::Smudge)).unwrap();
    let after = pixels(&e, doc, id);
    assert_eq!(after.pixel(4, 8), [128, 128, 128, 255]);
    assert_eq!(after.pixel(5, 8), [64, 64, 64, 255]);
    assert_eq!(after.pixel(6, 8), [32, 32, 32, 255]);
    assert_eq!(after.pixel(3, 8), [255; 4]);
    assert_eq!(after.pixel(19, 8), before.pixel(19, 8));
    assert_eq!(e.state(doc).unwrap().undo_depth, 1);
    assert_eq!(dirty.regions.len(), 1);
    assert!(dirty.regions[0].rect.width < 20);
    e.undo(doc).unwrap();
    assert_eq!(pixels(&e, doc, id), before);
    e.redo(doc).unwrap();
    assert_eq!(pixels(&e, doc, id), after);
}

#[test]
fn first_click_and_short_move_preserve_editable_metadata_and_redo() {
    let (base, old_doc, id) = setup();
    let mut initial = base.document(old_doc).unwrap().clone();
    initial.layer_mut(id).unwrap().extra.text =
        Some(serde_json::json!({"text":"Editable","runs":[]}));
    let mut e = Engine::new();
    let doc = e.insert_document(initial);
    e.execute(doc, command(id, WarpMode::Smudge)).unwrap();
    e.undo(doc).unwrap();
    let before = e.document(doc).unwrap().clone();
    assert!(before.layer(id).unwrap().extra.text.is_some());
    let state = e.state(doc).unwrap();
    for points in [vec![p(3.0, 8.0)], vec![p(3.0, 8.0), p(3.5, 8.0)]] {
        let mut warp = spec(WarpMode::Smudge);
        warp.points = points;
        e.execute(
            doc,
            Command::WarpStroke {
                id,
                mask: false,
                warp,
            },
        )
        .unwrap();
        assert!(e.document(doc).unwrap().same_content(&before));
        assert_eq!(e.state(doc).unwrap().undo_depth, state.undo_depth);
        assert_eq!(e.state(doc).unwrap().can_redo, state.can_redo);
    }
    e.redo(doc).unwrap();
    assert_eq!(pixels(&e, doc, id).pixel(4, 8), [128, 128, 128, 255]);
}

#[test]
fn selection_limits_writeback_but_not_smudge_pickup() {
    let (mut e, doc, id) = setup();
    selected(&mut e, doc, 5.0, 6.0);
    let before = pixels(&e, doc, id);
    let depth = e.state(doc).unwrap().undo_depth;
    e.execute(doc, command(id, WarpMode::Smudge)).unwrap();
    let after = pixels(&e, doc, id);
    assert_eq!(after.pixel(5, 8), [64, 64, 64, 255]);
    assert_eq!(after.pixel(4, 8), before.pixel(4, 8));
    assert_eq!(after.pixel(6, 8), before.pixel(6, 8));
    assert_eq!(e.state(doc).unwrap().undo_depth, depth + 1);
    e.undo(doc).unwrap();
    assert_eq!(pixels(&e, doc, id), before);
}

#[test]
fn transparent_smudge_clears_destination_instead_of_source_over() {
    let mut doc = Document::new(12, 12);
    let mut bytes = [255; 4].repeat(144);
    bytes[(6 * 12 + 3) * 4..(6 * 12 + 3) * 4 + 4].fill(0);
    let layer = Layer::with_pixels(
        "Alpha",
        Raster::from_premultiplied(12, 12, bytes),
        p(0.0, 0.0),
    );
    let id = layer.id;
    doc.layers.push(layer);
    let mut e = Engine::new();
    let doc = e.insert_document(doc);
    let mut warp = spec(WarpMode::Smudge);
    warp.strength = 1.0;
    warp.points = vec![p(3.0, 6.0), p(5.0, 6.0)];
    e.execute(
        doc,
        Command::WarpStroke {
            id,
            mask: false,
            warp,
        },
    )
    .unwrap();
    assert_eq!(pixels(&e, doc, id).pixel(4, 6), [0; 4]);
    assert_eq!(pixels(&e, doc, id).pixel(5, 6), [0; 4]);
    assert_eq!(pixels(&e, doc, id).pixel(6, 6), [255; 4]);
}

#[test]
fn empty_or_outside_footprint_is_a_no_op() {
    let (mut e, doc, id) = setup();
    let before = pixels(&e, doc, id);
    let mut warp = spec(WarpMode::Liquify);
    warp.points = vec![p(-20.0, -20.0), p(-10.0, -20.0)];
    e.execute(
        doc,
        Command::WarpStroke {
            id,
            mask: false,
            warp,
        },
    )
    .unwrap();
    assert_eq!(pixels(&e, doc, id), before);
    assert_eq!(e.state(doc).unwrap().undo_depth, 0);
    selected(&mut e, doc, 15.0, 16.0);
    let depth = e.state(doc).unwrap().undo_depth;
    e.execute(doc, command(id, WarpMode::Smudge)).unwrap();
    assert_eq!(pixels(&e, doc, id), before);
    assert_eq!(e.state(doc).unwrap().undo_depth, depth);
}

#[test]
fn invalid_settings_mask_and_large_reference_are_refused_atomically() {
    let (mut e, doc, id) = setup();
    let before = e.document(doc).unwrap().clone();
    let mut invalid = spec(WarpMode::Liquify);
    invalid.strength = f64::NAN;
    assert!(e
        .execute(
            doc,
            Command::WarpStroke {
                id,
                mask: false,
                warp: invalid
            }
        )
        .is_err());
    let err = e
        .execute(
            doc,
            Command::WarpStroke {
                id,
                mask: true,
                warp: spec(WarpMode::Smudge),
            },
        )
        .unwrap_err();
    assert!(err.to_string().contains(ops::warp::MASK_REFUSAL));
    assert!(e.document(doc).unwrap().same_content(&before));
    assert_eq!(e.state(doc).unwrap().undo_depth, 0);
    let mut large = before.clone();
    large.width = 3000;
    large.height = 2000;
    let big = e.insert_document(large);
    let err = e.execute(big, command(id, WarpMode::Liquify)).unwrap_err();
    assert!(err.to_string().contains(ops::warp::REFERENCE_LIMIT));
    assert_eq!(e.state(big).unwrap().undo_depth, 0);
}

#[test]
fn translated_rotated_flipped_layers_use_document_space_and_keep_appearance() {
    for (rotation, flip) in [(0.0, false), (90.0, false), (90.0, true)] {
        let (e, doc, id) = setup();
        let mut document = e.document(doc).unwrap().clone();
        document.width = 40;
        document.height = 40;
        let l = document.layer_mut(id).unwrap();
        l.transform.origin = p(10.0, 12.0);
        l.transform.rotation = rotation;
        l.transform.flip_x = flip;
        l.opacity = 0.25;
        l.blend_mode = BlendMode::Multiply;
        l.mask = Some(Mask {
            pixels: GrayRaster::from_bytes(20, 16, vec![0; 320]),
            enabled: true,
            placement: None,
            linked: Some(true),
        });
        let transform = l.transform;
        let mapping = transform.pixel_to_document(20, 16);
        let center = mapping.apply(p(3.5, 8.5));
        let a = p(center.x.floor(), center.y.floor());
        // Drag across the stripe in document coordinates; the rotated layer's
        // local x axis then points along document y (and reverses when flipped).
        let mut warp = spec(WarpMode::Smudge);
        warp.points = vec![
            a,
            if rotation == 0.0 {
                p(a.x + 3.0, a.y)
            } else {
                p(a.x, a.y + if flip { -3.0 } else { 3.0 })
            },
        ];
        let mut e = Engine::new();
        let doc = e.insert_document(document);
        let raw = ops::warp::source_plane(e.document(doc).unwrap(), id).unwrap();
        assert!(raw.bytes().chunks_exact(4).any(|p| p[0] > 0)); // Hidden mask/opacity are not baked.
        e.execute(
            doc,
            Command::WarpStroke {
                id,
                mask: false,
                warp,
            },
        )
        .unwrap();
        let l = e.document(doc).unwrap().layer(id).unwrap();
        assert_eq!(l.opacity, 0.25);
        assert_eq!(l.blend_mode, BlendMode::Multiply);
        assert_eq!(l.transform.rotation, rotation);
        assert_eq!(l.transform.flip_x, flip);
        assert!(l
            .mask
            .as_ref()
            .unwrap()
            .pixels
            .bytes()
            .iter()
            .all(|v| *v == 0));
        assert!(l
            .pixels
            .as_ref()
            .unwrap()
            .bytes()
            .chunks_exact(4)
            .any(|p| p[0] > 0 && p[0] < 255));
    }
}

#[test]
fn serialized_command_and_detached_job_equal_direct_execution() {
    for mode in [WarpMode::Liquify, WarpMode::Smudge] {
        let (mut direct, doc, id) = setup();
        let initial = direct.document(doc).unwrap().clone();
        let mut other = Engine::new();
        let other_doc = other.insert_document(initial);
        let command = command(id, mode);
        assert_eq!(
            command.action_name(),
            if mode == WarpMode::Liquify {
                "Liquify"
            } else {
                "Smudge"
            }
        );
        let json = serde_json::to_string(&command).unwrap();
        assert!(json.contains("WarpStroke"));
        let decoded: Command = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, command);
        direct.execute(doc, command.clone()).unwrap();
        let (input, px, mask, points) = other.job_input(other_doc, id).unwrap();
        let (output, result, mask, display) =
            run_edit_job(&input, px, mask, points.as_deref(), decoded, 1.0).unwrap();
        assert!(output.witness.is_some());
        other
            .install_job(other_doc, id, input.stamp, output, result, mask, display)
            .unwrap();
        assert_eq!(pixels(&other, other_doc, id), pixels(&direct, doc, id));
        assert_eq!(other.state(other_doc).unwrap().undo_depth, 1);
    }
}

#[test]
fn changed_source_transform_canvas_selection_or_mask_rejects_detached_result() {
    for change in 0..5 {
        let (mut e, doc, id) = setup();
        let (input, px, mask, points) = e.job_input(doc, id).unwrap();
        let (output, result, mask, display) = run_edit_job(
            &input,
            px,
            mask,
            points.as_deref(),
            command(id, WarpMode::Smudge),
            1.0,
        )
        .unwrap();
        let mutation = match change {
            0 => Command::InvertPixels { id, mask: false },
            1 => {
                let mut transform = e.document(doc).unwrap().layer(id).unwrap().transform;
                transform.origin.x += 1.0;
                Command::SetLayerTransform { id, transform }
            }
            2 => Command::CanvasSize {
                width: 21,
                height: 16,
                anchor: 0,
                fill: None,
            },
            3 => Command::SelectAll,
            _ => Command::AddMask {
                id,
                revealing: true,
            },
        };
        e.execute(doc, mutation).unwrap();
        let before = e.document(doc).unwrap().clone();
        let depth = e.state(doc).unwrap().undo_depth;
        let err = e
            .install_job(doc, id, input.stamp, output, result, mask, display)
            .unwrap_err();
        assert!(err.to_string().contains(LAYER_CHANGED));
        assert!(e.document(doc).unwrap().same_content(&before));
        assert_eq!(e.state(doc).unwrap().undo_depth, depth);
    }
}

#[test]
fn worker_preview_cancel_leaves_pixels_and_prior_history_untouched() {
    let (mut e, doc, id) = setup();
    e.execute(doc, Command::SelectAll).unwrap();
    let before = e.document(doc).unwrap().clone();
    let depth = e.state(doc).unwrap().undo_depth;
    let (input, px, mask, points) = e.job_input(doc, id).unwrap();
    let (output, result, mask, display) = run_edit_job(
        &input,
        px,
        mask,
        points.as_deref(),
        command(id, WarpMode::Smudge),
        1.0,
    )
    .unwrap();
    e.keep_job_preview(doc, id, input.stamp, output, result.unwrap(), mask, display)
        .unwrap();
    assert!(e.preview(doc).is_some());
    assert!(e.document(doc).unwrap().same_content(&before));
    e.undo(doc).unwrap();
    assert!(e.preview(doc).is_none());
    assert!(e.document(doc).unwrap().same_content(&before));
    assert_eq!(e.state(doc).unwrap().undo_depth, depth);
    e.undo(doc).unwrap();
    assert_eq!(e.state(doc).unwrap().undo_depth, depth - 1);
}
