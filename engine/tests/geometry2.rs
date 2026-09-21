use compositor_engine::*;

fn t(x: f64, y: f64, w: f64, h: f64, rot: f64) -> LayerTransform {
    let mut t = LayerTransform::axis_aligned(Point { x, y }, Size { width: w, height: h });
    t.rotation = rot; t
}
fn near(a: f64, b: f64) -> bool { (a - b).abs() < 1e-6 }

#[test]
fn following_a_plain_move_translates() {
    let mask = t(10.0, 10.0, 50.0, 20.0, 15.0);
    let moved = mask.following(&t(0.0, 0.0, 100.0, 80.0, 0.0), &t(30.0, -5.0, 100.0, 80.0, 0.0));
    assert!(near(moved.origin.x, 40.0) && near(moved.origin.y, 5.0) && near(moved.rotation, 15.0));
}

#[test]
fn following_a_scale_and_rotation_keeps_relative_placement() {
    let old = t(0.0, 0.0, 100.0, 50.0, 0.0);
    let new = t(20.0, 20.0, 200.0, 100.0, 90.0);
    let inner = t(25.0, 0.0, 50.0, 50.0, 0.0); // the right half of `old`
    let moved = inner.following(&old, &new);
    // The inner box's centre (50, 25) is at unit (0.5, 0.5) of old, so it lands on new's centre.
    let c = moved.center();
    assert!(near(c.x, 120.0) && near(c.y, 70.0), "{c:?}");
    assert!(near(moved.size.width, 100.0) && near(moved.size.height, 100.0), "{:?}", moved.size);
    assert!(near(moved.rotation, 90.0), "{}", moved.rotation);
    assert!(!moved.flip_y);
}

#[test]
fn placing_keeps_horizontal_flip_and_nearest_turn() {
    let mut base = t(0.0, 0.0, 10.0, 10.0, 350.0);
    base.flip_x = true;
    let map = LayerTransform::axis_aligned(Point { x: 0.0, y: 0.0 }, Size { width: 10.0, height: 10.0 }).unit_to_document();
    let placed = base.placing(map);
    assert!(placed.flip_x && near(placed.rotation, 360.0), "{}", placed.rotation);
}

#[test]
fn scale_percent_and_rounding() {
    let mut s = t(0.0, 0.0, 200.0, 300.0, 30.0);
    let pixel = Size { width: 100.0, height: 100.0 };
    assert!(near(s.scale_percent(pixel), 200.0));
    let half = s.scaled_to_percent(50.0, pixel);
    assert!(near(half.size.width, 50.0) && near(half.size.height, 50.0));
    assert!(near(half.center().x, s.center().x) && near(half.center().y, s.center().y) && near(half.rotation, 30.0));
    s.origin = Point { x: 1.4, y: -2.6 }; s.rotation = 29.6; s.size.width = 0.3;
    let r = s.rounded();
    assert_eq!((r.origin.x, r.origin.y, r.rotation, r.size.width), (1.0, -3.0, 30.0, 1.0));
    assert!(t(100.0, 200.0, 100.0, 50.0, 90.0).contains(Point { x: 150.0, y: 265.0 }));
    assert!(!t(100.0, 200.0, 100.0, 50.0, 90.0).contains(Point { x: 190.0, y: 225.0 }));
}

#[test]
fn homography_hits_corners_and_rejects_twisted_shapes() {
    let shape = [Point { x: 10.0, y: 10.0 }, Point { x: 60.0, y: 10.0 }, Point { x: 30.0, y: 30.0 }, Point { x: 10.0, y: 30.0 }];
    let h = Homography::unit_to(&shape);
    for (u, c) in [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)].iter().zip(shape.iter()) {
        let p = h.apply(Point { x: u.0, y: u.1 });
        assert!(near(p.x, c.x) && near(p.y, c.y), "{u:?} -> {p:?}");
    }
    let inv = h.invert().unwrap();
    let back = inv.apply(Point { x: 45.0, y: 20.0 });
    let fwd = h.apply(back);
    assert!(near(fwd.x, 45.0) && near(fwd.y, 20.0));
    assert!(Homography::is_usable(&shape));
    assert!(!Homography::is_usable(&[shape[0], shape[2], shape[1], shape[3]]));
    assert!(!Homography::is_usable(&[shape[0], shape[0], shape[2], shape[3]]));
}

#[test]
fn mask_placement_rule() {
    let old = t(0.0, 0.0, 400.0, 200.0, 0.0);
    let new = t(100.0, 0.0, 400.0, 200.0, 0.0);
    let pixels = |w, h| GrayRaster::from_bytes(w, h, vec![255; (w * h) as usize]);
    let linked = Mask { pixels: pixels(4, 2), enabled: true, placement: None, linked: None };
    assert_eq!(linked.follow(&old, &new), None, "a linked mask covering its layer keeps covering it");
    let unlinked = Mask { pixels: pixels(4, 2), enabled: true, placement: None, linked: Some(false) };
    assert_eq!(unlinked.follow(&old, &new), Some(old), "an unlinked mask stays on the canvas");
    let placed = Mask { pixels: pixels(4, 2), enabled: true, placement: Some(t(50.0, 0.0, 400.0, 200.0, 0.0)), linked: None };
    let moved = placed.follow(&old, &new).unwrap();
    assert!(near(moved.origin.x, 150.0));
    let uniform = Mask { pixels: pixels(1, 1), enabled: true, placement: None, linked: Some(false) };
    assert_eq!(uniform.follow(&old, &new), None, "a uniform mask looks the same anywhere");
    let back = Mask { pixels: pixels(4, 2), enabled: true, placement: Some(new), linked: Some(false) };
    assert_eq!(back.follow(&old, &new), None, "a placement equal to the layer collapses");
}

#[test]
fn mask_background_is_the_edge_majority() {
    let mut data = vec![0u8; 16];
    for i in [0, 1, 2, 3, 4, 7, 8, 11, 12, 13, 14] { data[i] = 255; } // 11 of 12 edge pixels white
    assert_eq!(Mask { pixels: GrayRaster::from_bytes(4, 4, data.clone()), enabled: true, placement: None, linked: None }.background(), 255);
    for v in &mut data { *v = 0; }
    assert_eq!(Mask { pixels: GrayRaster::from_bytes(4, 4, data), enabled: true, placement: None, linked: None }.background(), 0);
}

#[test]
fn document_hierarchy_helpers() {
    let mut d = Document::new(10, 10);
    let mut g = Layer::blank("Folder", d.size()); g.is_group = true;
    let mut a = Layer::blank("A", d.size()); a.parent_id = Some(g.id);
    let mut b = Layer::with_pixels("B", Raster::new_transparent(2, 2), Point { x: 0.0, y: 0.0 }); b.parent_id = Some(g.id);
    let c = Layer::with_pixels("C", Raster::new_transparent(2, 2), Point { x: 0.0, y: 0.0 });
    let (gid, aid, bid, cid) = (g.id, a.id, b.id, c.id);
    d.layers = vec![g, a, b, c];
    assert_eq!(d.descendants(gid), vec![aid, bid]);
    assert_eq!(d.siblings(Some(gid)), vec![aid, bid]);
    assert_eq!(d.siblings(None), vec![gid, cid]);
    assert_eq!(d.render_ids(), vec![aid, bid, cid]);
    d.layer_mut(gid).unwrap().visible = false;
    assert_eq!(d.render_ids(), vec![cid]);
    assert!(!d.visible_ids().contains(&aid) && d.visible_ids().contains(&cid));
    let mut l = d.layer_mut(cid).unwrap().clone();
    let before = l.mask_revision;
    l.set_mask(Some(Mask { pixels: GrayRaster::from_bytes(1, 1, vec![255]), enabled: true, placement: None, linked: None }));
    assert_eq!(l.mask_revision, before + 1);
    l.mask_mut().unwrap().enabled = false;
    assert_eq!(l.mask_revision, before + 2);
}
