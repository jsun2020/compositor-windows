use compositor_engine::ops::{adjust, hierarchy};
use compositor_engine::*;

fn canvas(color: [u8; 4]) -> (Document, uuid::Uuid) {
    let mut d = Document::new(2, 2);
    let l = Layer::with_pixels("Base", Raster::from_premultiplied(2, 2, color.repeat(4)), Point { x: 0.0, y: 0.0 });
    let id = l.id; d.active_layer_id = Some(id); d.layers.push(l);
    (d, id)
}
fn full(d: &Document) -> Raster { composite(d, Rect { x: 0.0, y: 0.0, width: 2.0, height: 2.0 }, 2, 2) }

#[test]
fn a_global_adjustment_changes_what_is_below_it_but_not_what_is_above() {
    let (mut d, base) = canvas([255, 255, 255, 255]);
    let a = adjust::add_adjustment_layer(&mut d, AdjustmentKind::Levels, 0, None).unwrap();
    let mut settings = LayerAdjustment::new(AdjustmentKind::Levels);
    settings.levels.ranges[0] = LevelRange { output_white: 0.0, ..LevelRange::default() };
    adjust::set_adjustment(&mut d, a, &settings).unwrap();
    assert_eq!(full(&d).bytes(), &[0, 0, 0, 255].repeat(4), "white is mapped to black");
    // A layer above the adjustment is untouched.
    let mut top = Layer::with_pixels("Top", Raster::from_premultiplied(1, 1, vec![255, 0, 0, 255]), Point { x: 0.0, y: 0.0 });
    top.transform.sampling = Sampling::Nearest;
    d.layers.push(top);
    assert_eq!(full(&d).pixel(0, 0), [255, 0, 0, 255]);
    assert_eq!(full(&d).pixel(1, 1), [0, 0, 0, 255]);
    // Hiding the adjustment brings the original back.
    d.layer_mut(a).unwrap().visible = false;
    assert_eq!(full(&d).pixel(1, 1), [255, 255, 255, 255]);
    let _ = base;
}

#[test]
fn opacity_and_a_mask_limit_an_adjustment_without_touching_the_pixels_below() {
    let (mut d, base) = canvas([255, 255, 255, 255]);
    let original = d.layer(base).unwrap().pixels.clone();
    let a = adjust::add_adjustment_layer(&mut d, AdjustmentKind::Levels, 0, None).unwrap();
    let mut settings = LayerAdjustment::new(AdjustmentKind::Levels);
    settings.levels.ranges[0] = LevelRange { output_white: 0.0, ..LevelRange::default() };
    adjust::set_adjustment(&mut d, a, &settings).unwrap();
    d.layer_mut(a).unwrap().opacity = 0.5;
    let half = full(&d).pixel(0, 0);
    assert!((half[0] as i64 - 128).abs() <= 2 && half[3] == 255, "{half:?}");
    d.layer_mut(a).unwrap().opacity = 1.0;
    compositor_engine::ops::masks::add_mask(&mut d, a, false).unwrap();
    assert_eq!(full(&d).pixel(0, 0), [255, 255, 255, 255], "a hiding mask blocks the adjustment entirely");
    assert_eq!(d.layer(base).unwrap().pixels, original, "the layer below is never rewritten");
}

#[test]
fn a_clipped_adjustment_changes_only_its_base() {
    let mut d = Document::new(2, 1);
    let bottom = Layer::with_pixels("Bottom", Raster::from_premultiplied(2, 1, vec![0, 0, 255, 255, 0, 0, 255, 255]), Point { x: 0.0, y: 0.0 });
    let bottom_id = bottom.id;
    let mut middle = Layer::with_pixels("Middle", Raster::from_premultiplied(1, 1, vec![0, 255, 0, 255]), Point { x: 0.0, y: 0.0 });
    middle.transform.sampling = Sampling::Nearest;
    let middle_id = middle.id;
    d.layers.push(bottom); d.layers.push(middle);
    d.active_layer_id = Some(middle_id);
    let a = adjust::add_adjustment_layer(&mut d, AdjustmentKind::Curves, 0, None).unwrap();
    let mut settings = LayerAdjustment::new(AdjustmentKind::Curves);
    settings.curves.channels[0] = vec![CurvePoint { x: 0.0, y: 255.0 }, CurvePoint { x: 255.0, y: 0.0 }];
    adjust::set_adjustment(&mut d, a, &settings).unwrap();
    hierarchy::toggle_clipping(&mut d, a).unwrap();
    let out = full(&d);
    assert_eq!(out.pixel(0, 0), [255, 0, 255, 255], "green inverts to magenta where the base covers");
    assert_eq!(out.pixel(1, 0), [0, 0, 255, 255], "the layer beside the base is untouched");
    // With the base hidden the clipped adjustment draws nothing at all.
    d.layer_mut(middle_id).unwrap().visible = false;
    assert_eq!(full(&d).pixel(0, 0), [0, 0, 255, 255]);
    let _ = bottom_id;
}

#[test]
fn an_adjustment_in_a_folder_reaches_beneath_it_and_a_folder_mask_limits_it() {
    let mut d = Document::new(2, 1);
    let outside = Layer::with_pixels("Outside", Raster::from_premultiplied(1, 1, vec![255, 255, 255, 255]), Point { x: 1.0, y: 0.0 });
    let outside_id = outside.id;
    let inside = Layer::with_pixels("Inside", Raster::from_premultiplied(1, 1, vec![255, 255, 255, 255]), Point { x: 0.0, y: 0.0 });
    let inside_id = inside.id;
    d.layers.push(outside); d.layers.push(inside);
    let folder = hierarchy::group_layers(&mut d, &[inside_id]).unwrap();
    d.active_layer_id = Some(inside_id);
    let a = adjust::add_adjustment_layer(&mut d, AdjustmentKind::Levels, 0, None).unwrap();
    assert_eq!(d.layer(a).unwrap().parent_id, Some(folder), "created inside the folder");
    let mut settings = LayerAdjustment::new(AdjustmentKind::Levels);
    settings.levels.ranges[0] = LevelRange { output_white: 0.0, ..LevelRange::default() };
    adjust::set_adjustment(&mut d, a, &settings).unwrap();
    // With no folder mask, the adjustment reaches beneath it just like the Mac's
    // LiveMaskRenderer: nothing isolates a folder's contents from what is below it.
    let out = full(&d);
    assert_eq!(out.pixel(0, 0), [0, 0, 0, 255], "the layer in the folder is adjusted");
    assert_eq!(out.pixel(1, 0), [0, 0, 0, 255], "the adjustment reaches beneath the folder too");
    // A folder limits an adjustment through its own mask (FolderMaskClip on the Mac), not by
    // isolating its contents: white over the left half lets the adjustment through, black blocks it.
    d.layer_mut(folder).unwrap().set_mask(Some(Mask { pixels: GrayRaster::from_bytes(2, 1, vec![255, 0]), enabled: true, placement: None, linked: None }));
    let out = full(&d);
    assert_eq!(out.pixel(0, 0), [0, 0, 0, 255], "the mask is white here: the adjustment still reaches");
    assert_eq!(out.pixel(1, 0), [255, 255, 255, 255], "the mask is black here: the adjustment does not reach");
    let _ = outside_id;
}

#[test]
fn the_plan_carries_the_adjustment_and_a_preview_edit_replaces_it() {
    let (mut d, _) = canvas([128, 128, 128, 255]);
    let a = adjust::add_adjustment_layer(&mut d, AdjustmentKind::Exposure, 0, None).unwrap();
    let plan = render_plan(&d, None);
    assert_eq!(plan.nodes.len(), 2);
    let PlanNode::Layer { draw } = &plan.nodes[1] else { panic!("{:?}", plan.nodes[1]) };
    assert_eq!(draw.id, a);
    assert_eq!(draw.adjustment.as_ref().unwrap().kind, AdjustmentKind::Exposure);
    assert!(plan.sources.is_empty());
    let json = serde_json::to_string(&plan).unwrap();
    assert!(json.contains(r#""adjustment":{"#) && json.contains(r#""kind":"Exposure""#));
    // A preview edit substitutes the settings without touching the document.
    let mut preview = LayerAdjustment::new(AdjustmentKind::Exposure);
    preview.exposure_settings = Some(ExposureSettings { exposure: 2.0, ..Default::default() });
    let edit = PreviewEdit::Adjustment { id: a, adjustment: preview.clone() };
    let previewed = render_plan(&d, Some(&edit));
    let PlanNode::Layer { draw } = &previewed.nodes[1] else { panic!() };
    assert_eq!(draw.adjustment.as_ref().unwrap().exposure().exposure, 2.0);
    let out = composite_edit(&d, Some(&edit), Rect { x: 0.0, y: 0.0, width: 2.0, height: 2.0 }, 2, 2);
    assert!(out.pixel(0, 0)[0] > 200, "the preview brightens: {:?}", out.pixel(0, 0));
    assert_eq!(d.layer(a).unwrap().extra.adjustment.as_ref().unwrap().exposure().exposure, 0.0, "the document is unchanged");
    let parsed: PreviewEdit = serde_json::from_str(&format!(r#"{{"kind":"adjustment","id":"{}","adjustment":{}}}"#, ids::upper_string(&a), serde_json::to_string(&preview).unwrap())).unwrap();
    assert_eq!(parsed, edit);
}

#[test]
fn an_adjustment_blends_with_its_own_blend_mode() {
    let (mut d, _) = canvas([200, 100, 50, 255]);
    let a = adjust::add_adjustment_layer(&mut d, AdjustmentKind::Levels, 0, None).unwrap();
    let mut settings = LayerAdjustment::new(AdjustmentKind::Levels);
    settings.levels.ranges[0] = LevelRange { output_black: 255.0, output_white: 255.0, ..LevelRange::default() };
    adjust::set_adjustment(&mut d, a, &settings).unwrap();
    d.layer_mut(a).unwrap().blend_mode = BlendMode::Multiply;
    // White adjusted, multiplied back over the original, leaves the original.
    let out = full(&d);
    assert_eq!(out.pixel(0, 0), [200, 100, 50, 255]);
}

/// Malformed shapes `is_valid` refuses and the table builders would index out of range with.
fn malformed() -> Vec<(&'static str, LayerAdjustment)> {
    let mut levels = LayerAdjustment::new(AdjustmentKind::Levels);
    levels.levels.ranges.truncate(2);
    levels.levels.channel = LevelsChannel::Blue;
    let mut curves = LayerAdjustment::new(AdjustmentKind::Curves);
    curves.curves.channels[0] = vec![CurvePoint { x: 0.0, y: 0.0 }];
    let mut few_curves = LayerAdjustment::new(AdjustmentKind::Curves);
    few_curves.curves.channels.truncate(1);
    let mut hsv = LayerAdjustment::new(AdjustmentKind::Hsv);
    hsv.hsv_settings = Some(HueSaturationSettings::new(500.0, 0.0, 0.0, false, ColorRange::Master));
    vec![("two levels ranges", levels), ("a one-point curve", curves), ("one curve channel", few_curves), ("hue 500", hsv)]
}

#[test]
fn a_malformed_adjustment_preview_shows_the_stored_adjustment_instead_of_trapping() {
    let (mut d, _) = canvas([200, 100, 50, 255]);
    let a = adjust::add_adjustment_layer(&mut d, AdjustmentKind::Levels, 0, None).unwrap();
    let mut stored = LayerAdjustment::new(AdjustmentKind::Levels);
    stored.levels.ranges[0] = LevelRange { output_white: 0.0, ..LevelRange::default() };
    adjust::set_adjustment(&mut d, a, &stored).unwrap();
    let expected = full(&d);
    for (label, bad) in malformed() {
        let edit = PreviewEdit::Adjustment { id: a, adjustment: bad };
        let plan = render_plan(&d, Some(&edit));
        let shown = plan.nodes.iter().find_map(|n| match n { PlanNode::Layer { draw } => draw.adjustment.clone(), _ => None });
        assert_eq!(shown.as_ref(), Some(&stored), "{label}: the plan falls back to the stored adjustment");
        assert_eq!(composite_edit(&d, Some(&edit), Rect { x: 0.0, y: 0.0, width: 2.0, height: 2.0 }, 2, 2).bytes(), expected.bytes(), "{label}");
    }
    // A valid edit still shows through.
    let edit = PreviewEdit::Adjustment { id: a, adjustment: LayerAdjustment::new(AdjustmentKind::Levels) };
    assert_eq!(composite_edit(&d, Some(&edit), Rect { x: 0.0, y: 0.0, width: 2.0, height: 2.0 }, 2, 2).pixel(0, 0), [200, 100, 50, 255]);
}

#[test]
fn the_gpu_tables_are_empty_for_a_malformed_adjustment() {
    for (label, bad) in malformed() {
        assert!(gpu_lut(&bad).is_empty(), "{label}");
        assert!(gpu_hue_response(&bad).is_empty(), "{label}");
    }
    assert_eq!(gpu_lut(&LayerAdjustment::new(AdjustmentKind::Levels)).len(), 256 * 4);
    assert_eq!(gpu_hue_response(&LayerAdjustment::new(AdjustmentKind::Hsv)).len(), 361 * 4);
    assert!(gpu_lut(&LayerAdjustment::new(AdjustmentKind::Hsv)).is_empty(), "no table for Hue/Saturation");
}