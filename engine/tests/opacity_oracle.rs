use compositor_engine::*;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Row {
    backdrop: [u8; 4],
    source: [u8; 4],
    opacity_byte: u8,
    mode: BlendMode,
    result: [u8; 4],
}

#[test]
fn covered_layers_match_independent_core_graphics_rgba8_oracles() {
    // Generated one-pixel framework probes, not user image content. Multiply
    // at full opacity uses a separate CG integer path and is not claimed here.
    let rows: Vec<Row> = serde_json::from_str(include_str!("fixtures/core-graphics-opacity.json")).unwrap();
    assert_eq!(rows.len(), 204);
    for row in rows {
        let mut doc = Document::new(1, 1);
        doc.layers.push(Layer::with_pixels("backdrop", Raster::from_premultiplied(1, 1, row.backdrop.to_vec()), Point { x: 0.0, y: 0.0 }));
        let mut source = Layer::with_pixels("source", Raster::from_premultiplied(1, 1, row.source.to_vec()), Point { x: 0.0, y: 0.0 });
        source.opacity = row.opacity_byte as f64 / 255.0;
        source.blend_mode = row.mode;
        doc.layers.push(source);
        let actual = compositor::composite(&doc, Rect { x: 0.0, y: 0.0, width: 1.0, height: 1.0 }, 1, 1);
        assert_eq!(actual.pixel(0, 0), row.result, "backdrop={:?} source={:?} opacity={} mode={:?}", row.backdrop, row.source, row.opacity_byte, row.mode);
    }
}
