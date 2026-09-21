use compositor_engine::*;

fn red(w: u32, h: u32) -> Raster { Raster::from_premultiplied(w, h, [255u8, 0, 0, 255].repeat((w * h) as usize)) }
fn seed(e: &mut Engine, id: uuid::Uuid, name: &str, x: f64, y: f64) -> uuid::Uuid {
    let bytes = encode_png(&red(4, 4), 72.0).unwrap();
    e.import_image(Some(id), &bytes, name, Some(Point { x: x + 2.0, y: y + 2.0 })).unwrap();
    e.state(id).unwrap().active_layer_id.unwrap()
}

#[test]
fn phase2_commands_round_trip_json_and_undo() {
    let mut e = Engine::new();
    let doc = e.new_document(20, 20, false).unwrap();
    let a = seed(&mut e, doc, "A", 0.0, 0.0);
    let b = seed(&mut e, doc, "B", 8.0, 8.0);
    let ida = ids::upper_string(&a); let idb = ids::upper_string(&b);
    let run = |e: &mut Engine, json: String| { let c: Command = serde_json::from_str(&json).unwrap(); e.execute(doc, c).unwrap() };
    run(&mut e, format!(r#"{{"type":"SetLayerOpacity","id":"{ida}","opacity":0.5}}"#));
    run(&mut e, format!(r#"{{"type":"SetLayerBlendMode","id":"{ida}","mode":"Multiply"}}"#));
    run(&mut e, format!(r#"{{"type":"GroupLayers","ids":["{ida}","{idb}"]}}"#));
    let s = e.state(doc).unwrap();
    assert_eq!(s.layers.len(), 3);
    let folder = s.layers.iter().find(|l| l.is_group).unwrap();
    assert_eq!(s.layers.iter().filter(|l| l.parent_id == Some(folder.id)).count(), 2);
    run(&mut e, format!(r#"{{"type":"AddMask","id":"{ida}","revealing":false}}"#));
    let la = e.state(doc).unwrap().layers.iter().find(|l| l.id == a).unwrap().clone();
    assert!(la.has_mask && !la.mask_linked == false && la.mask_enabled && la.mask_width == 1 && la.mask_background == 0);
    run(&mut e, format!(r#"{{"type":"SetLayerTransform","id":"{idb}","transform":{{"origin":[10,10],"size":[4,4],"rotation":45,"flipX":false,"flipY":false,"sampling":"Nearest"}}}}"#));
    assert_eq!(e.state(doc).unwrap().layers.iter().find(|l| l.id == b).unwrap().transform.rotation, 45.0);
    run(&mut e, format!(r#"{{"type":"DistortLayer","id":"{idb}","transform":{{"origin":[10,10],"size":[4,4]}},"corners":[[10,10],[18,10],[16,14],[10,14]]}}"#));
    let lb = e.state(doc).unwrap().layers.iter().find(|l| l.id == b).unwrap().clone();
    assert_eq!((lb.transform.rotation, lb.pixels_revision), (0.0, 2));
    run(&mut e, format!(r#"{{"type":"ToggleClipping","id":"{idb}"}}"#));
    assert_eq!(e.state(doc).unwrap().layers.iter().find(|l| l.id == b).unwrap().mask_source_id, Some(a));
    assert_eq!(e.clip_dependents(doc, &[a]).unwrap(), vec![b]);
    run(&mut e, format!(r#"{{"type":"DeleteLayers","ids":["{ida}"],"bake":true}}"#));
    assert_eq!(e.state(doc).unwrap().layers.iter().find(|l| l.id == b).unwrap().pixels_revision, 3);
    let steps = 8;
    for _ in 0..steps { e.undo(doc).unwrap(); }
    let s = e.state(doc).unwrap();
    assert_eq!(s.layers.len(), 2);
    assert!(s.can_undo, "the two seed imports are still undoable");
    assert_eq!(e.merge_action(doc, &[b]).unwrap(), Some("Merge Down"));
    assert!(e.group_box(doc, &[a, b]).unwrap().is_some());
}

#[test]
fn render_plan_and_composite_edit_through_the_facade() {
    let mut e = Engine::new();
    let doc = e.new_document(20, 20, false).unwrap();
    let a = seed(&mut e, doc, "A", 0.0, 0.0);
    let draft = LayerTransform::axis_aligned(Point { x: 10.0, y: 10.0 }, Size { width: 4.0, height: 4.0 });
    let plan = e.render_plan(doc, Some(&PreviewEdit::Layer { id: a, draft, corners: None })).unwrap();
    let PlanNode::Layer { draw } = &plan.nodes[0] else { panic!() };
    assert_eq!(draw.transform, draft);
    let out = e.composite_edit(doc, Some(&PreviewEdit::Layer { id: a, draft, corners: None }), Rect { x: 0.0, y: 0.0, width: 20.0, height: 20.0 }, 20, 20).unwrap();
    assert_eq!(out.pixel(11, 11)[3], 255);
    assert_eq!(out.pixel(1, 1)[3], 0);
    let json = serde_json::to_string(&plan).unwrap();
    assert!(json.contains("\"coverages\":[]"));
}
