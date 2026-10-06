use compositor_engine::warp::{WarpMode, WarpSettings, WarpStroke};
use compositor_engine::*;
fn p(x: f64, y: f64) -> Point {
    Point { x, y }
}
fn spec() -> WarpSpec {
    WarpSpec {
        mode: WarpMode::Smudge,
        diameter: 2.0,
        hardness: 0.5,
        strength: 0.5,
        points: vec![p(3.0, 8.0), p(6.0, 8.0)],
    }
}
fn setup(w: u32, h: u32) -> (Engine, uuid::Uuid, uuid::Uuid) {
    let mut d = Document::new(w, h);
    let mut b = [0, 0, 0, 255].repeat(w as usize * h as usize);
    for y in 0..h {
        b[((y * w + 3) * 4) as usize..((y * w + 3) * 4 + 4) as usize].copy_from_slice(&[255; 4]);
    }
    let l = Layer::with_pixels("Stripe", Raster::from_premultiplied(w, h, b), p(0.0, 0.0));
    let layer = l.id;
    d.layers.push(l);
    d.active_layer_id = Some(layer);
    let mut e = Engine::new();
    let doc = e.insert_document(d);
    (e, doc, layer)
}
fn tiles(source: &Raster) -> Vec<(WarpTileRect, Vec<u8>)> {
    if source.width as u64 * source.height as u64 > 4_194_304 {
        return vec![(
            WarpTileRect {
                x: 4,
                y: 8,
                width: 1,
                height: 1,
            },
            vec![128, 128, 128, 255],
        )];
    }
    let s = spec();
    let mut warp = WarpStroke::new(
        source.width as usize,
        source.height as usize,
        source.bytes(),
        s.mode,
        WarpSettings {
            diameter: s.diameter,
            hardness: s.hardness as f32,
            strength: s.strength as f32,
        },
    )
    .unwrap();
    for p in s.points {
        warp.append_dabs([p.x, p.y]).unwrap();
    }
    let r = WarpTileRect {
        x: 0,
        y: 4,
        width: 10,
        height: 9,
    };
    let mut b = vec![];
    for y in r.y..r.y + r.height {
        let at = ((y * source.width + r.x) * 4) as usize;
        b.extend_from_slice(&warp.pixels()[at..at + r.width as usize * 4]);
    }
    vec![(r, b)]
}
#[test]
fn live_preview_matches_final_pixels_and_does_not_mutate_stored_snapshot() {
    let (mut e, doc, layer) = setup(20, 16);
    e.execute(
        doc,
        Command::SelectShape {
            kind: SelectionShape::Rectangle,
            points: vec![p(4.0, 0.0), p(5.0, 0.0), p(5.0, 16.0), p(4.0, 16.0)],
            mode: SelectionMode::Replace,
            antialiased: false,
        },
    )
    .unwrap();
    let before = e.document(doc).unwrap().clone();
    let (input, pixels, mask, points) = e.job_input(doc, layer).unwrap();
    let job = WarpJob::new(input.clone(), pixels, mask, points.as_deref()).unwrap();
    let (output, pixels, mask, display) =
        job.preview(&spec(), tiles(&job.source())).unwrap().unwrap();
    let (final_output, final_pixels, _, _) = job
        .finish(
            &spec(),
            tiles(&ops::warp::source_plane(&before, layer).unwrap()),
            0.0,
        )
        .unwrap();
    assert_eq!(pixels, final_pixels);
    assert_eq!(output.transform, final_output.transform);
    e.keep_job_preview(
        doc,
        layer,
        input.stamp,
        output,
        pixels.unwrap(),
        mask,
        display,
    )
    .unwrap();
    assert_eq!(
        e.document(doc).unwrap().layer(layer).unwrap().pixels,
        before.layer(layer).unwrap().pixels
    );
    assert_eq!(e.job_input(doc, layer).unwrap().0.stamp, input.stamp);
    e.set_preview(doc, None).unwrap();
    assert_eq!(
        e.document(doc).unwrap().layer(layer).unwrap().pixels,
        before.layer(layer).unwrap().pixels
    );
}
#[test]
fn large_preview_is_bounded_and_final_keeps_native_grid() {
    let (e, doc, layer) = setup(2500, 1700);
    let (input, pixels, mask, points) = e.job_input(doc, layer).unwrap();
    let job = WarpJob::new(input, pixels, mask, points.as_deref()).unwrap();
    let (output, pixels, _, _) = job.preview(&spec(), tiles(&job.source())).unwrap().unwrap();
    let (w, h) = output.pixels.unwrap();
    assert!(w <= 1024 && h <= 1024);
    assert!(pixels.unwrap().bytes().len() <= 4 * 1024 * 1024);
    let (source, _, _, _) = job
        .finish(
            &spec(),
            tiles(&ops::warp::source_plane(e.document(doc).unwrap(), layer).unwrap()),
            0.0,
        )
        .unwrap();
    assert_eq!(source.pixels, Some((2500, 1700)));
}
#[test]
fn session_refuses_bad_tiles_and_reference_fallback_stays_bounded() {
    let (e, doc, layer) = setup(20, 16);
    let (input, pixels, mask, points) = e.job_input(doc, layer).unwrap();
    let job = WarpJob::new(input, pixels, mask, points.as_deref()).unwrap();
    assert!(job
        .preview(
            &spec(),
            vec![(
                WarpTileRect {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1
                },
                vec![255, 0, 0, 0]
            )]
        )
        .is_err());
    assert_eq!(
        job.reference(&spec(), 0.0).unwrap().1.unwrap().pixel(4, 8),
        [128, 128, 128, 255]
    );
    let (e, doc, layer) = setup(2500, 1700);
    let (input, pixels, mask, points) = e.job_input(doc, layer).unwrap();
    let job = WarpJob::new(input, pixels, mask, points.as_deref()).unwrap();
    assert!(job
        .reference(&spec(), 0.0)
        .unwrap_err()
        .to_string()
        .contains("4 Mi pixels"));
}

#[test]
fn rotated_growing_preview_follows_a_nonuniform_mask_like_completion() {
    let (e, doc, layer) = setup(20, 16);
    let mut d = e.document(doc).unwrap().clone();
    d.width = 60;
    d.height = 60;
    let l = d.layer_mut(layer).unwrap();
    l.transform.origin = p(10.0, 10.0);
    l.transform.rotation = 90.0;
    l.mask = Some(Mask {
        pixels: GrayRaster::from_bytes(
            20,
            16,
            (0..320)
                .map(|i| if i % 20 < 10 { 64 } else { 192 })
                .collect(),
        ),
        enabled: true,
        placement: None,
        linked: Some(true),
    });
    let mut e = Engine::new();
    let doc = e.insert_document(d);
    let (input, pixels, mask, points) = e.job_input(doc, layer).unwrap();
    let job = WarpJob::new(input, pixels, mask, points.as_deref()).unwrap();
    let s = WarpSpec {
        points: vec![p(20.0, 11.0), p(20.0, 5.0)],
        ..spec()
    };
    let source = job.source();
    let mut warp = WarpStroke::new(
        60,
        60,
        source.bytes(),
        s.mode,
        WarpSettings {
            diameter: s.diameter,
            hardness: s.hardness as f32,
            strength: s.strength as f32,
        },
    )
    .unwrap();
    for p in &s.points {
        warp.append_dabs([p.x, p.y]).unwrap();
    }
    let tiles = vec![(
        WarpTileRect {
            x: 0,
            y: 0,
            width: 60,
            height: 60,
        },
        warp.pixels().to_vec(),
    )];
    let (preview, pixels, mask, _) = job.preview(&s, tiles.clone()).unwrap().unwrap();
    let (final_output, final_pixels, final_mask, _) = job.finish(&s, tiles, 0.0).unwrap();
    assert_eq!(preview.transform, final_output.transform);
    assert_eq!(pixels, final_pixels);
    assert_eq!(mask, final_mask);
}

#[test]
fn identity_source_shares_the_asset_and_matches_an_unstyled_composite() {
    let (original, doc, layer) = setup(20, 16);
    let mut d = original.document(doc).unwrap().clone();
    let bytes = (0..320)
        .flat_map(|i| {
            let alpha = (i % 256) as u8;
            [alpha / 2, alpha / 3, alpha, alpha]
        })
        .collect();
    d.layer_mut(layer)
        .unwrap()
        .set_pixels(Some(Raster::from_premultiplied(20, 16, bytes)));
    let mut e = Engine::new();
    let doc = e.insert_document(d);
    e.execute(
        doc,
        Command::SetLayerOpacity {
            id: layer,
            opacity: 0.3,
        },
    )
    .unwrap();
    e.execute(
        doc,
        Command::AddMask {
            id: layer,
            revealing: false,
        },
    )
    .unwrap();
    let d = e.document(doc).unwrap();
    let source = ops::warp::source_plane(d, layer).unwrap();
    let original = d.layer(layer).unwrap().pixels.as_ref().unwrap();
    assert!(source.same_pixels(original));
    let mut raw = d.clone();
    let l = raw.layer_mut(layer).unwrap();
    l.opacity = 1.0;
    l.mask = None;
    l.extra.effects = None;
    l.blend_mode = BlendMode::Normal;
    assert_eq!(
        source,
        compositor::composite(
            &raw,
            Rect {
                x: 0.0,
                y: 0.0,
                width: 20.0,
                height: 16.0
            },
            20,
            16
        )
    );
}
