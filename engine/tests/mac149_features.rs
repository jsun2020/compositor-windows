use compositor_engine::adjust::scanlines::*;
use compositor_engine::export_as::{ExportFormat, ExportOptions};
use compositor_engine::*;
fn source() -> Raster {
    Raster::from_straight(
        12,
        8,
        &(0..96)
            .flat_map(|i| {
                [
                    (i * 11) as u8,
                    (i * 7) as u8,
                    (i * 3) as u8,
                    if i % 3 == 0 { 128 } else { 255 },
                ]
            })
            .collect::<Vec<_>>(),
    )
}

#[test]
fn quarter_rotation_moves_every_placement_and_selection_without_resampling() {
    let mut doc = Document::new(40, 20);
    doc.resolution = 300.;
    let mut layer = Layer::with_pixels("Image", source(), Point { x: 3., y: 5. });
    layer.transform.rotation = 25.;
    layer.transform.flip_x = true;
    layer.mask = Some(Mask {
        pixels: GrayRaster::from_bytes(1, 1, vec![128]),
        enabled: true,
        linked: Some(false),
        placement: Some(LayerTransform::axis_aligned(
            Point { x: -2., y: 4. },
            Size {
                width: 10.,
                height: 4.,
            },
        )),
    });
    layer.extra.text = Some(serde_json::json!({"content":"Editable"}));
    doc.active_layer_id = Some(layer.id);
    doc.layers.push(layer);
    doc.selection = Some(Selection::new(
        vec![vec![[256, 512], [1024, 512], [1024, 1536], [256, 1536]]],
        false,
        2.,
    ));
    let before = doc.clone();
    let mut e = Engine::new();
    let id = e.insert_document(doc);
    e.execute(id, Command::RotateCanvas { clockwise: true })
        .unwrap();
    let after = e.document(id).unwrap();
    assert_eq!(
        (after.width, after.height, after.resolution),
        (20, 40, 300.)
    );
    assert_eq!(after.layers[0].transform.center(), Point { x: 11., y: 9. });
    assert_eq!(after.layers[0].transform.rotation, 115.);
    assert!(after.layers[0]
        .pixels
        .as_ref()
        .unwrap()
        .same_pixels(before.layers[0].pixels.as_ref().unwrap()));
    assert_eq!(after.layers[0].extra, before.layers[0].extra);
    assert_eq!(
        after.layers[0]
            .mask
            .as_ref()
            .unwrap()
            .placement
            .unwrap()
            .center(),
        Point { x: 14., y: 3. }
    );
    assert_eq!(
        after.selection.as_ref().unwrap().contours[0][0],
        [18 * 256, 256]
    );
    assert_eq!(after.selection.as_ref().unwrap().feather, 2.);
    assert_eq!(e.state(id).unwrap().undo_depth, 1);
    e.undo(id).unwrap();
    assert!(e.document(id).unwrap().same_content(&before));
    e.redo(id).unwrap();
    e.execute(id, Command::RotateCanvas { clockwise: false })
        .unwrap();
    assert!(e.document(id).unwrap().same_content(&before));
}
#[test]
fn new_canvas_background_and_dpi_are_initial_state() {
    let mut e = Engine::new();
    for bg in [None, Some([0.; 3]), Some([1.; 3])] {
        let id = e.new_canvas(4, 2, 300., bg).unwrap();
        let d = e.document(id).unwrap();
        assert_eq!(d.resolution, 300.);
        assert_eq!(e.state(id).unwrap().undo_depth, 0);
        let png = e.export_png(id).unwrap();
        let r = decode_image(&png).unwrap().raster;
        assert_eq!(
            r.pixel(0, 0),
            bg.map_or([0; 4], |b| [
                (b[0] * 255.) as u8,
                (b[1] * 255.) as u8,
                (b[2] * 255.) as u8,
                255
            ])
        );
    }
    assert!(e.new_canvas(30000, 30000, 72., Some([1.; 3])).is_err());
    assert!(e.new_canvas(2, 2, f64::NAN, None).is_err());
}
#[test]
fn scanlines_is_deterministic_preserves_alpha_and_commits_as_one_edit() {
    let r = source();
    let mut s = ScanlinesSettings::default();
    s.dots = 60.;
    s.displace = 10.;
    s.wobble = 3.;
    s.split = 2.;
    s.colors = ScanlineColors::Original;
    let a = s.apply(&r).unwrap();
    assert_eq!(a.bytes(), s.apply(&r).unwrap().bytes());
    assert_ne!(a.bytes(), r.bytes());
    for (p, q) in a.bytes().chunks_exact(4).zip(r.bytes().chunks_exact(4)) {
        assert_eq!(p[3], q[3]);
        assert!(p[..3].iter().all(|&v| v <= p[3]));
    }
    s.glow = 50.;
    let lit = s.apply(&r).unwrap();
    for (p, q) in lit.bytes().chunks_exact(4).zip(a.bytes().chunks_exact(4)) {
        assert_eq!(p[3], q[3]);
        assert!(p[..3].iter().zip(&q[..3]).all(|(a, b)| a >= b));
    }
    let mut e = Engine::new();
    let mut d = Document::new(12, 8);
    let l = Layer::with_pixels("Scan", r.clone(), Point { x: 0., y: 0. });
    let lid = l.id;
    d.layers.push(l);
    d.active_layer_id = Some(lid);
    let id = e.insert_document(d);
    e.execute(
        id,
        Command::ApplyFilter {
            id: lid,
            params: FilterParams::Scanlines { settings: s },
        },
    )
    .unwrap();
    assert_eq!(e.state(id).unwrap().undo_depth, 1);
    e.undo(id).unwrap();
    assert_eq!(
        e.document(id)
            .unwrap()
            .layer(lid)
            .unwrap()
            .pixels
            .as_ref()
            .unwrap()
            .bytes(),
        r.bytes()
    );
}
#[test]
fn export_dimensions_dpi_alpha_and_pdf_are_independent_of_the_document() {
    let mut e = Engine::new();
    let mut d = Document::new(12, 8);
    d.resolution = 300.;
    d.layers.push(Layer::with_pixels(
        "Pixels",
        source(),
        Point { x: 0., y: 0. },
    ));
    let before = d.clone();
    let id = e.insert_document(d);
    let mut o = ExportOptions {
        width: 24,
        height: 16,
        format: ExportFormat::PNG,
        quality: 0.8,
        background: None,
    };
    let png = e.export_as(id, &o, 0).unwrap();
    let r = decode_image(&png).unwrap().raster;
    assert_eq!((r.width, r.height), (24, 16));
    assert!(r.bytes().chunks_exact(4).any(|p| p[3] < 255));
    o.format = ExportFormat::JPEG;
    let jpeg = e.export_as(id, &o, 0).unwrap();
    let r = decode_image(&jpeg).unwrap().raster;
    assert_eq!((r.width, r.height), (24, 16));
    assert!(r.bytes().chunks_exact(4).all(|p| p[3] == 255));
    o.format = ExportFormat::PDF;
    let pdf = e.export_as(id, &o, 0).unwrap();
    assert!(pdf.starts_with(b"%PDF-1.4"));
    let text = String::from_utf8_lossy(&pdf);
    assert!(text.contains("/MediaBox [0 0 5.76000000 3.84000000]"));
    assert!(text.contains("/SMask 6 0 R"));
    assert!(text.contains("/Filter /FlateDecode"));
    assert!(e.document(id).unwrap().same_content(&before));
    assert_eq!(e.state(id).unwrap().undo_depth, 0);
    o.width = 30000;
    o.height = 30000;
    assert!(e.export_as(id, &o, 0).is_err());
}

#[test]
fn dpi_only_retains_live_metadata_and_nonuniform_resize_uses_mac149_effect_sizes() {
    let mut doc = Document::new(40, 20);
    let mut layer = Layer::with_pixels("Rotated and flipped", source(), Point { x: 3., y: 5. });
    layer.transform.rotation = 25.;
    layer.transform.flip_x = true;
    layer.extra.text = Some(serde_json::json!({"content":"Editable"}));
    layer.extra.effects=Some(serde_json::from_value(serde_json::json!({"stroke":{"size":3,"red":0,"green":0,"blue":0,"opacity":0.7,"inside":false},"shadow":{"angle":42,"distance":12,"blur":8,"red":0,"green":0,"blue":0,"opacity":0.6},"innerShadow":{"angle":71,"distance":6,"blur":4,"red":0,"green":0,"blue":0,"opacity":0.5},"innerGlow":{"size":7,"red":1,"green":0,"blue":0,"opacity":0.4}})).unwrap());
    let original = layer.clone();
    let layer_id = layer.id;
    doc.layers.push(layer);
    doc.active_layer_id = Some(layer_id);
    let mut e = Engine::new();
    let id = e.insert_document(doc);
    e.execute(
        id,
        Command::ImageSize {
            width: 40,
            height: 20,
            resolution: 300.,
            sampling: Sampling::High,
        },
    )
    .unwrap();
    let kept = e.document(id).unwrap().layer(layer_id).unwrap();
    assert_eq!(kept.extra, original.extra);
    assert!(kept
        .pixels
        .as_ref()
        .unwrap()
        .same_pixels(original.pixels.as_ref().unwrap()));
    e.execute(
        id,
        Command::ImageSize {
            width: 80,
            height: 60,
            resolution: 300.,
            sampling: Sampling::High,
        },
    )
    .unwrap();
    let resized = e.document(id).unwrap().layer(layer_id).unwrap();
    assert!(resized.extra.text.is_none());
    let saved: serde_json::Value =
        serde_json::from_str(&e.save_package(id).unwrap().manifest_json).unwrap();
    let fx = &saved["layers"][0]["effects"];
    let factor = 6f64.sqrt();
    for (effect, key, value) in [
        ("stroke", "size", 3.),
        ("shadow", "distance", 12.),
        ("shadow", "blur", 8.),
        ("innerShadow", "distance", 6.),
        ("innerShadow", "blur", 4.),
        ("innerGlow", "size", 7.),
    ] {
        assert!(
            (fx[effect][key].as_f64().unwrap() - value * factor).abs() < 1e-9,
            "{effect}.{key}"
        );
    }
    assert_eq!(fx["shadow"]["angle"].as_f64(), Some(42.));
    assert_eq!(fx["innerShadow"]["angle"].as_f64(), Some(71.));
    assert_eq!(fx["stroke"]["opacity"], 0.7);
    e.undo(id).unwrap();
    assert_eq!(
        e.document(id).unwrap().layer(layer_id).unwrap().extra,
        original.extra
    );
    assert_eq!(e.document(id).unwrap().resolution, 300.);
    e.redo(id).unwrap();
    assert_eq!(e.document(id).unwrap().width, 80);
    assert!(e
        .document(id)
        .unwrap()
        .layer(layer_id)
        .unwrap()
        .extra
        .text
        .is_none());
}
