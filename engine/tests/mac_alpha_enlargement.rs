use compositor_engine::*;

#[test]
fn mac_145_white_alpha_grid_enlargement_retains_the_saved_sampling_record() {
    let oracle: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/mac-alpha-grid-1.4.5.json")).unwrap();
    for sampling in [Sampling::High, Sampling::Smooth] {
        let mut bytes = Vec::new();
        for y in 0..32 {
            for x in 0..32 {
                let a = ((37 * x + 71 * y + 11 * x * y) % 256) as u8;
                bytes.extend([a, a, a, a]);
            }
        }
        let mut layer = Layer::with_pixels(
            "Mac alpha grid",
            Raster::from_premultiplied(32, 32, bytes),
            Point { x: 16.0, y: 16.0 },
        );
        layer.transform.size = Size {
            width: 34.0,
            height: 34.0,
        };
        layer.transform.sampling = sampling;
        let original = layer.transform;
        let mut doc = Document::new(66, 66);
        doc.layers.push(layer);
        let image = render_full(&doc).unwrap();
        let expected = oracle["alpha"].as_array().unwrap();
        // Independent Mac bytes, including intermediate rounding; the old
        // bilinear filter misses this oracle by up to 37.
        for y in 0..34 {
            for x in 0..34 {
                let a = expected[(y * 34 + x) as usize].as_u64().unwrap() as i16;
                for channel in image.pixel(x + 16, y + 16) {
                    assert!(
                        channel as i16 == a,
                        "{sampling:?} ({x},{y}) {channel} vs Mac {a}"
                    );
                }
            }
        }
        assert_eq!(doc.layers[0].transform, original);
    }
}

#[test]
fn an_exact_half_texel_after_affine_inversion_chooses_the_lower_byte() {
    // The independently returned Mac-created text has this origin and width
    // ratio. Document column 1032 maps to the exact midpoint 857.5; the
    // inverse matrix can otherwise move it one f64 ulp above that midpoint.
    let mut pixels = vec![0; 1144 * 4];
    pixels[858 * 4..859 * 4].copy_from_slice(&[255; 4]);
    let mut layer = Layer::with_pixels(
        "Exact half",
        Raster::from_premultiplied(1144, 1, pixels),
        Point { x: 119.0, y: 0.0 },
    );
    layer.transform.size = Size {
        width: 1218.0,
        height: 1.0,
    };
    layer.transform.sampling = Sampling::High;
    let mut doc = Document::new(1920, 1);
    doc.layers.push(layer);
    // CG's lower-texel half choice, established by the independent alpha grid.
    assert_eq!(render_full(&doc).unwrap().pixel(1032, 0), [127; 4]);
}
