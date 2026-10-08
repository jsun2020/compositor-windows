use compositor_engine::ops::raster_edit::paint_grid;
use compositor_engine::*;

// Complete synthetic outputs captured with the pre-optimization release
// engine. Interior, overhanging and non-finite placement share the same inputs.
pub fn outputs() -> Vec<u8> {
    let (w, h) = (13, 11);
    let doc = Document::new(150, 100);
    let coverage = GrayRaster::from_bytes(w, h, (0..w*h)
        .map(|i| [0, 1, 127, 128, 254, 255][i as usize % 6]).collect());
    let paints = [
        Paint::Fill([0.3, 0.7, 0.1]),
        Paint::Gradient(GradientSpec { shape: GradientShape::Linear,
            start: Point { x: 17.25, y: 19.5 }, end: Point { x: 81.75, y: 67.125 },
            from: [1.0, 0.2, 0.0, 1.0], to: [0.0, 0.0, 1.0, 1.0], opacity: 1.0 }),
        Paint::Gradient(GradientSpec { shape: GradientShape::Radial,
            start: Point { x: 17.25, y: 19.5 }, end: Point { x: 81.75, y: 67.125 },
            from: [0.4, 0.7, 0.1, 1.0], to: [0.8, 0.2, 0.6, 1.0], opacity: 1.0 }),
    ];
    let mut all = Vec::new();
    for paint in &paints {
        for grey in [false, true] {
            for covered in [false, true] {
                for case in 0..6 {
                    let mut placed = LayerTransform::axis_aligned(
                        Point { x: 30.25, y: 31.5 }, Size { width: 35.5, height: 21.75 });
                    placed.rotation = [0.0, 23.0, 90.0, 180.0, 23.0, 0.0][case];
                    placed.flip_x = case == 2 || case == 3;
                    placed.flip_y = case == 3;
                    if case == 4 { placed.origin = Point { x: -4.25, y: -3.5 }; }
                    if case == 5 { placed.origin.x = f64::NAN; }
                    let mut data: Vec<u8> = if grey {
                        (0..w*h).map(|i| (i * 71 % 256) as u8).collect()
                    } else {
                        (0..w*h).flat_map(|i| {
                            let a = (17 + i * 71 % 239) as u8;
                            [(i * 13 % (a as u32 + 1)) as u8,
                             (i * 37 % (a as u32 + 1)) as u8,
                             (i * 91 % (a as u32 + 1)) as u8, a]
                        }).collect()
                    };
                    all.push(paint_grid(&doc, &mut data, w, h, &placed,
                        covered.then_some(&coverage), paint, grey) as u8);
                    all.extend_from_slice(&data);
                }
            }
        }
    }
    all
}
