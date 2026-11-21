use compositor_engine::*;

fn at(x: f64, y: f64) -> Point {
    Point { x, y }
}

#[test]
fn psd_round_trip_keeps_clipping_group_mask_unicode_and_visibility() {
    let mut doc = Document::new(6, 4);
    doc.resolution = 300.;
    let mut folder = Layer::blank("分组", doc.size());
    folder.is_group = true;
    let mut base = Layer::with_pixels(
        "底",
        Raster::from_straight(2, 2, &[73, 182, 9, 255].repeat(4)),
        at(1., 1.),
    );
    base.parent_id = Some(folder.id);
    base.opacity = 0.6;
    base.mask = Some(Mask {
        pixels: GrayRaster::from_bytes(2, 2, vec![0, 128, 255, 63]),
        enabled: false,
        linked: Some(false),
        placement: None,
    });
    let mut clip = base.clone();
    clip.id = uuid::Uuid::new_v4();
    clip.mask = None;
    clip.name = "Light".into();
    clip.mask_source_id = Some(base.id);
    clip.blend_mode = BlendMode::Screen;
    clip.visible = false;
    doc.layers = vec![folder, base, clip];
    let merged = compositor::render_full(&doc).unwrap();
    let bytes = psd_writer::encode(&doc, &EffectsCache::default(), &merged).unwrap();
    let read = psd::read(&bytes, MAX_PIXELS, MAX_PIXELS).unwrap().document;
    assert_eq!((read.width, read.height, read.resolution), (6, 4, 300.));
    let base = read.layers.iter().find(|l| l.name == "底").unwrap();
    let folder = read.layers.iter().find(|l| l.name == "分组").unwrap();
    let clip = read.layers.iter().find(|l| l.name == "Light").unwrap();
    assert!(folder.is_group && !clip.visible);
    assert_eq!(clip.blend_mode, BlendMode::Screen);
    assert_eq!(
        (base.parent_id, clip.parent_id, clip.mask_source_id),
        (Some(folder.id), Some(folder.id), Some(base.id))
    );
    assert_eq!(base.transform.origin, at(1., 1.));
    assert_eq!(
        base.mask.as_ref().unwrap().pixels.bytes(),
        [0, 128, 255, 63]
    );
    assert!(!base.mask.as_ref().unwrap().enabled && !base.mask.as_ref().unwrap().is_linked());
    assert_eq!(
        merged.bytes(),
        compositor::render_full(&read).unwrap().bytes()
    );
    // Independently inspect the final raw composite planes, not only the layer section.
    let offset = bytes.len() - 6 * 4 * 4;
    assert_eq!(&bytes[offset - 2..offset], &[0, 0]);
    assert_eq!(bytes[offset + 7], 73);
    assert_eq!(bytes[offset + 3 * 24 + 7], 153);
}

#[test]
fn psd_sixteen_bit_preserves_sub_byte_differences_in_the_merged_image() {
    let raster = Raster::from_premultiplied_f32(
        2,
        1,
        vec![0.4, 0.3, 0., 1., 0.4 + 2. / 65535., 0.3, 0., 1.],
        PixelDepth::Sixteen,
    );
    let mut doc = Document::new(2, 1);
    doc.layers
        .push(Layer::with_pixels("深度", raster, at(0., 0.)));
    let merged = compositor::render_full(&doc).unwrap();
    let bytes = psd_writer::encode(&doc, &EffectsCache::default(), &merged).unwrap();
    assert_eq!(&bytes[22..24], &16u16.to_be_bytes());
    let red = &bytes[bytes.len() - 16..][..4];
    assert_eq!(
        u16::from_be_bytes([red[2], red[3]]) - u16::from_be_bytes([red[0], red[1]]),
        2
    );
}

#[test]
fn measured_noise_reduces_grain_while_keeping_alpha_and_step_edge() {
    let mut bytes = Vec::new();
    let mut seed = 17u32;
    for y in 0..32 {
        for x in 0..48 {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let v = (if x < 24 { 50 } else { 198 }) + (seed >> 28) as u8;
            bytes.extend([v, v, v, if y == 0 { 0 } else { 255 }]);
        }
    }
    let src = Raster::from_straight(48, 32, &bytes);
    let p:FilterParams=serde_json::from_value(serde_json::json!({"filter":"ReduceNoise","settings":{"strength":100,"details":50,"color":0}})).unwrap();
    let dst = adjust::filters::apply_filter(&src, &p).unwrap();
    let variation = |r: &Raster| -> i32 {
        (4..27)
            .flat_map(|y| {
                (4..20).map(move |x| {
                    (i32::from(r.pixel(x, y)[0]) - i32::from(r.pixel(x + 1, y)[0])).abs()
                })
            })
            .sum()
    };
    assert!(variation(&dst) * 2 < variation(&src));
    assert!(dst.pixel(23, 16)[0] < 70 && dst.pixel(24, 16)[0] > 190);
    assert!(dst
        .bytes()
        .chunks_exact(4)
        .zip(src.bytes().chunks_exact(4))
        .all(|(a, b)| a[3] == b[3]));
    let mut doc = Document::new(48, 32);
    doc.layers
        .push(Layer::with_pixels("Photo", src.clone(), at(0., 0.)));
    let mut adjustment = LayerAdjustment::new(AdjustmentKind::ReduceNoise);
    adjustment.filter_layer_settings = Some(
        serde_json::from_value(
            serde_json::json!({"reduceNoise":{"strength":100,"details":50,"color":0}}),
        )
        .unwrap(),
    );
    let mut layer = Layer::blank("Noise", doc.size());
    layer.extra.adjustment = Some(adjustment);
    doc.layers.push(layer);
    assert_eq!(doc.manifest().version, 21);
    assert_eq!(compositor::render_full(&doc).unwrap().bytes(), dst.bytes());
    let mut old = doc.manifest();
    old.version = 20;
    assert!(old.validate().is_err());
}

#[test]
fn editable_inside_stroke_survives_project_round_trip_and_transparent_corners() {
    let mut doc = Document::new(24, 16);
    let spec = ShapeSpec::Ellipse {
        rect: Rect {
            x: 0.,
            y: 0.,
            width: 24.,
            height: 16.,
        },
    };
    let stroke = ops::shape::ShapeStroke {
        width: 3.,
        color: [1., 0., 0.],
    };
    let id = ops::shape::add_styled(&mut doc, &spec, [0., 0., 1.], Some(&stroke)).unwrap();
    let layer = doc.layer(id).unwrap();
    let image = layer.pixels.as_ref().unwrap();
    assert_eq!(image.pixel(0, 0), [0; 4]);
    assert_eq!(image.pixel(1, 8), [255, 0, 0, 255]);
    assert_eq!(image.pixel(12, 8), [0, 0, 255, 255]);
    for px in image
        .bytes()
        .chunks_exact(4)
        .filter(|p| p[3] > 0 && p[3] < 255)
    {
        assert_eq!(px[2], 0);
    }
    let mut manifest = doc.manifest();
    assert_eq!(manifest.version, 22);
    assert!(manifest.validate().is_ok());
    manifest.version = 21;
    assert!(manifest.validate().is_err());
    let shape = layer.extra.shape.as_ref().unwrap();
    assert_eq!(
        ops::shape::redraw_image(shape, doc.size(), 1.).unwrap(),
        *image
    );
    ops::shape::edit_appearance(
        &mut doc,
        id,
        &ops::shape::ShapeAppearance {
            fill: Some([0., 1., 0.]),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        doc.layer(id).unwrap().pixels.as_ref().unwrap().pixel(12, 8),
        [0, 255, 0, 255]
    );
}

#[test]
fn wider_line_grows_around_its_endpoints_and_keeps_the_mask_in_place() {
    use ops::shape::{ShapeAppearance, ShapeLineCap, ShapeLineStyle};
    let mut doc = Document::new(60, 40);
    let spec = ShapeSpec::Line {
        start: at(10., 14.),
        end: at(40., 14.),
        width: 2.,
        style: ShapeLineStyle::Solid,
        cap: ShapeLineCap::Round,
    };
    let id = ops::shape::add_styled(&mut doc, &spec, [1., 0., 0.], None).unwrap();
    let before = doc.layer(id).unwrap().transform;
    doc.layer_mut(id).unwrap().mask = Some(Mask {
        pixels: GrayRaster::from_bytes(1, 1, vec![255]),
        enabled: true,
        linked: Some(false),
        placement: None,
    });
    let edit = ShapeAppearance {
        line_width: Some(8.),
        line_cap: Some(ShapeLineCap::Square),
        ..Default::default()
    };
    ops::shape::edit_appearance(&mut doc, id, &edit).unwrap();
    let updated = doc.layer(id).unwrap();
    assert_eq!(updated.transform.origin, at(6., 10.));
    assert_eq!(
        updated.transform.size,
        Size {
            width: 38.,
            height: 8.
        }
    );
    assert_eq!(updated.mask.as_ref().unwrap().placement, Some(before));
    assert_eq!(
        updated.pixels.as_ref().unwrap().pixel(18, 0),
        [255, 0, 0, 255]
    );
    let record = updated.extra.shape.as_ref().unwrap();
    let start: Point = serde_json::from_value(record["start"].clone()).unwrap();
    assert_eq!(updated.transform.point(start), at(10., 14.));
}

#[test]
fn copied_clipping_pair_stays_together_in_the_destination_folder() {
    let mut doc = Document::new(8, 8);
    let mut group = Layer::blank("Destination", doc.size());
    group.is_group = true;
    let base = Layer::with_pixels(
        "Base",
        Raster::from_straight(1, 1, &[220, 0, 0, 255]),
        at(1., 1.),
    );
    let mut clip = Layer::with_pixels(
        "Ink",
        Raster::from_straight(1, 1, &[0, 200, 0, 255]),
        at(1., 1.),
    );
    clip.mask_source_id = Some(base.id);
    let (folder, b, c) = (group.id, base.id, clip.id);
    doc.layers = vec![group, base, clip];
    ops::hierarchy::duplicate_selected_to(&mut doc, &[b, c], Some(folder), None, false).unwrap();
    let copies: Vec<_> = doc
        .layers
        .iter()
        .filter(|l| l.parent_id == Some(folder))
        .collect();
    assert_eq!(copies.len(), 2);
    assert_eq!(copies[0].name, "Base copy");
    assert_eq!(copies[1].mask_source_id, Some(copies[0].id));
    assert_eq!(doc.layer(c).unwrap().mask_source_id, Some(b));
    assert!(doc.manifest().validate().is_ok());
}
