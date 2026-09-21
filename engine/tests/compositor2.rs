mod fixtures;
use compositor_engine::*;

fn alphas(r: &Raster) -> Vec<u8> { r.bytes().chunks_exact(4).map(|p| p[3]).collect() }
fn reds(r: &Raster) -> Vec<u8> { r.bytes().chunks_exact(4).map(|p| p[0]).collect() }
fn full(d: &Document) -> Raster { composite(d, Rect { x: 0.0, y: 0.0, width: d.width as f64, height: d.height as f64 }, d.width, d.height) }
fn gray_mask(w: u32, h: u32, values: Vec<u8>) -> Mask { Mask { pixels: GrayRaster::from_bytes(w, h, values), enabled: true, placement: None, linked: None } }
fn premul_red(alphas: &[u8], w: u32, h: u32) -> Raster {
    let mut data = Vec::new();
    for &a in alphas { data.extend_from_slice(&[a, 0, 0, a]); }
    Raster::from_premultiplied(w, h, data)
}
fn nearest(layer: &mut Layer) { layer.transform.sampling = Sampling::Nearest; }

/// A 2x2 opaque red layer on a 2x2 canvas, nearest sampling.
fn red_doc() -> Document {
    let mut d = Document::new(2, 2);
    let mut l = Layer::with_pixels("Red", premul_red(&[255; 4], 2, 2), Point { x: 0.0, y: 0.0 });
    nearest(&mut l);
    d.active_layer_id = Some(l.id);
    d.layers.push(l);
    d
}

#[test]
fn masks_coverage_opacity_disabled_and_solid() {
    let mut d = red_doc();
    assert_eq!(alphas(&full(&d)), [255; 4]);
    d.layers[0].set_mask(Some(gray_mask(1, 1, vec![0])));
    assert_eq!(alphas(&full(&d)), [0; 4], "hide-all");
    d.layers[0].mask_mut().unwrap().enabled = false;
    assert_eq!(alphas(&full(&d)), [255; 4], "disabled mask is ignored");
    d.layers[0].set_mask(Some(gray_mask(2, 2, vec![255, 0, 128, 255])));
    assert_eq!(alphas(&full(&d)), [255, 0, 128, 255]);
    d.layers[0].opacity = 0.5;
    let a = alphas(&full(&d));
    for (v, e) in a.iter().zip([128u8, 0, 64, 128]) { assert!((*v as i32 - e as i32).abs() <= 1, "{a:?}"); }
}

#[test]
fn masks_follow_flips_and_rotation() {
    let mut d = red_doc();
    d.layers[0].set_mask(Some(gray_mask(2, 2, vec![255, 0, 128, 255])));
    d.layers[0].transform.flip_x = true;
    assert_eq!(alphas(&full(&d)), [0, 255, 255, 128]);
    d.layers[0].transform.flip_x = false;
    d.layers[0].transform.rotation = 90.0;
    let mut a = alphas(&full(&d)); a.sort();
    assert_eq!(a, [0, 128, 255, 255]);
}

#[test]
fn folder_masks_clip_descendants_and_multiply() {
    let mut d = red_doc();
    let red = d.layers[0].id;
    let mut folder = Layer::blank("Folder", d.size()); folder.is_group = true; nearest(&mut folder);
    let fid = folder.id;
    d.layers[0].parent_id = Some(fid);
    d.layers.insert(0, folder);
    d.layer_mut(fid).unwrap().set_mask(Some(gray_mask(1, 1, vec![0])));
    assert_eq!(alphas(&full(&d)), [0; 4]);
    d.layer_mut(fid).unwrap().mask_mut().unwrap().enabled = false;
    assert_eq!(alphas(&full(&d)), [255; 4]);
    d.layer_mut(fid).unwrap().set_mask(Some(gray_mask(2, 2, vec![255, 0, 128, 255])));
    assert_eq!(alphas(&full(&d)), [255, 0, 128, 255]);
    d.layer_mut(red).unwrap().set_mask(Some(gray_mask(2, 2, vec![255, 0, 128, 255])));
    let a = alphas(&full(&d));
    for (v, e) in a.iter().zip([255u8, 0, 64, 255]) { assert!((*v as i32 - e as i32).abs() <= 1, "{a:?}"); }
    // An enclosing folder's mask applies too.
    d.layer_mut(red).unwrap().set_mask(None);
    let mut outer = Layer::blank("Outer", d.size()); outer.is_group = true; outer.set_mask(Some(gray_mask(1, 1, vec![0])));
    let oid = outer.id;
    d.layer_mut(fid).unwrap().parent_id = Some(oid);
    d.layers.insert(0, outer);
    assert_eq!(alphas(&full(&d)), [0; 4]);
    d.layer_mut(oid).unwrap().set_mask(None);
    assert_eq!(alphas(&full(&d)), [255, 0, 128, 255]);
}

#[test]
fn clipping_uses_hidden_source_alpha_opacity_masks_movement_and_chains() {
    let mut d = Document::new(2, 2);
    let mut target = Layer::with_pixels("T", premul_red(&[255; 4], 2, 2), Point { x: 0.0, y: 0.0 }); nearest(&mut target);
    let mut source = Layer::with_pixels("S", premul_red(&[255, 0, 128, 255], 2, 2), Point { x: 0.0, y: 0.0 }); nearest(&mut source);
    source.visible = false;
    target.mask_source_id = Some(source.id);
    let sid = source.id;
    d.layers = vec![target, source];
    assert_eq!(alphas(&full(&d)), [255, 0, 128, 255]);
    d.layer_mut(sid).unwrap().opacity = 0.5;
    let a = alphas(&full(&d));
    assert!((a[0] as i32 - 128).abs() <= 1 && a[1] == 0 && (a[2] as i32 - 64).abs() <= 1, "{a:?}");
    d.layers[0].set_mask(Some(gray_mask(1, 1, vec![128])));
    assert!(alphas(&full(&d))[0] < a[0]);
    d.layers[0].set_mask(None);
    d.layer_mut(sid).unwrap().opacity = 1.0;
    d.layer_mut(sid).unwrap().transform.origin.x += 1.0;
    assert_eq!(alphas(&full(&d)), [0, 255, 0, 128], "a moved source changes coverage");
    d.layer_mut(sid).unwrap().transform.origin.x -= 1.0;
    let mut s2 = Layer::with_pixels("S2", premul_red(&[0, 255, 255, 255], 2, 2), Point { x: 0.0, y: 0.0 }); nearest(&mut s2);
    s2.visible = false;
    d.layer_mut(sid).unwrap().mask_source_id = Some(s2.id);
    d.layers.push(s2);
    assert_eq!(alphas(&full(&d)), [0, 0, 128, 255], "chains multiply");
}

#[test]
fn clipping_stack_keeps_the_base_alpha_and_colours_without_fringe() {
    let mut d = Document::new(2, 2);
    let mut base = Layer::with_pixels("Base", premul_red(&[255, 128, 32, 0], 2, 2), Point { x: 0.0, y: 0.0 }); nearest(&mut base);
    // The child is opaque red everywhere (its own alpha 255).
    let mut child = Layer::with_pixels("Child", premul_red(&[255; 4], 2, 2), Point { x: 0.0, y: 0.0 }); nearest(&mut child);
    child.mask_source_id = Some(base.id);
    d.layers = vec![base, child];
    let out = full(&d);
    assert_eq!(alphas(&out), [255, 128, 32, 0]);
    for px in out.bytes().chunks_exact(4) { assert_eq!(px[0], px[3]); assert_eq!((px[1], px[2]), (0, 0)); }
    d.layers[1].opacity = 0.5;
    assert_eq!(alphas(&full(&d)), [255, 128, 32, 0]);
    d.layers[1].opacity = 1.0;
    let mut white = Layer::with_pixels("White", Raster::from_premultiplied(2, 2, vec![255; 16]), Point { x: 0.0, y: 0.0 }); nearest(&mut white);
    d.layers.insert(0, white);
    let out = full(&d);
    assert_eq!(alphas(&out), [255; 4]);
    assert_eq!(reds(&out), [255; 4]);
}

#[test]
fn blend_modes_match_known_values_through_the_document() {
    let grey = |v: u8| Raster::from_premultiplied(4, 4, [v, v, v, 255].repeat(16));
    let mut d = Document::new(4, 4);
    d.layers.push(Layer::with_pixels("Back", grey(102), Point { x: 0.0, y: 0.0 }));
    d.layers.push(Layer::with_pixels("Front", grey(204), Point { x: 0.0, y: 0.0 }));
    for (mode, expected) in [(BlendMode::Normal, 0.8), (BlendMode::Multiply, 0.32), (BlendMode::Screen, 0.88), (BlendMode::Overlay, 0.64),
        (BlendMode::Darken, 0.4), (BlendMode::Lighten, 0.8), (BlendMode::Difference, 0.4), (BlendMode::ColorDodge, 1.0), (BlendMode::ColorBurn, 0.25)] {
        d.layers[1].blend_mode = mode;
        let px = full(&d).pixel(1, 1);
        assert!(((px[0] as f64 / 255.0) - expected).abs() < 0.02, "{mode:?}: {}", px[0]);
        assert_eq!(px[3], 255);
    }
    d.layers[1].blend_mode = BlendMode::Normal;
    d.layers[1].opacity = 0.5;
    assert!(((full(&d).pixel(1, 1)[0] as f64 / 255.0) - 0.6).abs() < 0.02);
}

#[test]
fn distortion_preview_warps_into_the_shape() {
    let mut d = Document::new(100, 60);
    let mut l = Layer::with_pixels("Red", premul_red(&[255; 400], 20, 20), Point { x: 10.0, y: 10.0 }); nearest(&mut l);
    let id = l.id;
    d.layers.push(l);
    let shape = [Point { x: 10.0, y: 10.0 }, Point { x: 60.0, y: 10.0 }, Point { x: 30.0, y: 30.0 }, Point { x: 10.0, y: 30.0 }];
    let edit = PreviewEdit::Layer { id, draft: d.layers[0].transform, corners: Some(shape) };
    let out = composite_edit(&d, Some(&edit), Rect { x: 0.0, y: 0.0, width: 100.0, height: 60.0 }, 100, 60);
    assert_eq!(out.pixel(50, 12)[3], 255);
    assert_eq!(out.pixel(15, 25)[3], 255);
    assert_eq!(out.pixel(50, 28)[3], 0);
    assert_eq!(out.pixel(80, 12)[3], 0);
}

#[test]
fn alpha_bounds_and_crop() {
    let mut data = vec![0u8; 40 * 20 * 4];
    for y in 5..15 { for x in 15..25 { let i = (y * 40 + x) * 4; data[i] = 255; data[i + 3] = 255; } }
    let r = Raster::from_premultiplied(40, 20, data);
    assert_eq!(alpha_bounds(&r), Some((15, 5, 25, 15)));
    let c = r.cropped(15, 5, 10, 10);
    assert_eq!((c.width, c.height), (10, 10));
    assert_eq!(c.pixel(0, 0)[3], 255);
    assert_eq!(alpha_bounds(&Raster::new_transparent(3, 3)), None);
}
