use compositor_engine::{adjust::lens_effects::*, Raster};
use std::path::Path;

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mac157").join(name)).unwrap()
}

#[test]
fn tone_curves_match_independent_dense_core_image_ramps() {
    for threshold in [50, 75, 80, 90, 95] {
        let bytes = fixture(&format!("lens-threshold-{threshold}.f32"));
        let mut max = 0f32;
        for (i, value) in bytes.chunks_exact(4).enumerate() {
            let expected = f32::from_le_bytes(value.try_into().unwrap());
            max = max.max((highlight_curve(threshold as f64 / 100., i as f32 / 4096.) - expected).abs());
        }
        assert!(max < 0.003, "threshold {threshold}: {max}");
    }
}

#[test]
fn original_lens_reference_comparison() {
    let input = Raster::from_premultiplied(96, 64, fixture("lens-input.rgba"));
    let mut cases = vec![];
    for (name, effect) in [("Anamorphic Flare", LensEffect::AnamorphicFlare), ("Starburst", LensEffect::Starburst),
        ("Mist", LensEffect::Mist), ("Prism", LensEffect::Prism), ("Bokeh", LensEffect::Bokeh)] {
        cases.push((name, LensEffectSettings { effect, ..Default::default() }));
    }
    for (name, bokeh_shape) in [("Bokeh-Hexagon", BokehShape::Hexagon), ("Bokeh-Oval", BokehShape::Oval), ("Bokeh-Cat's Eye", BokehShape::CatsEye)] {
        cases.push((name, LensEffectSettings { effect: LensEffect::Bokeh, bokeh_radius: 6., bokeh_shape, ..Default::default() }));
    }
    cases.push(("Prism-45", LensEffectSettings { effect: LensEffect::Prism, prism_angle: 45., prism_offset: 22., ..Default::default() }));
    cases.push(("Starburst-14", LensEffectSettings { effect: LensEffect::Starburst, star_points: 14, star_angle: -35., ..Default::default() }));
    for (name, settings) in cases {
        let actual = apply(&input, &settings);
        let expected = fixture(&format!("lens-{name}.rgba"));
        let mut errors = vec![];
        for ((a, b), src) in actual.bytes().chunks_exact(4).zip(expected.chunks_exact(4)).zip(input.bytes().chunks_exact(4)) {
            assert_eq!(a[3], src[3], "alpha: {name}");
            for c in 0..3 { errors.push(a[c].abs_diff(b[c])); }
        }
        errors.sort_unstable();
        // Tight bounds for the direct Metal aperture port. Core Image's proprietary
        // rotation/Lanczos/Gaussian paths remain an explicitly measured approximation.
        if settings.effect==LensEffect::Bokeh {assert!(*errors.last().unwrap()<=1,"{name}");}
        if settings.effect==LensEffect::Mist {assert!(*errors.last().unwrap()<=5,"{name}");}
        let mean = errors.iter().map(|&v| v as f64).sum::<f64>() / errors.len() as f64;
        eprintln!("{name}: max={} p99={} mean={mean:.4}", errors.last().unwrap(), errors[errors.len()*99/100]);
        if let Ok(dir) = std::env::var("LENS_DIAGNOSTICS") {
            std::fs::write(Path::new(&dir).join(format!("lens-{name}.rgba")), actual.bytes()).unwrap();
        }
    }
}

#[test]
fn lens_edits_preserve_depth_alpha_transform_and_undo() {
    use compositor_engine::*;
    for depth in [PixelDepth::Eight,PixelDepth::Sixteen,PixelDepth::Hdr] {
        let mut engine=Engine::new();let doc=engine.new_document(96,64,false).unwrap();
        let raster=Raster::from_premultiplied(96,64,fixture("lens-input.rgba")).to_depth(depth);
        let raster=if depth==PixelDepth::Hdr {raster.hdr_exposure(2.,0.,1.)}else{raster};
        let bytes=match depth {PixelDepth::Eight=>encode_png(&raster,72.).unwrap(),PixelDepth::Sixteen=>encode_deep_png(&raster,72.).unwrap(),PixelDepth::Hdr=>encode_exr(&raster).unwrap()};
        engine.import_image(Some(doc),&bytes,"Lens source",None).unwrap();
        let id=engine.document(doc).unwrap().active_layer_id.unwrap();let before=engine.document(doc).unwrap().layer(id).unwrap().clone();
        engine.execute(doc,Command::ApplyFilter{id,params:FilterParams::LensEffects{settings:LensEffectSettings{effect:LensEffect::Mist,..Default::default()}}}).unwrap();
        let after=engine.document(doc).unwrap().layer(id).unwrap();let pixels=after.pixels.as_ref().unwrap();
        assert_eq!(after.transform,before.transform);assert_eq!(pixels.depth(),depth);assert_ne!(after.pixels,before.pixels);
        for y in 0..64{for x in 0..96{assert_eq!(pixels.pixel_f32(x,y)[3],before.pixels.as_ref().unwrap().pixel_f32(x,y)[3]);}}
        engine.undo(doc).unwrap();assert_eq!(engine.document(doc).unwrap().layer(id).unwrap().pixels,before.pixels);
    }
}

#[test]
fn memory_refusal_is_atomic_and_identity_allocates_no_working_grid() {
    use compositor_engine::*;
    let mut engine=Engine::new();let doc=engine.new_document(20000,1,false).unwrap();
    let r=Raster::from_straight(20000,1,&[200,100,50,128].repeat(20000));
    engine.import_image(Some(doc),&encode_png(&r,72.).unwrap(),"Panorama",None).unwrap();
    let id=engine.document(doc).unwrap().active_layer_id.unwrap();let before=engine.document(doc).unwrap().layer(id).unwrap().clone();
    let settings=LensEffectSettings{effect:LensEffect::Starburst,..Default::default()};
    let error=engine.execute(doc,Command::ApplyFilter{id,params:FilterParams::LensEffects{settings:settings.clone()}}).unwrap_err();
    assert!(error.to_string().contains("memory limit"));assert_eq!(engine.document(doc).unwrap().layer(id).unwrap().pixels,before.pixels);
    assert_eq!(apply_checked(&r,&LensEffectSettings{star_amount:0.,..settings}).unwrap(),r);
}
