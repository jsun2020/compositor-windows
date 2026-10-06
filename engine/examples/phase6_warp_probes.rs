//! Synthetic, font-free projects for actual Mac/Windows gesture acceptance.
//! cargo run -p compositor-engine --example phase6_warp_probes -- <new directory>
use compositor_engine::*;
use std::{fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("Pass a new output directory")?,
    );
    // Refuse replacement of any existing operator evidence.
    fs::create_dir(&root)?;
    for (name, styled) in [
        ("01-liquify", false),
        ("02-smudge", false),
        ("03-styled-mask", true),
    ] {
        let (w, h) = (640u32, 480u32);
        let mut doc = Document::new(w, h);
        doc.layers.push(Layer::with_pixels(
            "Background",
            Raster::from_premultiplied(w, h, [238, 238, 238, 255].repeat((w * h) as usize)),
            Point { x: 0.0, y: 0.0 },
        ));
        let mut bytes = vec![0; (w * h * 4) as usize];
        for y in 80..400 {
            for x in 120..520 {
                let color = if (x as i32 - 260).pow(2) + (y as i32 - 240).pow(2) < 45 * 45 {
                    [255, 255, 255, 255]
                } else if x / 20 % 2 == 0 {
                    [30, 110, 220, 255]
                } else {
                    [235, 95, 30, 255]
                };
                let i = ((y * w + x) * 4) as usize;
                bytes[i..i + 4].copy_from_slice(&color);
            }
        }
        let mut layer = Layer::with_pixels(
            "Warp this layer",
            Raster::from_premultiplied(w, h, bytes),
            Point { x: 0.0, y: 0.0 },
        );
        if styled {
            layer.opacity = 0.8;
            layer.mask = Some(Mask {
                pixels: GrayRaster::from_bytes(
                    w,
                    h,
                    (0..w * h)
                        .map(|i| if i % w < 320 { 160 } else { 255 })
                        .collect(),
                ),
                enabled: true,
                placement: None,
                linked: Some(true),
            });
            layer.extra.effects = Some(serde_json::from_value(serde_json::json!({
                "stroke": {"size": 3, "red": 0, "green": 0, "blue": 0, "opacity": 1, "inside": false},
                "shadow": {"angle": 45, "distance": 8, "blur": 4, "red": 0, "green": 0, "blue": 0, "opacity": 0.6}
            }))?);
        }
        doc.active_layer_id = Some(layer.id);
        doc.layers.push(layer);
        let package = save_package(&doc)?;
        let reopened = open_package(&package)?;
        assert_eq!(doc.manifest(), reopened.manifest());
        for (a, b) in doc.layers.iter().zip(&reopened.layers) {
            assert_eq!(a.pixels, b.pixels);
            assert_eq!(a.mask, b.mask);
        }
        let folder = root.join(format!("{name}.comp"));
        fs::create_dir(&folder)?;
        fs::create_dir(folder.join("images"))?;
        fs::write(folder.join("manifest.json"), package.manifest_json)?;
        for (file, image) in package.images {
            fs::write(folder.join("images").join(file), image)?;
        }
        let rendered = composite(
            &doc,
            Rect {
                x: 0.0,
                y: 0.0,
                width: w as f64,
                height: h as f64,
            },
            w,
            h,
        );
        let png = encode_png(&rendered, doc.resolution)?;
        assert_eq!(decode_package_png(&png)?.bytes(), rendered.bytes());
        fs::write(root.join(format!("{name}-seed.png")), png)?;
    }
    fs::write(
        root.join("README.md"),
        include_str!("../../docs/superpowers/phase6-manual-checks.md"),
    )?;
    fs::write(root.join("RESULT.txt"), "Platform / app build:\n01 Liquify: live preview / Escape / undo / redo / save-reopen:\n02 Smudge: live preview / fading trail / undo / redo / save-reopen:\n03 Styled-mask: effects / mask preserved / selection / mask refusal:\nNotes or failed steps:\n")?;
    println!(
        "Verified three synthetic project round-trips and seed PNGs: {}",
        root.display()
    );
    Ok(())
}
