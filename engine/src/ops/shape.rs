//! The Shape tool's layers (Phase 4b-1), as Compositor for Mac makes them (ShapeTool.swift): a
//! rectangle (its corners rounded by at most half its shorter side), an ellipse, or a line stroked
//! with round ends, filled with one colour on a new layer above the active one, in one undo step.
//! The layer keeps a `shape` record the Mac reads (`LayerShapeStyle`), so the Mac redraws the shape
//! when the layer is scaled there; this port does not redraw it (ruling OQ12). Any other change to
//! the layer's pixels drops the record (`Layer::set_pixels`), as the Mac's `liveShape` does.
use crate::selection::coverage::rasterize;
use crate::selection::geometry::{cubic, ellipse, polygon, rectangle};
use crate::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The three kinds, named as the Mac names them (`ShapeKind`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShapeKind { Rectangle, Ellipse, Line }

impl ShapeKind {
    pub fn name(self) -> &'static str {
        match self { ShapeKind::Rectangle => "Rectangle", ShapeKind::Ellipse => "Ellipse", ShapeKind::Line => "Line" }
    }
}

/// What the Shape tool made: a box in document pixels (the Marquee's `DragBox`, whole pixels) for a
/// rectangle or an ellipse; for a line, the two points it was dragged between and its width.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ShapeSpec {
    Rectangle { rect: Rect, #[serde(rename = "cornerRadius", default)] corner_radius: f64 },
    Ellipse { rect: Rect },
    Line { start: Point, end: Point, width: f64 },
}

/// The words the Mac shows when a shape would hold too many pixels (ShapeTool.swift:131), with this
/// port's limit.
pub const SHAPE_TOO_LARGE: &str = "That shape is too large. A shape can cover up to 100 megapixels.";

impl ShapeSpec {
    pub fn kind(&self) -> ShapeKind {
        match self { ShapeSpec::Rectangle { .. } => ShapeKind::Rectangle, ShapeSpec::Ellipse { .. } => ShapeKind::Ellipse, ShapeSpec::Line { .. } => ShapeKind::Line }
    }
    /// The layer's box: the rectangle, or a line's two ends grown by half its width on every side
    /// (`finishShape`, ShapeTool.swift:123-128).
    pub fn bounds(&self) -> Rect {
        match self {
            ShapeSpec::Rectangle { rect, .. } | ShapeSpec::Ellipse { rect } => *rect,
            ShapeSpec::Line { start, end, width } => Rect {
                x: start.x.min(end.x) - width / 2.0, y: start.y.min(end.y) - width / 2.0,
                width: (end.x - start.x).abs() + width, height: (end.y - start.y).abs() + width,
            },
        }
    }
    fn check(&self) -> Result<(), CommandError> {
        let numbers: Vec<f64> = match self {
            ShapeSpec::Rectangle { rect, corner_radius } => vec![rect.x, rect.y, rect.width, rect.height, *corner_radius],
            ShapeSpec::Ellipse { rect } => vec![rect.x, rect.y, rect.width, rect.height],
            ShapeSpec::Line { start, end, width } => vec![start.x, start.y, end.x, end.y, *width],
        };
        if numbers.iter().any(|v| !v.is_finite() || v.abs() > MAX_SIDE as f64 * 4.0) {
            return Err(CommandError::Argument("shape out of range".into()));
        }
        if let ShapeSpec::Line { width, .. } = self { if !(*width > 0.0) { return Err(CommandError::Argument("a line needs a width".into())); } }
        Ok(())
    }
}

/// The outline of a rectangle `r` with corners rounded by `radius` (already clamped): Core Graphics'
/// `CGPath(roundedRect:cornerWidth:cornerHeight:)`, each corner a quarter circle as one cubic Bezier
/// with the circle constant, flattened as the ellipse is.
fn rounded_rectangle(r: Rect, radius: f64) -> Contour {
    const KAPPA: f64 = 0.552_284_749_830_793_4;
    let k = radius * KAPPA;
    let p = |x: f64, y: f64| Point { x, y };
    let (x0, y0, x1, y1) = (r.x, r.y, r.max_x(), r.max_y());
    // Clockwise in y-down from the top edge's left end: each straight edge to a corner, then the
    // corner's curve; the last curve ends where the contour began.
    let mut points = vec![p(x0 + radius, y0), p(x1 - radius, y0)];
    cubic(&mut points, [p(x1 - radius, y0), p(x1 - radius + k, y0), p(x1, y0 + radius - k), p(x1, y0 + radius)]);
    points.push(p(x1, y1 - radius));
    cubic(&mut points, [p(x1, y1 - radius), p(x1, y1 - radius + k), p(x1 - radius + k, y1), p(x1 - radius, y1)]);
    points.push(p(x0 + radius, y1));
    cubic(&mut points, [p(x0 + radius, y1), p(x0 + radius - k, y1), p(x0, y1 - radius + k), p(x0, y1 - radius)]);
    points.push(p(x0, y0 + radius));
    cubic(&mut points, [p(x0, y0 + radius), p(x0, y0 + radius - k), p(x0 + radius - k, y0), p(x0 + radius, y0)]);
    points.pop();
    polygon(&points)
}

/// A line from `a` to `b` stroked `width` wide with round caps (`setLineCap(.round)`): two sides and
/// two half circles, each half circle two quarter-circle Beziers. A line of no length is a dot.
fn capsule(a: Point, b: Point, width: f64) -> Contour {
    const KAPPA: f64 = 0.552_284_749_830_793_4;
    let h = width / 2.0;
    let length = (b.x - a.x).hypot(b.y - a.y);
    if length == 0.0 { return ellipse(Rect { x: a.x - h, y: a.y - h, width, height: width }); }
    // Along the line (u) and across it (n), each h long; k of them for the Bezier handles.
    let (ux, uy) = ((b.x - a.x) / length * h, (b.y - a.y) / length * h);
    let (nx, ny) = (-uy, ux);
    let k = KAPPA;
    let p = |x: f64, y: f64| Point { x, y };
    // Along the n side from a to b, round b's end, back along the other side, round a's end; the
    // last curve ends where the contour began.
    let mut points = vec![p(a.x + nx, a.y + ny), p(b.x + nx, b.y + ny)];
    cubic(&mut points, [p(b.x + nx, b.y + ny), p(b.x + nx + k * ux, b.y + ny + k * uy), p(b.x + ux + k * nx, b.y + uy + k * ny), p(b.x + ux, b.y + uy)]);
    cubic(&mut points, [p(b.x + ux, b.y + uy), p(b.x + ux - k * nx, b.y + uy - k * ny), p(b.x - nx + k * ux, b.y - ny + k * uy), p(b.x - nx, b.y - ny)]);
    points.push(p(a.x - nx, a.y - ny));
    cubic(&mut points, [p(a.x - nx, a.y - ny), p(a.x - nx - k * ux, a.y - ny - k * uy), p(a.x - ux - k * nx, a.y - uy - k * ny), p(a.x - ux, a.y - uy)]);
    cubic(&mut points, [p(a.x - ux, a.y - uy), p(a.x - ux + k * nx, a.y - uy + k * ny), p(a.x + nx - k * ux, a.y + ny - k * uy), p(a.x + nx, a.y + ny)]);
    points.pop();
    polygon(&points)
}

/// The shape drawn into a layer of `bounds`: `Int(width)` x `Int(height)` pixels, the shape laid on
/// its box from the pixel grid's corner, filled with `color` at the area it covers (Core Graphics'
/// antialiased fill), premultiplied (`shapeImage`, ShapeTool.swift:197-223).
pub fn shape_raster(spec: &ShapeSpec, color: [f64; 3]) -> Raster {
    let bounds = spec.bounds();
    let (w, h) = (bounds.width as u32, bounds.height as u32);
    let local = Rect { x: 0.0, y: 0.0, width: bounds.width, height: bounds.height };
    let contour = match spec {
        ShapeSpec::Rectangle { corner_radius, .. } => {
            let radius = corner_radius.max(0.0).min(bounds.width / 2.0).min(bounds.height / 2.0);
            if radius > 0.0 { rounded_rectangle(local, radius) } else { rectangle(local) }
        }
        ShapeSpec::Ellipse { .. } => ellipse(local),
        ShapeSpec::Line { start, end, width } => {
            let at = |q: Point| Point { x: q.x - bounds.x, y: q.y - bounds.y };
            capsule(at(*start), at(*end), width.max(1.0))
        }
    };
    let coverage = rasterize(&[contour], 0.0, 0.0, w, h, true);
    let c = color.map(|v| v.clamp(0.0, 1.0) * 255.0);
    let data = coverage.bytes().iter().flat_map(|&k| {
        let a = k as f64 / 255.0;
        [(c[0] * a + 0.5) as u8, (c[1] * a + 0.5) as u8, (c[2] * a + 0.5) as u8, k]
    }).collect();
    Raster::from_premultiplied(w, h, data)
}

/// A number as Swift's JSONEncoder writes a CGFloat: a whole one without a fraction.
fn swift_number(v: f64) -> serde_json::Value {
    if v.fract() == 0.0 && v.abs() < 1e15 { serde_json::json!(v as i64) } else { serde_json::json!(v) }
}

/// The layer's `shape` record, as the Mac's `LayerShapeStyle` encodes: the kind, the colour, the
/// corner radius (0 but for rectangles), and for a line its width and its two ends as fractions of
/// the layer's box (ShapeTool.swift:136-145).
pub fn shape_record(spec: &ShapeSpec, color: [f64; 3]) -> serde_json::Value {
    let mut record = serde_json::json!({
        "kind": spec.kind().name(),
        "red": swift_number(color[0]), "green": swift_number(color[1]), "blue": swift_number(color[2]),
        "cornerRadius": swift_number(match spec { ShapeSpec::Rectangle { corner_radius, .. } => *corner_radius, _ => 0.0 }),
    });
    if let ShapeSpec::Line { start, end, width } = spec {
        let b = spec.bounds();
        let unit = |p: &Point| serde_json::json!([
            swift_number(if b.width > 0.0 { (p.x - b.x) / b.width } else { 0.5 }),
            swift_number(if b.height > 0.0 { (p.y - b.y) / b.height } else { 0.5 })]);
        record["lineWidth"] = swift_number(*width);
        record["start"] = unit(start);
        record["end"] = unit(end);
    }
    record
}

/// "Rectangle 1", "Ellipse 2", ... skipping names already in the document (`nextShapeName`).
pub fn next_shape_name(doc: &Document, kind: ShapeKind) -> String {
    let mut n = 1;
    while doc.layers.iter().any(|l| l.name == format!("{} {n}", kind.name())) { n += 1; }
    format!("{} {n}", kind.name())
}

/// Draws the shape on a new layer above the active one and makes it active; the selection stays
/// (`finishShape`, `addPixelLayer(dropsSelection: false)`). Refused when its box is under a pixel
/// on either side (a click makes nothing) or its pixels pass what the project may hold.
pub fn add_shape(doc: &mut Document, spec: &ShapeSpec, color: [f64; 3]) -> Result<Uuid, CommandError> {
    spec.check()?;
    if color.iter().any(|v| !v.is_finite() || !(0.0..=1.0).contains(v)) { return Err(CommandError::Argument("shape colour out of range".into())); }
    let bounds = spec.bounds();
    if !(bounds.width >= 1.0 && bounds.height >= 1.0) { return Err(CommandError::Argument("a shape needs a box at least a pixel on each side".into())); }
    let (w, h) = (bounds.width as u64, bounds.height as u64);
    if w as i64 > MAX_SIDE || h as i64 > MAX_SIDE || w * h > MAX_PIXELS.saturating_sub(doc.used_pixels()) {
        return Err(CommandError::Refused(SHAPE_TOO_LARGE.into()));
    }
    let mut layer = Layer::with_pixels(&next_shape_name(doc, spec.kind()), shape_raster(spec, color), Point { x: bounds.x, y: bounds.y });
    layer.extra.shape = Some(shape_record(spec, color));
    ops::layers::insert_above_active(doc, layer)
}
