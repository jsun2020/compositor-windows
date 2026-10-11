//! Independent-reader PDF fixtures, including the documented 100 MP output limit.
use compositor_engine::{
    export_as::{ExportFormat, ExportOptions},
    Document, Engine, Layer, Point, Raster,
};
use std::{fs, path::PathBuf};
fn main() {
    let root = PathBuf::from(std::env::args().nth(1).expect("fresh output directory"));
    assert!(!root.exists());
    fs::create_dir_all(&root).unwrap();
    let mut bytes = Vec::new();
    for y in 0..8 {
        for x in 0..12 {
            bytes.extend([
                (x * 21) as u8,
                (y * 31) as u8,
                ((x + y) * 13) as u8,
                ((x * 17 + y * 13) % 256) as u8,
            ]);
        }
    }
    let mut doc = Document::new(12, 8);
    doc.resolution = 300.;
    let layer = Layer::with_pixels(
        "RGB and alpha",
        Raster::from_straight(12, 8, &bytes),
        Point { x: 0., y: 0. },
    );
    doc.active_layer_id = Some(layer.id);
    doc.layers.push(layer);
    let mut engine = Engine::new();
    let id = engine.insert_document(doc);
    for format in [ExportFormat::PNG, ExportFormat::JPEG, ExportFormat::PDF] {
        let ext = match format {
            ExportFormat::PNG => "png",
            ExportFormat::JPEG => "jpg",
            ExportFormat::PDF => "pdf",
        };
        let options = ExportOptions {
            width: 24,
            height: 16,
            format,
            quality: 0.85,
            background: None,
        };
        fs::write(
            root.join(format!("transparent.{ext}")),
            engine.export_as(id, &options, 0).unwrap(),
        )
        .unwrap();
    }
    let options = ExportOptions {
        width: 24,
        height: 16,
        format: ExportFormat::PDF,
        quality: 0.85,
        background: Some([1.; 3]),
    };
    fs::write(
        root.join("white.pdf"),
        engine.export_as(id, &options, 0).unwrap(),
    )
    .unwrap();
    engine.close_document(id);
    let id = engine.new_canvas(10000, 10000, 300., None).unwrap();
    let options = ExportOptions {
        width: 10000,
        height: 10000,
        format: ExportFormat::PDF,
        quality: 0.85,
        background: None,
    };
    let start = std::time::Instant::now();
    let output = engine.export_as(id, &options, 0).unwrap();
    fs::write(root.join("100mp.pdf"), output).unwrap();
    println!("100 MP transparent PDF: {:?}", start.elapsed());
}
