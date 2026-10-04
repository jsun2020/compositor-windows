use compositor_engine::ops::raster_edit::paint_grid;
use compositor_engine::*;

fn source(w: u32, h: u32) -> Vec<u8> {
    (0..w * h)
        .flat_map(|i| {
            let a = (17 + i * 71 % 239) as u8;
            [
                (i * 13 % (a as u32 + 1)) as u8,
                (i * 37 % (a as u32 + 1)) as u8,
                (i * 91 % (a as u32 + 1)) as u8,
                a,
            ]
        })
        .collect()
}

pub fn paint_outputs() -> Vec<u8> {
    let (w, h) = (29, 19);
    let doc = Document::new(37, 23);
    let paints = [
        Paint::Fill([0.3, 0.7, 0.1]),
        Paint::Gradient(GradientSpec {
            shape: GradientShape::Linear,
            start: Point { x: 3.5, y: 4.25 },
            end: Point { x: 27.75, y: 18.5 },
            from: [1.0, 0.2, 0.0, 1.0],
            to: [0.0, 0.0, 1.0, 0.3],
            opacity: 0.9,
        }),
        Paint::Gradient(GradientSpec {
            shape: GradientShape::Radial,
            start: Point { x: 3.5, y: 4.25 },
            end: Point { x: 27.75, y: 18.5 },
            from: [0.4, 0.7, 0.1, 0.0],
            to: [0.8, 0.2, 0.6, 0.75],
            opacity: 0.65,
        }),
    ];
    let coverage = GrayRaster::from_bytes(
        w,
        h,
        (0..w * h)
            .map(|i| [0, 1, 127, 128, 254, 255][(i * 7 % 6) as usize])
            .collect(),
    );
    let mut all = Vec::new();
    for paint in &paints {
        for grey in [false, true] {
            for covered in [false, true] {
                for rotation in [0.0, 23.0, 90.0, 180.0] {
                    let mut placed = LayerTransform::axis_aligned(
                        Point { x: -4.25, y: -3.5 },
                        Size {
                            width: 35.0,
                            height: 25.0,
                        },
                    );
                    placed.rotation = rotation;
                    placed.flip_x = rotation == 90.0;
                    if rotation == 180.0 {
                        placed.origin = Point {
                            x: 1000.0,
                            y: 1000.0,
                        };
                    }
                    let mut data = if grey {
                        (0..w * h).map(|i| (i * 71 % 256) as u8).collect()
                    } else {
                        source(w, h)
                    };
                    let touched = paint_grid(
                        &doc,
                        &mut data,
                        w,
                        h,
                        &placed,
                        covered.then_some(&coverage),
                        paint,
                        grey,
                    );
                    all.push(touched as u8);
                    all.extend_from_slice(&data);
                }
            }
        }
    }
    all
}

pub fn spatial_outputs() -> Vec<u8> {
    let mut all = Vec::new();
    for (w, h) in [(17, 13), (41, 29), (65, 47)] {
        let raster = Raster::from_premultiplied(w, h, source(w, h));
        for sigma in [20.0, 70.0] {
            all.extend_from_slice(blur_for_layer(raster.clone(), sigma).bytes());
        }
        for (angle, distance) in [(-37.0, 140.0), (30.0, 2000.0)] {
            all.extend_from_slice(streak_for_layer(raster.clone(), angle, distance).bytes());
        }
    }
    all
}

#[cfg(not(test))]
pub fn benchmark() {
    use std::{hint::black_box, time::Instant};
    let w = 625;
    let h = 625;
    let doc = Document::new(10000, 10000);
    let placed = LayerTransform::axis_aligned(
        Point { x: 0.0, y: 0.0 },
        Size {
            width: 10000.0,
            height: 10000.0,
        },
    );
    let paint = Paint::Gradient(GradientSpec {
        shape: GradientShape::Linear,
        start: Point {
            x: 2000.0,
            y: 3000.0,
        },
        end: Point {
            x: 8000.0,
            y: 7000.0,
        },
        from: [1.0, 0.2, 0.0, 1.0],
        to: [0.0, 0.0, 1.0, 0.3],
        opacity: 0.9,
    });
    let mut times = Vec::new();
    let original = [128, 102, 77, 255].repeat((w * h) as usize);
    let mut data = original.clone();
    for i in 0..30 {
        data.copy_from_slice(&original);
        let start = Instant::now();
        black_box(paint_grid(
            &doc, &mut data, w, h, &placed, None, &paint, false,
        ));
        black_box(&data);
        if i >= 5 {
            times.push(start.elapsed().as_secs_f64() * 1000.0);
        }
    }
    times.sort_by(f64::total_cmp);
    println!(
        "gradient median_ms={} worst_ms={}",
        times[times.len() / 2],
        times[times.len() - 1]
    );
    let raster = Raster::from_premultiplied(2072, 2000, source(2072, 2000));
    let mut times = Vec::new();
    for i in 0..4 {
        let start = Instant::now();
        black_box(streak_for_layer(raster.clone(), 30.0, 2000.0));
        if i > 0 {
            times.push(start.elapsed().as_secs_f64() * 1000.0);
        }
    }
    times.sort_by(f64::total_cmp);
    println!(
        "spatial median_ms={} worst_ms={}",
        times[times.len() / 2],
        times[times.len() - 1]
    );
}
