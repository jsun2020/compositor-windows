use compositor_engine::ops::{hierarchy, merge};
use compositor_engine::*;

fn solid(w: u32, h: u32, rgb: [u8; 3]) -> Raster { Raster::from_premultiplied(w, h, [rgb[0], rgb[1], rgb[2], 255].repeat((w * h) as usize)) }
fn full(d: &Document) -> Raster { composite(d, Rect { x: 0.0, y: 0.0, width: d.width as f64, height: d.height as f64 }, d.width, d.height) }

#[test]
fn merge_down_bakes_blend_and_trims() {
    let mut d = Document::new(10, 10);
    let mut below = Layer::with_pixels("Below", solid(4, 4, [102, 102, 102]), Point { x: 2.0, y: 2.0 }); below.transform.sampling = Sampling::Nearest;
    let mut above = Layer::with_pixels("Above", solid(2, 2, [204, 204, 204]), Point { x: 3.0, y: 3.0 }); above.transform.sampling = Sampling::Nearest;
    above.blend_mode = BlendMode::Multiply;
    let (bid, aid) = (below.id, above.id);
    d.layers = vec![below, above];
    d.active_layer_id = Some(aid);
    let before = full(&d);
    let plan = merge::merge_plan(&d, &[aid]).unwrap();
    assert_eq!((plan.action, plan.name.as_str(), plan.anchor), ("Merge Down", "Below", aid));
    let merged = merge::merge(&mut d.clone(), &[aid]).map(|_| ()).err();
    assert!(merged.is_none());
    let result = merge::merge(&mut d, &[aid]).unwrap();
    assert_eq!(d.layers.len(), 1);
    assert_eq!(d.layers[0].id, result);
    assert_eq!(d.layers[0].name, "Below");
    assert_eq!(d.active_layer_id, Some(result));
    assert_eq!(d.layers[0].transform.origin, Point { x: 2.0, y: 2.0 });
    assert_eq!(d.layers[0].pixels.as_ref().unwrap().width, 4);
    assert_eq!(full(&d).bytes(), before.bytes(), "the merged layer looks the same");
    assert!(((full(&d).pixel(3, 3)[0] as f64 / 255.0) - 0.32).abs() < 0.02);
    let _ = bid;
}

#[test]
fn merge_group_removes_the_folder_and_merge_layers_keeps_position() {
    let mut d = Document::new(10, 10);
    let bottom = Layer::with_pixels("Bottom", solid(10, 10, [0, 0, 255]), Point { x: 0.0, y: 0.0 });
    let a = Layer::with_pixels("A", solid(2, 2, [255, 0, 0]), Point { x: 0.0, y: 0.0 });
    let b = Layer::with_pixels("B", solid(2, 2, [0, 255, 0]), Point { x: 5.0, y: 5.0 });
    let top = Layer::with_pixels("Top", solid(1, 1, [255, 255, 255]), Point { x: 9.0, y: 9.0 });
    let (aid, bid, tid) = (a.id, b.id, top.id);
    d.layers = vec![bottom, a, b, top];
    let folder = hierarchy::group_layers(&mut d, &[aid, bid]).unwrap();
    let before = full(&d);
    assert_eq!(merge::merge_plan(&d, &[folder]).unwrap().action, "Merge Group");
    let result = merge::merge(&mut d, &[folder]).unwrap();
    assert!(d.layer(folder).is_none() && d.layer(aid).is_none());
    assert_eq!(d.layers.iter().map(|l| l.name.as_str()).collect::<Vec<_>>(), ["Bottom", "Folder 1", "Top"]);
    assert_eq!(d.layer(result).unwrap().transform.size, Size { width: 7.0, height: 7.0 }, "trimmed to A..B");
    assert_eq!(full(&d).bytes(), before.bytes());
    // Merge Layers with a multi-selection keeps the topmost selected layer's slot and name.
    let plan = merge::merge_plan(&d, &[result, tid]).unwrap();
    assert_eq!((plan.action, plan.name.as_str()), ("Merge Layers", "Top"));
    let r2 = merge::merge(&mut d, &[result, tid]).unwrap();
    assert_eq!(d.layers.iter().map(|l| l.name.as_str()).collect::<Vec<_>>(), ["Bottom", "Top"]);
    assert_eq!(d.layers[1].id, r2);
}

#[test]
fn clipped_layers_repoint_and_nothing_to_merge_is_none() {
    let mut d = Document::new(4, 4);
    let base = Layer::with_pixels("Base", solid(4, 4, [255, 0, 0]), Point { x: 0.0, y: 0.0 });
    let mid = Layer::with_pixels("Mid", solid(4, 4, [0, 255, 0]), Point { x: 0.0, y: 0.0 });
    let mut clipped = Layer::with_pixels("Clipped", solid(4, 4, [0, 0, 255]), Point { x: 0.0, y: 0.0 });
    clipped.mask_source_id = Some(mid.id);
    let (bid, mid_id, cid) = (base.id, mid.id, clipped.id);
    d.layers = vec![base, mid, clipped];
    assert!(merge::merge_plan(&d, &[bid]).is_none(), "nothing beneath the bottom layer");
    let result = merge::merge(&mut d, &[mid_id]).unwrap();
    assert_eq!(d.layer(cid).unwrap().mask_source_id, Some(result), "clipping follows the merged result");
    let g = hierarchy::add_group(&mut d).unwrap();
    assert!(merge::merge_plan(&d, &[g]).is_none(), "an empty folder has nothing to merge");
}
