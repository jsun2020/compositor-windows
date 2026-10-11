use compositor_engine::{decode_image, ImportError, Raster};

#[test]
fn independent_webp_decoder_pixels_alpha_and_header_budget() {
    let cases: [(&[u8], &[u8]); 2] = [
        (
            include_bytes!("fixtures/mac149-webp/lossless.webp"),
            include_bytes!("fixtures/mac149-webp/lossless-pillow.rgba"),
        ),
        (
            include_bytes!("fixtures/mac149-webp/lossy.webp"),
            include_bytes!("fixtures/mac149-webp/lossy-pillow.rgba"),
        ),
    ];
    for (file, oracle) in cases {
        let decoded = decode_image(file).unwrap().raster;
        assert_eq!((decoded.width, decoded.height), (24, 18));
        let reference = Raster::from_straight(24, 18, oracle);
        // Independent YUV decoders may round a colour channel by one byte.
        // Alpha and zero-alpha RGB are required to match exactly.
        for (actual, expected) in decoded
            .bytes()
            .chunks_exact(4)
            .zip(reference.bytes().chunks_exact(4))
        {
            assert_eq!(actual[3], expected[3]);
            for channel in 0..3 {
                assert!(actual[channel].abs_diff(expected[channel]) <= 1);
                if actual[3] == 0 {
                    assert_eq!(actual[channel], 0);
                }
            }
        }
    }
    assert!(matches!(
        decode_image(include_bytes!("fixtures/mac149-webp/over-budget.webp")),
        Err(ImportError::OverBudget)
    ));
}
