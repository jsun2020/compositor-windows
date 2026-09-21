use compositor_engine::*;
use uuid::Uuid;

fn px(name: &str) -> Layer { Layer::with_pixels(name, Raster::new_transparent(4, 4), Point { x: 0.0, y: 0.0 }) }
fn mask(w: u32, h: u32, v: u8) -> Mask { Mask { pixels: GrayRaster::from_bytes(w, h, vec![v; (w * h) as usize]), enabled: true, placement: None, linked: None } }
fn draws(plan: &RenderPlan) -> Vec<Uuid> {
    plan.nodes.iter().flat_map(|n| match n { PlanNode::Layer { draw } => vec![draw.id], PlanNode::Stack { base, children, .. } => std::iter::once(base.id).chain(children.iter().map(|c| c.id)).collect() }).collect()
}

#[test]
fn plain_layers_in_order_with_folder_and_own_coverages() {
    let mut d = Document::new(10, 10);
    let mut outer = Layer::blank("Outer", d.size()); outer.is_group = true; outer.set_mask(Some(mask(1, 1, 0)));
    let mut inner = Layer::blank("Inner", d.size()); inner.is_group = true; inner.parent_id = Some(outer.id);
    let mut a = px("A"); a.parent_id = Some(inner.id); a.set_mask(Some(mask(2, 2, 128)));
    let b = px("B");
    let (oid, aid, bid) = (outer.id, a.id, b.id);
    d.layers = vec![outer, inner, a, b];
    let plan = render_plan(&d, None);
    assert_eq!(draws(&plan), vec![aid, bid]);
    let PlanNode::Layer { draw } = &plan.nodes[0] else { panic!() };
    assert_eq!(draw.coverages.len(), 2, "own mask then the outer folder's mask");
    assert_eq!(draw.coverages[0].layer_id, aid);
    assert_eq!((draw.coverages[0].width, draw.coverages[0].height), (2, 2));
    assert_eq!(draw.coverages[1].layer_id, oid);
    assert_eq!(draw.coverages[1].background, 0);
    assert_eq!(draw.coverages[1].placement, d.layer(oid).unwrap().transform);
    let PlanNode::Layer { draw } = &plan.nodes[1] else { panic!() };
    assert!(draw.coverages.is_empty());
    assert!(plan.sources.is_empty());
}

#[test]
fn disabled_masks_and_hidden_layers_are_left_out() {
    let mut d = Document::new(10, 10);
    let mut a = px("A"); a.set_mask(Some(Mask { enabled: false, ..mask(2, 2, 0) }));
    let mut b = px("B"); b.visible = false;
    let aid = a.id;
    d.layers = vec![a, b];
    let plan = render_plan(&d, None);
    assert_eq!(draws(&plan), vec![aid]);
    let PlanNode::Layer { draw } = &plan.nodes[0] else { panic!() };
    assert!(draw.coverages.is_empty());
}

#[test]
fn clipping_stack_and_non_stack_sources() {
    let mut d = Document::new(10, 10);
    let base = px("Base");
    let mut c1 = px("C1"); c1.mask_source_id = Some(base.id);
    let mut c2 = px("C2"); c2.mask_source_id = Some(base.id); c2.blend_mode = BlendMode::Multiply;
    let mut hidden_src = px("Hidden"); hidden_src.visible = false;
    let mut t = px("T"); t.mask_source_id = Some(hidden_src.id);
    let (bid, c1id, c2id, hid, tid) = (base.id, c1.id, c2.id, hidden_src.id, t.id);
    d.layers = vec![t, hidden_src, base, c1, c2];
    let plan = render_plan(&d, None);
    // T draws clipped to the hidden source (not a stack: the source is above it and hidden).
    let PlanNode::Layer { draw } = &plan.nodes[0] else { panic!("first node: {:?}", plan.nodes[0]) };
    assert_eq!(draw.id, tid);
    assert_eq!(draw.clip, Some(hid));
    // Base + C1 + C2 form a stack.
    let PlanNode::Stack { base, children, .. } = &plan.nodes[1] else { panic!("second node: {:?}", plan.nodes[1]) };
    assert_eq!(base.id, bid);
    assert_eq!(children.iter().map(|c| c.id).collect::<Vec<_>>(), vec![c1id, c2id]);
    assert_eq!(children[1].blend, BlendMode::Multiply);
    assert!(children.iter().all(|c| c.clip.is_none()));
    assert_eq!(plan.nodes.len(), 2);
    assert_eq!(plan.sources.iter().map(|s| s.id).collect::<Vec<_>>(), vec![hid], "hidden source still supplies coverage");
}

#[test]
fn a_hidden_child_breaks_the_stack_and_chains_add_sources() {
    let mut d = Document::new(10, 10);
    let base = px("Base");
    let mut c1 = px("C1"); c1.mask_source_id = Some(base.id); c1.visible = false;
    let mut c2 = px("C2"); c2.mask_source_id = Some(base.id);
    let mut s2 = px("S2");
    s2.mask_source_id = Some(c2.id);
    let mut t = px("T"); t.mask_source_id = Some(s2.id);
    let (bid, c2id, s2id, tid) = (base.id, c2.id, s2.id, t.id);
    d.layers = vec![base, c1, c2, s2, t];
    let plan = render_plan(&d, None);
    // C1 hidden: the stack is base + C2 (C2 directly follows base in render order).
    let PlanNode::Stack { base, children, .. } = &plan.nodes[0] else { panic!() };
    assert_eq!((base.id, children.len()), (bid, 1));
    assert_eq!(children[0].id, c2id);
    // S2 is clipped to C2 (a stacked child, so S2 is not part of the stack); T clips to S2 which clips to C2.
    let ids: Vec<Uuid> = plan.sources.iter().map(|s| s.id).collect();
    assert!(ids.contains(&s2id) && ids.contains(&c2id));
    let s2 = plan.sources.iter().find(|s| s.id == s2id).unwrap();
    assert_eq!(s2.clip, Some(c2id));
    let _ = tid;
}

#[test]
fn preview_edits_change_displayed_transforms_and_masks() {
    let mut d = Document::new(100, 100);
    let mut a = px("A");
    a.set_mask(Some(mask(4, 4, 255)));
    let mut b = px("B"); b.transform.origin = Point { x: 10.0, y: 0.0 };
    let (aid, bid) = (a.id, b.id);
    d.layers = vec![a, b];
    let draft = LayerTransform::axis_aligned(Point { x: 20.0, y: 30.0 }, Size { width: 8.0, height: 8.0 });
    let plan = render_plan(&d, Some(&PreviewEdit::Layer { id: aid, draft, corners: None }));
    let PlanNode::Layer { draw } = &plan.nodes[0] else { panic!() };
    assert_eq!(draw.transform, draft);
    assert_eq!(draw.coverages[0].placement, draft, "a linked covering mask follows");
    // Unlinked: the mask stays where it was.
    d.layer_mut(aid).unwrap().mask_mut().unwrap().linked = Some(false);
    let plan = render_plan(&d, Some(&PreviewEdit::Layer { id: aid, draft, corners: None }));
    let PlanNode::Layer { draw } = &plan.nodes[0] else { panic!() };
    assert_eq!(draw.coverages[0].placement, d.layer(aid).unwrap().transform);
    // Mask edit moves only the mask.
    let plan = render_plan(&d, Some(&PreviewEdit::Mask { id: aid, draft }));
    let PlanNode::Layer { draw } = &plan.nodes[0] else { panic!() };
    assert_eq!(draw.transform, d.layer(aid).unwrap().transform);
    assert_eq!(draw.coverages[0].placement, draft);
    // Group edit: both layers follow the box.
    let bounds = LayerTransform::axis_aligned(Point { x: 0.0, y: 0.0 }, Size { width: 14.0, height: 4.0 });
    let moved = LayerTransform::axis_aligned(Point { x: 0.0, y: 0.0 }, Size { width: 28.0, height: 8.0 });
    let plan = render_plan(&d, Some(&PreviewEdit::Group { ids: vec![aid, bid], bounds, draft: moved, corners: None }));
    let PlanNode::Layer { draw: db } = &plan.nodes[1] else { panic!() };
    assert_eq!(db.id, bid);
    assert!((db.transform.origin.x - 20.0).abs() < 1e-6 && (db.transform.size.width - 8.0).abs() < 1e-6);
    // Distortion: corners carried; a linked covering mask carries the corners too.
    d.layer_mut(aid).unwrap().mask_mut().unwrap().linked = None;
    let shape = [Point { x: 0.0, y: 0.0 }, Point { x: 8.0, y: 0.0 }, Point { x: 6.0, y: 4.0 }, Point { x: 0.0, y: 4.0 }];
    let plan = render_plan(&d, Some(&PreviewEdit::Layer { id: aid, draft: d.layer(aid).unwrap().transform, corners: Some(shape) }));
    let PlanNode::Layer { draw } = &plan.nodes[0] else { panic!() };
    assert_eq!(draw.corners, Some(shape));
    assert_eq!(draw.coverages[0].corners, Some(shape));
}

#[test]
fn group_distortion_carries_linked_covering_masks_and_leaves_placed_masks() {
    let mut d = Document::new(100, 100);
    let mut a = px("A");
    a.set_mask(Some(mask(4, 4, 255)));
    let mut b = px("B");
    b.transform.origin = Point { x: 10.0, y: 0.0 };
    let mut b_mask_placement = b.transform;
    b_mask_placement.origin.x += 2.0;
    b.set_mask(Some(Mask { placement: Some(b_mask_placement), ..mask(4, 4, 128) }));
    let (aid, bid) = (a.id, b.id);
    d.layers = vec![a, b];
    let bounds = LayerTransform::axis_aligned(Point { x: 0.0, y: 0.0 }, Size { width: 14.0, height: 4.0 });
    let corners = [Point { x: 0.0, y: 0.0 }, Point { x: 14.0, y: 0.0 }, Point { x: 12.0, y: 4.0 }, Point { x: 0.0, y: 4.0 }];
    let plan = render_plan(&d, Some(&PreviewEdit::Group { ids: vec![aid, bid], bounds, draft: bounds, corners: Some(corners) }));
    assert_eq!(plan.nodes.len(), 2);
    let PlanNode::Layer { draw: da } = &plan.nodes[0] else { panic!() };
    assert_eq!(da.id, aid);
    assert!(da.corners.is_some(), "A's own draw carries the distortion");
    assert_eq!(da.coverages[0].corners, da.corners, "a linked covering mask carries the same corners as its layer");
    assert_eq!(da.coverages[0].placement, da.transform, "its placement is the layer's own displayed transform");
    let PlanNode::Layer { draw: db } = &plan.nodes[1] else { panic!() };
    assert_eq!(db.id, bid);
    assert!(db.corners.is_some(), "B's own draw carries the distortion");
    assert_eq!(db.coverages[0].corners, None, "a placed mask keeps its affine placement, no distortion");
    assert_eq!(db.coverages[0].placement, b_mask_placement);
}

#[test]
fn plan_serialises_camel_case_with_uppercase_ids() {
    let mut d = Document::new(10, 10);
    let a = px("A");
    let upper = ids::upper_string(&a.id);
    d.layers = vec![a];
    let json = serde_json::to_string(&render_plan(&d, None)).unwrap();
    assert!(json.contains("\"kind\":\"layer\""));
    assert!(json.contains("\"pixelsRevision\""));
    assert!(json.contains(&upper));
    let edit: PreviewEdit = serde_json::from_str(&format!(r#"{{"kind":"group","ids":["{upper}"],"box":{{"origin":[0,0],"size":[4,4]}},"draft":{{"origin":[1,1],"size":[4,4]}}}}"#)).unwrap();
    assert!(matches!(edit, PreviewEdit::Group { .. }));
}
