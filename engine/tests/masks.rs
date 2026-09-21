use compositor_engine::ops::{hierarchy, masks};
use compositor_engine::*;

fn red_doc() -> (Document, uuid::Uuid) {
    let mut d = Document::new(2, 2);
    let mut l = Layer::with_pixels("Red", Raster::from_premultiplied(2, 2, [255u8, 0, 0, 255].repeat(4)), Point { x: 0.0, y: 0.0 });
    l.transform.sampling = Sampling::Nearest;
    let id = l.id; d.active_layer_id = Some(id); d.layers.push(l);
    (d, id)
}
fn alphas(d: &Document) -> Vec<u8> { composite(d, Rect { x: 0.0, y: 0.0, width: d.width as f64, height: d.height as f64 }, d.width, d.height).bytes().chunks_exact(4).map(|p| p[3]).collect() }

#[test]
fn add_disable_delete_and_folder_masks() {
    let (mut d, id) = red_doc();
    masks::add_mask(&mut d, id, false).unwrap();
    let m = d.layer(id).unwrap().mask.as_ref().unwrap();
    assert_eq!((m.pixels.width, m.pixels.height, m.pixels.bytes()[0]), (1, 1, 0));
    assert!(masks::add_mask(&mut d, id, true).is_err(), "never overwrite an existing mask");
    assert_eq!(alphas(&d), [0; 4]);
    masks::set_mask_enabled(&mut d, id, false).unwrap();
    assert_eq!(alphas(&d), [255; 4]);
    masks::delete_mask(&mut d, id).unwrap();
    assert!(d.layer(id).unwrap().mask.is_none());
    let g = hierarchy::group_layers(&mut d, &[id]).unwrap();
    masks::add_mask(&mut d, g, false).unwrap();
    assert!(d.layer(g).unwrap().mask.is_some());
    assert_eq!(alphas(&d), [0; 4]);
}

#[test]
fn invert_fill_and_link() {
    let (mut d, id) = red_doc();
    d.layer_mut(id).unwrap().set_mask(Some(Mask { pixels: GrayRaster::from_bytes(2, 2, vec![255, 0, 128, 255]), enabled: true, placement: None, linked: None }));
    masks::invert_mask(&mut d, id).unwrap();
    assert_eq!(alphas(&d), [0, 255, 127, 0]);
    masks::fill_mask(&mut d, id, true).unwrap();
    assert_eq!(alphas(&d), [255; 4]);
    assert_eq!(d.layer(id).unwrap().mask.as_ref().unwrap().pixels.width, 2, "fill keeps the grid");
    masks::set_mask_linked(&mut d, id, false).unwrap();
    assert_eq!(d.layer(id).unwrap().mask.as_ref().unwrap().linked, Some(false));
    masks::set_mask_linked(&mut d, id, true).unwrap();
    assert_eq!(d.layer(id).unwrap().mask.as_ref().unwrap().linked, Some(true));
    let rev = d.layer(id).unwrap().mask_revision;
    masks::fill_mask(&mut d, id, false).unwrap();
    assert!(d.layer(id).unwrap().mask_revision > rev);
}

#[test]
fn blur_softens_edges_and_keeps_uniform_masks() {
    let mut data = vec![0u8; 20 * 20];
    for y in 0..20 { for x in 10..20 { data[y * 20 + x] = 255; } }
    let blurred = blur_gray(&GrayRaster::from_bytes(20, 20, data), 2.0);
    let row: Vec<u8> = blurred.bytes()[10 * 20..11 * 20].to_vec();
    assert!(row[9] > 20 && row[9] < 235 && row[10] > 20 && row[10] < 235, "{row:?}");
    assert!(row[0] < 3 && row[19] > 252);
    assert!(row.windows(2).all(|w| w[0] <= w[1]));
    let (mut d, id) = red_doc();
    masks::add_mask(&mut d, id, true).unwrap();
    masks::blur_mask(&mut d, id, 3.0).unwrap();
    assert_eq!(d.layer(id).unwrap().mask.as_ref().unwrap().pixels.width, 1);
    assert!(masks::blur_mask(&mut d, id, 0.0).is_err());
}

#[test]
fn copy_mask_places_it_where_it_sits() {
    let (mut d, a) = red_doc();
    let mut b = Layer::with_pixels("B", Raster::new_transparent(2, 2), Point { x: 1.0, y: 0.0 });
    let bid = b.id;
    b.transform.sampling = Sampling::Nearest;
    d.layers.push(b);
    d.layer_mut(a).unwrap().set_mask(Some(Mask { pixels: GrayRaster::from_bytes(2, 2, vec![255, 0, 128, 255]), enabled: true, placement: None, linked: None }));
    masks::copy_mask(&mut d, a, bid).unwrap();
    let m = d.layer(bid).unwrap().mask.as_ref().unwrap();
    assert_eq!(m.placement, Some(d.layer(a).unwrap().transform), "sits where the source mask sits");
    assert_eq!(m.pixels.bytes(), &[255, 0, 128, 255]);
    let g = hierarchy::add_group(&mut d).unwrap();
    assert!(masks::copy_mask(&mut d, a, g).is_err());
}
