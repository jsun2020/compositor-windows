use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(from = "(f64, f64)", into = "(f64, f64)")]
pub struct Point { pub x: f64, pub y: f64 }
impl From<(f64, f64)> for Point { fn from((x, y): (f64, f64)) -> Self { Point { x, y } } }
impl From<Point> for (f64, f64) { fn from(p: Point) -> Self { (p.x, p.y) } }

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(from = "(f64, f64)", into = "(f64, f64)")]
pub struct Size { pub width: f64, pub height: f64 }
impl From<(f64, f64)> for Size { fn from((width, height): (f64, f64)) -> Self { Size { width, height } } }
impl From<Size> for (f64, f64) { fn from(s: Size) -> Self { (s.width, s.height) } }

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rect { pub x: f64, pub y: f64, pub width: f64, pub height: f64 }
impl Rect {
    pub fn max_x(&self) -> f64 { self.x + self.width }
    pub fn max_y(&self) -> f64 { self.y + self.height }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Sampling {
    #[serde(rename = "Nearest")] Nearest,
    #[serde(rename = "Smooth")] Smooth,
    #[default]
    #[serde(rename = "High quality")] High,
}

/// Unrotated bounds in document pixels; rotation is clockwise (y down) around their center.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct LayerTransform {
    pub origin: Point,
    pub size: Size,
    #[serde(default)] pub rotation: f64,
    #[serde(default, rename = "flipX")] pub flip_x: bool,
    #[serde(default, rename = "flipY")] pub flip_y: bool,
    #[serde(default)] pub sampling: Sampling,
}

impl LayerTransform {
    pub fn axis_aligned(origin: Point, size: Size) -> Self {
        LayerTransform { origin, size, rotation: 0.0, flip_x: false, flip_y: false, sampling: Sampling::High }
    }
    pub fn center(&self) -> Point {
        Point { x: self.origin.x + self.size.width / 2.0, y: self.origin.y + self.size.height / 2.0 }
    }
    pub fn radians(&self) -> f64 { (self.rotation % 360.0) * std::f64::consts::PI / 180.0 }
    pub fn is_valid(&self) -> bool {
        [self.origin.x, self.origin.y, self.size.width, self.size.height, self.rotation].iter().all(|v| v.is_finite())
            && (1.0..=300_000.0).contains(&self.size.width) && (1.0..=300_000.0).contains(&self.size.height)
            && self.origin.x.abs() <= 1_000_000.0 && self.origin.y.abs() <= 1_000_000.0
    }
    /// Unit square (0..1, y down) point placed on the document.
    pub fn point(&self, unit: Point) -> Point {
        let x = (unit.x - 0.5) * self.size.width;
        let y = (unit.y - 0.5) * self.size.height;
        let (s, c) = self.radians().sin_cos();
        let center = self.center();
        Point { x: center.x + x * c - y * s, y: center.y + x * s + y * c }
    }
    pub fn corners(&self) -> [Point; 4] {
        [self.point(Point { x: 0.0, y: 0.0 }), self.point(Point { x: 1.0, y: 0.0 }),
         self.point(Point { x: 1.0, y: 1.0 }), self.point(Point { x: 0.0, y: 1.0 })]
    }
    /// Upright bounding box of the rotated rectangle.
    pub fn bounds(&self) -> Rect {
        let c = self.corners();
        let min_x = c.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
        let max_x = c.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
        let min_y = c.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
        let max_y = c.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
        Rect { x: min_x, y: min_y, width: max_x - min_x, height: max_y - min_y }
    }
    /// Mirrored across a vertical line at `axis` (horizontal = true) or a horizontal one.
    pub fn mirrored(&self, horizontal: bool, axis: f64) -> Self {
        let mut r = *self;
        let center = self.center();
        if horizontal {
            r.flip_x = !r.flip_x;
            r.origin.x = 2.0 * axis - center.x - self.size.width / 2.0;
        } else {
            r.flip_y = !r.flip_y;
            r.origin.y = 2.0 * axis - center.y - self.size.height / 2.0;
        }
        r.rotation = -self.rotation;
        r
    }
    /// Maps layer pixel coordinates (0..width, 0..height, y down) to document coordinates.
    pub fn pixel_to_document(&self, width: u32, height: u32) -> Affine {
        let center = self.center();
        let sx = self.size.width / width.max(1) as f64 * if self.flip_x { -1.0 } else { 1.0 };
        let sy = self.size.height / height.max(1) as f64 * if self.flip_y { -1.0 } else { 1.0 };
        Affine::translation(center.x, center.y)
            .then(Affine::rotation(self.radians()))
            .then(Affine::scale(sx, sy))
            .then(Affine::translation(-(width as f64) / 2.0, -(height as f64) / 2.0))
    }
}

/// Row-vector affine map: x' = a*x + c*y + tx, y' = b*x + d*y + ty (CGAffineTransform layout).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine { pub a: f64, pub b: f64, pub c: f64, pub d: f64, pub tx: f64, pub ty: f64 }

impl Affine {
    pub const IDENTITY: Affine = Affine { a: 1.0, b: 0.0, c: 0.0, d: 1.0, tx: 0.0, ty: 0.0 };
    pub fn translation(tx: f64, ty: f64) -> Self { Affine { tx, ty, ..Self::IDENTITY } }
    pub fn scale(sx: f64, sy: f64) -> Self { Affine { a: sx, d: sy, ..Self::IDENTITY } }
    pub fn rotation(radians: f64) -> Self {
        let (s, c) = radians.sin_cos();
        Affine { a: c, b: s, c: -s, d: c, tx: 0.0, ty: 0.0 }
    }
    /// `self` applied after `inner`: matches CGAffineTransform's `.rotated(by:)`, `.scaledBy` chaining,
    /// where each call pre-multiplies, so `then(inner)` applies `inner` to points first.
    pub fn then(self, inner: Affine) -> Affine {
        Affine {
            a: inner.a * self.a + inner.b * self.c,
            b: inner.a * self.b + inner.b * self.d,
            c: inner.c * self.a + inner.d * self.c,
            d: inner.c * self.b + inner.d * self.d,
            tx: inner.tx * self.a + inner.ty * self.c + self.tx,
            ty: inner.tx * self.b + inner.ty * self.d + self.ty,
        }
    }
    pub fn apply(&self, p: Point) -> Point {
        Point { x: self.a * p.x + self.c * p.y + self.tx, y: self.b * p.x + self.d * p.y + self.ty }
    }
    pub fn invert(&self) -> Option<Affine> {
        let det = self.a * self.d - self.b * self.c;
        if det.abs() < 1e-12 || !det.is_finite() { return None; }
        let a = self.d / det; let b = -self.b / det; let c = -self.c / det; let d = self.a / det;
        Some(Affine { a, b, c, d, tx: -(a * self.tx + c * self.ty), ty: -(b * self.tx + d * self.ty) })
    }
}
