//! Fixed Compositor v1.4.9 CRT3 tables and Swift stage orchestration (MIT, 0ab3ff5).
use super::camera_raw::{CameraRawCurve, CameraRawGrading, CameraRawMixer, CameraRawSettings};
use crate::Raster;
use std::cell::RefCell;

pub const SIZE: i32 = 17;
pub const GRID: i32 = 33;
const CELLS: usize = 17 * 17 * 17 * 3;
static TABLES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/camera-raw-tables.rgb"));
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Stage {
    table: [*const u8; 8],
    weight: [f32; 8],
}
extern "C" {
    fn camera_raw_compose(out: *mut f32, grid: i32, stages: *const Stage, count: i32, size: i32);
    fn camera_raw_compose_onto(
        out: *mut f32,
        grid: i32,
        stages: *const Stage,
        count: i32,
        size: i32,
    );
    fn camera_raw_stage_color(stage: *const Stage, size: i32, rgb: *mut f64);
    fn camera_raw_statistics(bytes: *const u8, w: usize, h: usize, stride: usize, out: *mut f64);
    fn camera_raw_table_curves(
        out: *mut f32,
        grid: i32,
        tone: *const f32,
        r: *const f32,
        g: *const f32,
        b: *const f32,
    );
}
fn table(index: usize) -> *const u8 {
    TABLES[index * CELLS..].as_ptr()
}
pub fn exposure_tables() -> *const u8 {
    table(81)
}
fn blend(parts: &[(Option<usize>, f64)]) -> Stage {
    let mut out = Stage {
        table: [std::ptr::null(); 8],
        weight: [0.; 8],
    };
    for (i, (index, weight)) in parts.iter().take(8).enumerate() {
        out.table[i] = index.map_or(std::ptr::null(), table);
        out.weight[i] = *weight as f32;
    }
    out
}
fn between(value: f64, points: &[f64]) -> (usize, f64) {
    let value = value.clamp(points[0], *points.last().unwrap());
    let mut i = 0;
    while i < points.len() - 2 && value > points[i + 1] {
        i += 1;
    }
    (i, (value - points[i]) / (points[i + 1] - points[i]))
}
fn regular(value: f64, low: f64, step: f64, count: usize) -> (usize, f64) {
    let p = ((value - low) / step).clamp(0., (count - 1) as f64);
    let i = (p as usize).min(count - 2);
    (i, p - i as f64)
}
fn stage(start: usize, value: f64, low: f64, step: f64, count: usize) -> Stage {
    let (i, f) = regular(value, low, step, count);
    blend(&[(Some(start + i), 1. - f), (Some(start + i + 1), f)])
}
fn fifty(start: usize, v: f64) -> Stage {
    stage(start, v, -100., 50., 5)
}
fn adaptive(start: usize, value: f64, brightness: f64) -> Stage {
    let (r, d) = between(brightness, &[53., 80., 104., 130., 160., 190., 224.]);
    let (c, a) = between(value, &[-100., -50., -25., 0., 25., 50., 100.]);
    let b = start + r * 7 + c;
    blend(&[
        (Some(b), (1. - d) * (1. - a)),
        (Some(b + 1), (1. - d) * a),
        (Some(b + 7), d * (1. - a)),
        (Some(b + 8), d * a),
    ])
}
fn white_balance(t: f64, tint: f64) -> Stage {
    let (r, d) = regular(t, -100., 25., 9);
    let (c, a) = regular(tint, -100., 25., 9);
    let b = r * 9 + c;
    blend(&[
        (Some(b), (1. - d) * (1. - a)),
        (Some(b + 1), (1. - d) * a),
        (Some(b + 9), d * (1. - a)),
        (Some(b + 10), d * a),
    ])
}
thread_local! { static BRIGHTNESS: RefCell<Option<(Raster, [f64;3])>> = const { RefCell::new(None) }; }
thread_local! { static COMPOSED: RefCell<Option<(Raster, CameraRawSettings, bool, Option<Vec<f32>>)>> = const { RefCell::new(None) }; }
pub(crate) fn clear_cache() {
    super::camera_raw_scope::clear_cache();
    BRIGHTNESS.with(|c| *c.borrow_mut() = None);
    COMPOSED.with(|c| *c.borrow_mut() = None);
}
pub fn lookup(
    s: &CameraRawSettings,
    source: &Raster,
    fold: bool,
    tone: &[f32; 256],
    channels: &[[f32; 256]; 3],
) -> Option<Vec<f32>> {
    COMPOSED.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some((old, settings, folded, result)) = &*cache {
            if old.same_pixels(source) && settings == s && *folded == fold {
                return result.clone();
            }
        }
        let mut result = compose(&light_stages(s, source));
        if fold && s.curve_color() {
            result = Some(fold_color(s, result, tone, channels));
        }
        *cache = Some((source.clone(), s.clone(), fold, result.clone()));
        result
    })
}
fn brightness(source: &Raster) -> [f64; 3] {
    BRIGHTNESS.with(|cache| {
        let mut cache = cache.borrow_mut();
        // Retain the source Arc: its address cannot be recycled while this cache owns it.
        if let Some((old, value)) = &*cache {
            if old.bytes().as_ptr() == source.bytes().as_ptr()
                && old.width == source.width
                && old.height == source.height
            {
                return *value;
            }
        }
        let mut out = [0.; 3];
        unsafe {
            camera_raw_statistics(
                source.bytes().as_ptr(),
                source.width as usize,
                source.height as usize,
                source.width as usize * 4,
                out.as_mut_ptr(),
            );
        }
        let out = out.map(|v| v * 255.);
        *cache = Some((source.clone(), out));
        out
    })
}
pub fn measured_curve(c: &CameraRawCurve) -> bool {
    c.shadow_split == 25.
        && c.dark_split == 50.
        && c.light_split == 75.
        && c.refine_saturation == 0.
}
pub fn light_stages(s: &CameraRawSettings, source: &Raster) -> Vec<Stage> {
    let c = &s.calibration;
    let mut out = Vec::new();
    for (i, v) in [
        c.red_hue,
        c.red_saturation,
        c.green_hue,
        c.green_saturation,
        c.blue_hue,
        c.blue_saturation,
    ]
    .into_iter()
    .enumerate()
    {
        if v != 0. {
            out.push(fifty(405 + i * 5, v));
        }
    }
    if s.exposure != 0. {
        out.push(stage(81, s.exposure, -5., 0.5, 21));
    }
    if s.temperature != 0. || s.tint != 0. {
        out.push(white_balance(s.temperature, s.tint));
    }
    let b = if s.contrast != 0. || s.highlights != 0. || s.shadows != 0. {
        brightness(source)
    } else {
        [128.; 3]
    };
    if s.contrast != 0. {
        out.push(adaptive(138, s.contrast, b[0]));
    }
    if s.highlights != 0. {
        out.push(adaptive(187, s.highlights, b[2]));
    }
    if s.shadows != 0. {
        out.push(adaptive(236, s.shadows, b[1]));
    }
    for (start, v) in [
        (102, s.whites),
        (111, s.blacks),
        (120, s.saturation),
        (129, s.vibrance),
    ] {
        if v != 0. {
            out.push(stage(start, v, -100., 25., 9));
        }
    }
    if measured_curve(&s.curve) {
        for (i, v) in [
            s.curve.shadows,
            s.curve.darks,
            s.curve.lights,
            s.curve.highlights,
        ]
        .into_iter()
        .enumerate()
        {
            if v != 0. {
                out.push(fifty(435 + i * 5, v));
            }
        }
    }
    out
}
pub fn mixer_stages(m: &CameraRawMixer) -> Vec<Stage> {
    let mut out = Vec::new();
    for (kind, values) in [&m.luminance, &m.saturation, &m.hue]
        .into_iter()
        .enumerate()
    {
        for (color, v) in values.iter().enumerate() {
            if *v != 0. {
                out.push(fifty(285 + (kind * 8 + color) * 5, *v));
            }
        }
    }
    out
}
pub fn grading_stages(g: &CameraRawGrading) -> Vec<Stage> {
    let mut out = Vec::new();
    for (i, w) in [&g.shadows, &g.midtones, &g.highlights, &g.global]
        .into_iter()
        .enumerate()
    {
        if w.luminance != 0. {
            out.push(fifty(743 + i * 5, w.luminance));
        }
    }
    let b = g.balance / 100.;
    let hi = (if b < 0. { 1. + 1.4 * b } else { 1. + 2. * b }).max(0.);
    let sh = (if b < 0. { 1. - 1.2 * b } else { 1. - 1.4 * b }).max(0.);
    let (blend_low, blend_up) = between(g.blending, &[0., 50., 100.]);
    for (wheel, color, share, blends) in [
        (3, &g.global, 1., false),
        (2, &g.midtones, 1., false),
        (1, &g.highlights, hi, true),
        (0, &g.shadows, sh, true),
    ] {
        if color.saturation * share <= 0. {
            continue;
        }
        let p = color.hue.rem_euclid(360.) / 30.;
        let first = p as usize % 12;
        let second = (first + 1) % 12;
        let around = p - p.trunc();
        let (level, up) = between(color.saturation * share, &[0., 25., 50., 100.]);
        let mut parts = Vec::new();
        let bs = if blends {
            vec![(blend_low, 1. - blend_up), (blend_low + 1, blend_up)]
        } else {
            vec![(1, 1.)]
        };
        for (bi, bw) in bs {
            let base = 455
                + match bi {
                    0 => wheel * 36,
                    1 => 72 + wheel * 36,
                    _ => 216 + wheel * 36,
                };
            for (hue, hw) in [(first, 1. - around), (second, around)] {
                for (sat, sw) in [(level, 1. - up), (level + 1, up)] {
                    parts.push((
                        if sat == 0 {
                            None
                        } else {
                            Some(base + hue * 3 + sat - 1)
                        },
                        bw * hw * sw,
                    ));
                }
            }
        }
        out.push(blend(&parts));
    }
    out
}
pub fn compose(stages: &[Stage]) -> Option<Vec<f32>> {
    if stages.is_empty() {
        return None;
    }
    let mut out = vec![0.; GRID as usize * GRID as usize * GRID as usize * 3];
    unsafe {
        camera_raw_compose(
            out.as_mut_ptr(),
            GRID,
            stages.as_ptr(),
            stages.len() as i32,
            SIZE,
        );
    }
    Some(out)
}
pub fn fold_color(
    s: &CameraRawSettings,
    base: Option<Vec<f32>>,
    tone: &[f32; 256],
    channels: &[[f32; 256]; 3],
) -> Vec<f32> {
    let mut out = base.unwrap_or_else(|| {
        let mut o = vec![0.; GRID as usize * GRID as usize * GRID as usize * 3];
        unsafe {
            camera_raw_compose(o.as_mut_ptr(), GRID, std::ptr::null(), 0, SIZE);
        }
        o
    });
    let c = &s.curve;
    let linear = vec![
        super::settings::CurvePoint { x: 0., y: 0. },
        super::settings::CurvePoint { x: 1., y: 1. },
    ];
    if [
        c.shadows,
        c.darks,
        c.lights,
        c.highlights,
        c.refine_saturation,
    ]
    .iter()
    .any(|v| *v != 0.)
        || [&c.rgb, &c.red, &c.green, &c.blue]
            .iter()
            .any(|p| **p != linear)
    {
        unsafe {
            camera_raw_table_curves(
                out.as_mut_ptr(),
                GRID,
                tone.as_ptr(),
                channels[0].as_ptr(),
                channels[1].as_ptr(),
                channels[2].as_ptr(),
            );
        }
    }
    let mut stages = mixer_stages(&s.mixer);
    stages.extend(grading_stages(&s.grading));
    unsafe {
        camera_raw_compose_onto(
            out.as_mut_ptr(),
            GRID,
            stages.as_ptr(),
            stages.len() as i32,
            SIZE,
        );
    }
    out
}
pub fn ptr(table: &Option<Vec<f32>>) -> *const f32 {
    table.as_ref().map_or(std::ptr::null(), |t| t.as_ptr())
}
pub fn neutralize(rgb: [f64; 3]) -> Option<(f64, f64)> {
    if rgb.iter().any(|v| !v.is_finite() || *v <= 1e-4 || *v > 1.) {
        return None;
    }
    let cast = |t, tint| {
        let s = white_balance(t, tint);
        let mut c = rgb;
        unsafe {
            camera_raw_stage_color(&s, SIZE, c.as_mut_ptr());
        }
        c.into_iter().fold(0f64, f64::max) - c.into_iter().fold(1f64, f64::min)
    };
    let mut best = (0., 0., cast(0., 0.));
    for (step, reach) in [(10., 100.), (2., 10.), (0.5, 2.)] {
        let center = best;
        let n = (2. * reach / step) as usize;
        for ti in 0..=n {
            for ii in 0..=n {
                let t = center.0 - reach + ti as f64 * step;
                let tint = center.1 - reach + ii as f64 * step;
                if t.abs() > 100. || tint.abs() > 100. {
                    continue;
                }
                let found = cast(t, tint);
                if found < best.2 {
                    best = (t, tint, found);
                }
            }
        }
    }
    Some((best.0, best.1))
}
