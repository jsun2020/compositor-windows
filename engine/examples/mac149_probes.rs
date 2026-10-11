//! Deterministic Windows outputs and .comp inputs for an independent Mac 1.4.9 oracle.
use compositor_engine::{
    adjust::{camera_raw::CameraRawSettings, scanlines::ScanlinesSettings},
    Command, Document, Engine, Layer, LayerEffects, Point, Raster,
};
use serde_json::{json, Value};
use std::{fs, path::PathBuf};
extern "C" {
    fn dither_quantize16(
        wide: *const u16,
        out: *mut u8,
        width: usize,
        height: usize,
        stride: usize,
    );
}

fn merge(target: &mut Value, patch: &Value) {
    if let (Some(to), Some(from)) = (target.as_object_mut(), patch.as_object()) {
        for (k, v) in from {
            if let Some(existing) = to.get_mut(k) {
                merge(existing, v);
            } else {
                to.insert(k.clone(), v.clone());
            }
        }
    } else {
        *target = patch.clone();
    }
}
fn main() {
    let root = PathBuf::from(std::env::args().nth(1).expect("fresh output directory"));
    assert!(!root.exists(), "preserve old outputs");
    fs::create_dir_all(root.join("Windows")).unwrap();
    let mut straight = Vec::new();
    for y in 0..96u32 {
        for x in 0..128u32 {
            let a = if y < 48 {
                255
            } else {
                ((x * 17 + y * 13) % 256) as u8
            };
            straight.extend([
                ((x * 31 + y * 7) % 256) as u8,
                ((x * 3 + y * 41) % 256) as u8,
                ((x * 19 + y * 23) % 256) as u8,
                a,
            ]);
        }
    }
    let raster = Raster::from_straight(128, 96, &straight);
    fs::write(root.join("input.rgba"), raster.bytes()).unwrap();
    fs::write(
        root.join("input.png"),
        compositor_engine::encode_png(&raster, 72.).unwrap(),
    )
    .unwrap();
    let specs = vec![
        ("identity", "CameraRaw", json!({})),
        ("exposure-positive", "CameraRaw", json!({"exposure":1.25})),
        ("exposure-negative", "CameraRaw", json!({"exposure":-2.25})),
        (
            "light-adaptive",
            "CameraRaw",
            json!({"contrast":27,"highlights":-35,"shadows":41,"whites":17,"blacks":-24}),
        ),
        (
            "white-balance",
            "CameraRaw",
            json!({"temperature":31,"tint":-14,"vibrance":23,"saturation":-11}),
        ),
        (
            "calibration",
            "CameraRaw",
            json!({"calibration":{"shadowTint":13,"redHue":33,"redSaturation":-24,"greenHue":12,"greenSaturation":18,"blueHue":-27,"blueSaturation":34}}),
        ),
        (
            "mixer",
            "CameraRaw",
            json!({"mixer":{"hue":[31,-20,7,0,-11,24,19,-9],"saturation":[8,-17,24,-5,13,-21,19,11],"luminance":[-13,22,4,-15,16,9,-7,18]}}),
        ),
        (
            "grading",
            "CameraRaw",
            json!({"grading":{"global":{"hue":170,"saturation":25,"luminance":-17},"highlights":{"hue":30,"saturation":26,"luminance":15},"shadows":{"hue":217,"saturation":38,"luminance":7},"balance":31,"blending":67}}),
        ),
        (
            "parametric-curve",
            "CameraRaw",
            json!({"curve":{"shadows":23,"darks":-37,"lights":41,"highlights":-19}}),
        ),
        (
            "point-curve",
            "CameraRaw",
            json!({"curve":{"rgb":[{"x":0,"y":0},{"x":0.25,"y":0.12},{"x":0.75,"y":0.83},{"x":1,"y":1}],"red":[{"x":0,"y":0.02},{"x":0.5,"y":0.47},{"x":1,"y":0.96}]}}),
        ),
        (
            "custom-curve",
            "CameraRaw",
            json!({"curve":{"shadows":23,"lights":41,"shadowSplit":20,"darkSplit":47,"lightSplit":81,"refineSaturation":37}}),
        ),
        (
            "local-contrast",
            "CameraRaw",
            json!({"texture":43,"clarity":-21,"dehaze":25}),
        ),
        ("negative-dehaze", "CameraRaw", json!({"dehaze":-35})),
        (
            "diffusion-vignette",
            "CameraRaw",
            json!({"glow":23,"glowRange":-17,"glowSpread":31,"glowWarmth":14,"vignetteAmount":-36,"vignetteHighlights":28}),
        ),
        (
            "bloom",
            "CameraRaw",
            json!({"glow":31,"glowStyle":"Bloom","glowRange":17,"glowSpread":-23,"glowWarmth":-11}),
        ),
        (
            "halation",
            "CameraRaw",
            json!({"glow":29,"glowStyle":"Halation","glowRange":23,"glowSpread":11}),
        ),
        (
            "detail",
            "CameraRaw",
            json!({"detail":{"sharpenAmount":63,"sharpenRadius":12,"sharpenDetail":37,"sharpenMasking":18}}),
        ),
        (
            "noise-reduction",
            "CameraRaw",
            json!({"detail":{"noiseLuminance":18,"noiseLuminanceDetail":56,"noiseLuminanceContrast":22,"noiseColor":30,"noiseColorDetail":34,"noiseColorSmoothness":28}}),
        ),
        (
            "optics-vignette",
            "CameraRaw",
            json!({"optics":{"vignetteAmount":17,"vignetteMidpoint":40,"removeChromaticAberration":true}}),
        ),
        (
            "grain",
            "CameraRaw",
            json!({"grainAmount":27,"grainSize":31,"grainRoughness":42,"seed":12345}),
        ),
        (
            "combined",
            "CameraRaw",
            json!({"exposure":0.75,"contrast":17,"temperature":-21,"tint":13,"texture":23,"dehaze":17,"detail":{"noiseColor":11},"grading":{"global":{"hue":73,"saturation":17,"luminance":4}}}),
        ),
        ("scanlines-default", "Scanlines", json!({})),
        (
            "scanlines-shapes",
            "Scanlines",
            json!({"lineSpacing":7,"thickness":61,"dots":51,"wobble":4,"displace":11,"smoothness":47,"threshold":13,"split":3,"density":-12,"contrast":18,"blackLevel":9}),
        ),
        (
            "scanlines-original",
            "Scanlines",
            json!({"lineSpacing":7,"split":3,"colors":"Original"}),
        ),
        (
            "scanlines-two-colors",
            "Scanlines",
            json!({"lineSpacing":5,"dots":48,"colors":"Two Colors","dark":[0.1,0.17,0.2],"light":[1,0.8,0.3]}),
        ),
        (
            "scanlines-glow",
            "Scanlines",
            json!({"lineSpacing":7,"glow":32,"colors":"Original"}),
        ),
    ];
    let mut rows = Vec::new();
    for (name, kind, patch) in specs {
        let (result, settings) = if kind == "CameraRaw" {
            let mut value = serde_json::to_value(CameraRawSettings::default()).unwrap();
            merge(&mut value, &patch);
            let s: CameraRawSettings = serde_json::from_value(value.clone()).unwrap();
            (s.apply(&raster).unwrap(), value)
        } else {
            let mut value = serde_json::to_value(ScanlinesSettings::default()).unwrap();
            merge(&mut value, &patch);
            let s: ScanlinesSettings = serde_json::from_value(value.clone()).unwrap();
            (s.apply(&raster).unwrap(), value)
        };
        fs::write(
            root.join("Windows").join(format!("{name}.rgba")),
            result.bytes(),
        )
        .unwrap();
        fs::write(
            root.join("Windows").join(format!("{name}.png")),
            compositor_engine::encode_png(&result, 72.).unwrap(),
        )
        .unwrap();
        rows.push(json!({"name":name,"kind":kind,"patch":patch,"settings":settings}));
    }
    fs::write(root.join("cases.json"),serde_json::to_vec_pretty(&json!({"upstream":"0ab3ff58a3dcf3d52a455fe3ab0336ac95923edd","width":128,"height":96,"cases":rows})).unwrap()).unwrap();
    let mut wide = Vec::new();
    for y in 0..96u32 {
        for x in 0..128u32 {
            let alpha = if y < 48 {
                65535
            } else {
                (x * 443 + y * 123) % 65536
            };
            wide.extend([
                ((x * 541 + y * 73) % 65536).min(alpha) as u16,
                ((x * 117 + y * 397) % 65536).min(alpha) as u16,
                ((x * 313 + y * 227) % 65536).min(alpha) as u16,
                alpha as u16,
            ]);
        }
    }
    fs::write(
        root.join("input16.rgba"),
        wide.iter()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let mut quantized = vec![0; 128 * 96 * 4];
    unsafe {
        dither_quantize16(wide.as_ptr(), quantized.as_mut_ptr(), 128, 96, 512);
    }
    fs::write(root.join("Windows/quantize16.rgba"), quantized).unwrap();
    // Open/export these without changing their parameters to exercise the actual installed Mac application.
    for opaque in [true, false] {
        let mut e = Engine::new();
        let r = if opaque {
            Raster::from_straight(
                128,
                96,
                &straight
                    .chunks_exact(4)
                    .flat_map(|p| [p[0], p[1], p[2], 255])
                    .collect::<Vec<_>>(),
            )
        } else {
            raster.clone()
        };
        let mut d = Document::new(128, 96);
        let l = Layer::with_pixels("Alpha effects", r, Point { x: 0., y: 0. });
        let layer = l.id;
        d.layers.push(l);
        d.active_layer_id = Some(layer);
        let id = e.insert_document(d);
        let fx:LayerEffects=serde_json::from_value(json!({"colorOverlay":{"red":0.9,"green":0.2,"blue":0.1,"opacity":0.42},"innerGlow":{"size":6,"red":0.2,"green":0.9,"blue":0.3,"opacity":0.57},"innerShadow":{"angle":135,"distance":3,"blur":4,"red":0,"green":0,"blue":0,"opacity":0.53},"shadow":{"angle":135,"distance":7,"blur":3,"red":0,"green":0,"blue":0,"opacity":0.6}})).unwrap();
        e.execute(
            id,
            Command::SetLayerEffects {
                id: layer,
                effects: Some(fx),
            },
        )
        .unwrap();
        let name = if opaque {
            "effects-opaque"
        } else {
            "effects-alpha"
        };
        let folder = root.join(format!("{name}.comp"));
        fs::create_dir(&folder).unwrap();
        let p = e.save_package(id).unwrap();
        fs::write(folder.join("manifest.json"), p.manifest_json).unwrap();
        for (name, bytes) in p.images {
            let path = folder.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        }
        fs::write(
            root.join("Windows").join(format!("{name}.png")),
            e.export_png(id).unwrap(),
        )
        .unwrap();
    }
}
