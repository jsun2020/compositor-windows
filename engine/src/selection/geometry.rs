//! Outlines as Core Graphics builds and combines them for the Mac's selection (Selection.swift):
//! polygons, the Marquee's rectangle and ellipse, the canvas clip, union / intersection /
//! difference with the winding rule, and the round-joined band Expand and Contract stroke. The
//! booleans and the stroke are i_overlay's, on its i32 integer engine, in `SUBPIXEL` units, so a
//! whole-pixel shape comes back exactly.
use super::{Contour, SUBPIXEL};
use crate::{Affine, Point, Rect};
use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay::Overlay;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::i_float::int::angle::Angle;
use i_overlay::i_float::int::point::IntPoint;
use i_overlay::mesh::int::arc::ArcOptions;
use i_overlay::mesh::int::stroke::offset::IntStrokeOffset;
use i_overlay::mesh::int::style::{IntLineCap, IntLineJoin, IntStrokeStyle};

/// How far a flattened curve may stray from the curve, in document pixels: an ellipse's edge
/// coverage moves by at most about 1/255 per 0.004 px, so this keeps a flattened ellipse within
/// about 3 levels of the curve on any pixel. Also the chord error of an Expand's round joins.
pub const CURVE_TOLERANCE: f64 = 0.01;

/// A document point in `SUBPIXEL` units, rounded.
pub fn quantize(p: Point) -> [i32; 2] { [(p.x * SUBPIXEL).round() as i32, (p.y * SUBPIXEL).round() as i32] }

/// One closed contour through `points` (`addLines(between:)` then `closeSubpath`).
pub fn polygon(points: &[Point]) -> Contour { points.iter().map(|p| quantize(*p)).collect() }

/// The rectangle `r` as one contour.
pub fn rectangle(r: Rect) -> Contour {
    polygon(&[Point { x: r.x, y: r.y }, Point { x: r.max_x(), y: r.y }, Point { x: r.max_x(), y: r.max_y() }, Point { x: r.x, y: r.max_y() }])
}

/// `CGPath.addEllipse(in: r)`: four cubic Beziers from the right-hand point, with control points
/// at 0.5523 of each half-axis (Core Graphics' circle constant, kappa = 4 (sqrt 2 - 1) / 3), each
/// flattened into chords within `CURVE_TOLERANCE` of the curve (Wang's bound, n = ceil(sqrt(3 L /
/// 4 tol)) for the largest second difference L of its control points).
pub fn ellipse(r: Rect) -> Contour {
    const KAPPA: f64 = 0.552_284_749_830_793_4;
    let (cx, cy, rx, ry) = (r.x + r.width / 2.0, r.y + r.height / 2.0, r.width / 2.0, r.height / 2.0);
    let (kx, ky) = (rx * KAPPA, ry * KAPPA);
    let p = |x: f64, y: f64| Point { x, y };
    // Right, bottom, left, top, and back to the right: the quadrants in y-down order.
    let arcs = [
        [p(cx + rx, cy), p(cx + rx, cy + ky), p(cx + kx, cy + ry), p(cx, cy + ry)],
        [p(cx, cy + ry), p(cx - kx, cy + ry), p(cx - rx, cy + ky), p(cx - rx, cy)],
        [p(cx - rx, cy), p(cx - rx, cy - ky), p(cx - kx, cy - ry), p(cx, cy - ry)],
        [p(cx, cy - ry), p(cx + kx, cy - ry), p(cx + rx, cy - ky), p(cx + rx, cy)],
    ];
    let mut points = Vec::new();
    for [p0, p1, p2, p3] in arcs {
        let second = |a: Point, b: Point, c: Point| ((a.x - 2.0 * b.x + c.x).powi(2) + (a.y - 2.0 * b.y + c.y).powi(2)).sqrt();
        let l = second(p0, p1, p2).max(second(p1, p2, p3));
        let n = ((0.75 * l / CURVE_TOLERANCE).sqrt().ceil() as usize).max(1);
        for i in 0..n {
            let t = i as f64 / n as f64;
            let u = 1.0 - t;
            let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
            points.push(Point { x: a * p0.x + b * p1.x + c * p2.x + d * p3.x, y: a * p0.y + b * p1.y + c * p2.y + d * p3.y });
        }
    }
    polygon(&points)
}

/// The contours mapped through `map` (document points in, document points out), then quantized.
pub fn transformed(contours: &[Contour], map: &Affine) -> Vec<Contour> {
    contours.iter().map(|c| c.iter().map(|p| quantize(map.apply(Point { x: p[0] as f64 / SUBPIXEL, y: p[1] as f64 / SUBPIXEL }))).collect()).collect()
}

fn to_int(contours: &[Contour]) -> Vec<Vec<IntPoint<i32>>> {
    contours.iter().filter(|c| c.len() >= 3).map(|c| c.iter().map(|p| IntPoint::new(p[0], p[1])).collect()).collect()
}
fn from_shapes(shapes: Vec<Vec<Vec<IntPoint<i32>>>>) -> Vec<Contour> {
    shapes.into_iter().flatten().map(|c| c.into_iter().map(|p| [p.x, p.y]).collect()).collect()
}

/// A boolean of two outlines, each filled by the nonzero winding rule (`union`, `intersection`,
/// `subtracting` with `.winding`, macOS 14). The result has no overlapping contours: outer
/// boundaries and holes wound in opposite directions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Boolean { Union, Intersection, Difference }

pub fn combine(a: &[Contour], b: &[Contour], op: Boolean) -> Vec<Contour> {
    let rule = match op { Boolean::Union => OverlayRule::Union, Boolean::Intersection => OverlayRule::Intersect, Boolean::Difference => OverlayRule::Difference };
    let (subject, clip) = (to_int(a), to_int(b));
    from_shapes(Overlay::from_subj_and_clip(subject.as_slice(), clip.as_slice()).overlay(rule, FillRule::NonZero))
}

/// The band `half_width` pixels either side of every contour: `copy(strokingWithWidth: 2 x
/// half_width, lineCap: .round, lineJoin: .round)` (Selection.swift:336), as a filled outline.
/// Round joins turn in steps whose chords stray at most `CURVE_TOLERANCE` from the arc.
pub fn band(contours: &[Contour], half_width: f64) -> Option<Vec<Contour>> {
    let radius = (half_width * SUBPIXEL).round();
    // The chord of an arc of radius r over angle a strays r (1 - cos(a / 2)) from it.
    let step = 2.0 * (1.0 - CURVE_TOLERANCE * SUBPIXEL / radius).clamp(-1.0, 1.0).acos();
    let arc = ArcOptions { max_step: Angle::from_radians(step).unwrap_or(ArcOptions::MIN_STEP), rotation_precision: 32 };
    let style = IntStrokeStyle::new(2 * radius as i32).line_join(IntLineJoin::Round(arc)).start_cap(IntLineCap::Round(arc)).end_cap(IntLineCap::Round(arc));
    let paths = to_int(contours);
    // i_overlay's stroke assumes its safe coordinate range (2^30 units) without checking it (a debug
    // panic, a garbled band in release), so it is checked first: None, never an empty band, which
    // would quietly drop the selection on Expand (final review M2).
    paths.as_slice().validate_stroke(&style).ok()?;
    paths.as_slice().stroke(&style, true).ok().map(from_shapes)
}
