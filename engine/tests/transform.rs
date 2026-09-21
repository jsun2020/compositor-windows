use compositor_engine::ops::transform::*;
use compositor_engine::*;

fn near(a: f64, b: f64) -> bool { (a - b).abs() < 1e-6 }
fn layer(name: &str, x: f64, y: f64, w: u32, h: u32) -> Layer { Layer::with_pixels(name, Raster::new_transparent(w, h), Point { x, y }) }
fn t(x: f64, y: f64, w: f64, h: f64) -> LayerTransform { LayerTransform::axis_aligned(Point { x, y }, Size { width: w, height: h }) }
fn gray(w: u32, h: u32) -> GrayRaster { GrayRaster::from_bytes(w, h, vec![255; (w * h) as usize]) }

#[test]
fn set_transform_moves_linked_mask_and_leaves_unlinked_mask() {
    let mut d = Document::new(400, 200);
    let mut l = layer("L", 0.0, 0.0, 400, 200);
    l.set_mask(Some(Mask { pixels: gray(400, 200), enabled: true, placement: None, linked: None }));
    let id = l.id; d.layers.push(l);
    let moved = t(100.0, 0.0, 400.0, 200.0);
    set_transform(&mut d, id, moved).unwrap();
    assert_eq!(d.layer(id).unwrap().transform, moved);
    assert_eq!(d.layer(id).unwrap().mask.as_ref().unwrap().placement, None, "a linked mask keeps covering its layer");
    d.layer_mut(id).unwrap().mask_mut().unwrap().linked = Some(false);
    let moved2 = t(150.0, 0.0, 400.0, 200.0);
    set_transform(&mut d, id, moved2).unwrap();
    assert_eq!(d.layer(id).unwrap().mask.as_ref().unwrap().placement, Some(moved), "an unlinked mask stays on the canvas");
    let mut bad = moved2; bad.size.width = 0.0;
    assert!(set_transform(&mut d, id, bad).is_err());
    let mut g = Layer::blank("G", d.size()); g.is_group = true; let gid = g.id; d.layers.push(g);
    assert!(set_transform(&mut d, gid, moved).is_err());
}

#[test]
fn mask_moves_alone_and_relinks() {
    let mut d = Document::new(400, 200);
    let mut l = layer("L", 0.0, 0.0, 400, 200);
    l.set_mask(Some(Mask { pixels: gray(400, 200), enabled: true, placement: None, linked: Some(false) }));
    let id = l.id; d.layers.push(l);
    let moved = t(100.0, 0.0, 400.0, 200.0);
    set_mask_placement(&mut d, id, moved).unwrap();
    assert_eq!(d.layer(id).unwrap().transform, t(0.0, 0.0, 400.0, 200.0));
    assert_eq!(d.layer(id).unwrap().mask.as_ref().unwrap().placement, Some(moved));
    set_mask_placement(&mut d, id, t(0.0, 0.0, 400.0, 200.0)).unwrap();
    assert_eq!(d.layer(id).unwrap().mask.as_ref().unwrap().placement, None, "back on its layer collapses to none");
    set_mask_placement(&mut d, id, moved).unwrap();
    d.layer_mut(id).unwrap().mask_mut().unwrap().linked = None;
    set_transform(&mut d, id, t(50.0, 0.0, 400.0, 200.0)).unwrap();
    let p = d.layer(id).unwrap().mask.as_ref().unwrap().placement.unwrap();
    assert!(near(p.origin.x, 150.0) && near(p.origin.y, 0.0), "relinked, the placed mask follows: {p:?}");
}

#[test]
fn group_transform_and_box() {
    let mut d = Document::new(400, 200);
    let a = layer("A", 0.0, 0.0, 100, 50); let b = layer("B", 100.0, 50.0, 100, 50);
    let (aid, bid) = (a.id, b.id);
    d.layers.push(a); d.layers.push(b);
    let bounds = group_box(&d, &[aid, bid]).unwrap();
    assert_eq!(bounds, t(0.0, 0.0, 200.0, 100.0));
    let draft = t(10.0, 10.0, 400.0, 200.0);
    transform_group(&mut d, &[aid, bid], &bounds, &draft).unwrap();
    let ta = d.layer(aid).unwrap().transform; let tb = d.layer(bid).unwrap().transform;
    assert!(near(ta.origin.x, 10.0) && near(ta.size.width, 200.0) && near(ta.size.height, 100.0));
    assert!(near(tb.origin.x, 210.0) && near(tb.origin.y, 110.0));
    nudge(&mut d, &[aid, bid], 1.0, -2.0).unwrap();
    assert!(near(d.layer(aid).unwrap().transform.origin.x, 11.0) && near(d.layer(bid).unwrap().transform.origin.y, 108.0));
}

#[test]
fn flip_one_layer_about_itself_and_several_about_their_box() {
    let mut d = Document::new(400, 200);
    let mut a = layer("A", 0.0, 0.0, 100, 50); a.transform.rotation = 30.0;
    let b = layer("B", 300.0, 0.0, 100, 50);
    let (aid, bid) = (a.id, b.id);
    d.layers.push(a); d.layers.push(b);
    flip_layers(&mut d, &[aid], true).unwrap();
    let ta = d.layer(aid).unwrap().transform;
    assert!(ta.flip_x && near(ta.rotation, -30.0) && near(ta.origin.x, 0.0));
    let axis = group_box(&d, &[aid, bid]).unwrap().center().x;
    // A still carries rotation -30, so the box starts left of 0 and the axis sits below 200.
    assert!(axis < 200.0 && axis > 195.0, "{axis}");
    flip_layers(&mut d, &[aid, bid], true).unwrap();
    assert!(near(d.layer(aid).unwrap().transform.center().x, 2.0 * axis - 50.0));
    assert!(near(d.layer(bid).unwrap().transform.center().x, 2.0 * axis - 350.0));
    assert!(!d.layer(aid).unwrap().transform.flip_x && d.layer(bid).unwrap().transform.flip_x);
}
