use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(from = "(f64, f64)")]
pub struct Point { pub x: f64, pub y: f64 }
impl From<(f64, f64)> for Point { fn from((x, y): (f64, f64)) -> Self { Point { x, y } } }
impl From<Point> for (f64, f64) { fn from(p: Point) -> Self { (p.x, p.y) } }

#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(from = "(f64, f64)")]
pub struct Size { pub width: f64, pub height: f64 }
impl From<(f64, f64)> for Size { fn from((width, height): (f64, f64)) -> Self { Size { width, height } } }
impl From<Size> for (f64, f64) { fn from(s: Size) -> Self { (s.width, s.height) } }

/// Swift's JSONEncoder writes a whole-number Double without a fraction (`120`, not `120.0`), and a
/// manifest's transforms go through it (3.5a M2). Otherwise a plain f64, non-finite included:
/// `LayerTransform::is_valid` keeps those out of every manifest.
struct Whole(f64);
impl Serialize for Whole {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if self.0.is_finite() && self.0.fract() == 0.0 && self.0.abs() < 1e15 { s.serialize_i64(self.0 as i64) } else { s.serialize_f64(self.0) }
    }
}
fn whole<S: serde::Serializer>(value: &f64, s: S) -> Result<S::Ok, S::Error> { Whole(*value).serialize(s) }
impl Serialize for Point { fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> { (Whole(self.x), Whole(self.y)).serialize(s) } }
impl Serialize for Size { fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> { (Whole(self.width), Whole(self.height)).serialize(s) } }

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
    #[serde(default, serialize_with = "whole")] pub rotation: f64,
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
        // Never -0.0: the Mac decodes it the same, but a file should not carry it (3.5a M2).
        r.rotation = if self.rotation == 0.0 { 0.0 } else { -self.rotation };
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
    /// Matrix product: `self` applied after `other` (same as `then`, named for readability in chains).
    pub fn mul(self, other: Affine) -> Affine { self.then(other) }
}

impl LayerTransform {
    /// The unit square (0..1, y down) mapped where this transform places a layer.
    pub fn unit_to_document(&self) -> Affine { self.pixel_to_document(1, 1) }

    /// A transform placing the unit square as `map` does: a rotated, maybe flipped rectangle (shear is dropped).
    /// Keeps this transform's horizontal flip and the rotation nearest this one's, and its sampling.
    pub fn placing(&self, map: Affine) -> LayerTransform {
        let sign = if self.flip_x { -1.0 } else { 1.0 };
        let angle = (map.b * sign).atan2(map.a * sign);
        let along = -map.c * angle.sin() + map.d * angle.cos();
        let middle = map.apply(Point { x: 0.5, y: 0.5 });
        let mut result = *self;
        result.size = Size { width: (map.a * map.a + map.b * map.b).sqrt(), height: along.abs() };
        let degrees = angle * 180.0 / std::f64::consts::PI;
        result.rotation = degrees + ((self.rotation - degrees) / 360.0).round() * 360.0;
        result.flip_y = along < 0.0;
        result.origin = Point { x: middle.x - result.size.width / 2.0, y: middle.y - result.size.height / 2.0 };
        result
    }

    /// This placement carried along as a layer moves from `old` to `new`.
    pub fn following(&self, old: &LayerTransform, new: &LayerTransform) -> LayerTransform {
        if old.same_placement(new) { return *self; }
        if old.size == new.size && old.rotation == new.rotation && old.flip_x == new.flip_x && old.flip_y == new.flip_y {
            let mut moved = *self;
            moved.origin.x += new.origin.x - old.origin.x;
            moved.origin.y += new.origin.y - old.origin.y;
            return moved;
        }
        let Some(old_inv) = old.unit_to_document().invert() else { return *self; };
        // Apply self's unit map, then old's inverse, then new's map.
        let map = new.unit_to_document().then(old_inv).then(self.unit_to_document());
        self.placing(map)
    }

    /// The same place on the document, whatever the sampling.
    pub fn same_placement(&self, other: &LayerTransform) -> bool {
        self.origin == other.origin && self.size == other.size && self.rotation == other.rotation
            && self.flip_x == other.flip_x && self.flip_y == other.flip_y
    }
    /// Width as a percentage of the pixel size it places (100% draws pixels 1:1).
    pub fn scale_percent(&self, pixel: Size) -> f64 { self.size.width / pixel.width.max(1.0) * 100.0 }
    /// Both sides set to `percent` of `pixel`, keeping the centre, rotation and flips.
    pub fn scaled_to_percent(&self, percent: f64, pixel: Size) -> LayerTransform {
        let center = self.center();
        let mut result = *self;
        result.size = Size { width: pixel.width * percent / 100.0, height: pixel.height * percent / 100.0 };
        result.origin = Point { x: center.x - result.size.width / 2.0, y: center.y - result.size.height / 2.0 };
        result
    }
    /// Whole pixels and whole degrees, what dragging leaves behind.
    pub fn rounded(&self) -> LayerTransform {
        let mut r = *self;
        r.origin = Point { x: self.origin.x.round(), y: self.origin.y.round() };
        r.size = Size { width: self.size.width.round().max(1.0), height: self.size.height.round().max(1.0) };
        r.rotation = self.rotation.round();
        r
    }
    pub fn contains(&self, p: Point) -> bool {
        let c = self.center();
        let (s, co) = self.radians().sin_cos();
        let x = p.x - c.x; let y = p.y - c.y;
        (x * co + y * s).abs() <= self.size.width / 2.0 && (-x * s + y * co).abs() <= self.size.height / 2.0
    }
}

/// A projective map of the plane, row-major 3x3.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Homography { pub m: [[f64; 3]; 3] }

impl Homography {
    /// Maps the unit square (0,0),(1,0),(1,1),(0,1) onto `c` (handle order TL, TR, BR, BL).
    pub fn unit_to(c: &[Point; 4]) -> Homography {
        let sx = c[0].x - c[1].x + c[2].x - c[3].x;
        let sy = c[0].y - c[1].y + c[2].y - c[3].y;
        let (mut g, mut h) = (0.0, 0.0);
        if sx.abs() > 1e-9 || sy.abs() > 1e-9 {
            let dx1 = c[1].x - c[2].x; let dx2 = c[3].x - c[2].x;
            let dy1 = c[1].y - c[2].y; let dy2 = c[3].y - c[2].y;
            let den = dx1 * dy2 - dx2 * dy1;
            if den.abs() > 1e-12 {
                g = (sx * dy2 - dx2 * sy) / den;
                h = (dx1 * sy - sx * dy1) / den;
            }
        }
        let a = c[1].x - c[0].x + g * c[1].x; let b = c[3].x - c[0].x + h * c[3].x;
        let d = c[1].y - c[0].y + g * c[1].y; let e = c[3].y - c[0].y + h * c[3].y;
        Homography { m: [[a, b, c[0].x], [d, e, c[0].y], [g, h, 1.0]] }
    }
    pub fn apply(&self, p: Point) -> Point {
        let m = &self.m;
        let w = m[2][0] * p.x + m[2][1] * p.y + m[2][2];
        let w = if w.abs() < 1e-12 { 1e-12 } else { w };
        Point { x: (m[0][0] * p.x + m[0][1] * p.y + m[0][2]) / w, y: (m[1][0] * p.x + m[1][1] * p.y + m[1][2]) / w }
    }
    pub fn invert(&self) -> Option<Homography> {
        let m = &self.m;
        let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
        if det.abs() < 1e-12 || !det.is_finite() { return None; }
        let inv = |r: usize, c: usize| -> f64 {
            let (r1, r2) = ((r + 1) % 3, (r + 2) % 3); let (c1, c2) = ((c + 1) % 3, (c + 2) % 3);
            (m[c1][r1] * m[c2][r2] - m[c1][r2] * m[c2][r1]) / det
        };
        Some(Homography { m: [[inv(0, 0), inv(0, 1), inv(0, 2)], [inv(1, 0), inv(1, 1), inv(1, 2)], [inv(2, 0), inv(2, 1), inv(2, 2)]] })
    }
    /// Four finite corners making a convex, non-degenerate shape.
    pub fn is_usable(c: &[Point; 4]) -> bool {
        if !c.iter().all(|p| p.x.is_finite() && p.y.is_finite() && p.x.abs() <= 1_000_000.0 && p.y.abs() <= 1_000_000.0) { return false; }
        let mut sign = 0.0;
        for i in 0..4 {
            let a = c[i]; let b = c[(i + 1) % 4]; let d = c[(i + 2) % 4];
            let cross = (b.x - a.x) * (d.y - b.y) - (b.y - a.y) * (d.x - b.x);
            if cross.abs() <= 0.01 { return false; }
            if sign == 0.0 { sign = if cross < 0.0 { -1.0 } else { 1.0 }; } else if (cross < 0.0) != (sign < 0.0) { return false; }
        }
        true
    }
    /// A transform's corners in handle order (unit corners through `point`, flips not applied).
    pub fn corners_of(t: &LayerTransform) -> [Point; 4] {
        [t.point(Point { x: 0.0, y: 0.0 }), t.point(Point { x: 1.0, y: 0.0 }), t.point(Point { x: 1.0, y: 1.0 }), t.point(Point { x: 0.0, y: 1.0 })]
    }
    /// Where `placement`'s corners land when the perspective taking `by`'s corners to `to` is applied around it too.
    pub fn carried(placement: &LayerTransform, by: &LayerTransform, to: &[Point; 4]) -> [Point; 4] {
        let c = by.center();
        // to_unit: translate(-0.5,-0.5) . scale(size) . rotate . translate(center), then inverted.
        let forward = Affine::translation(c.x, c.y).then(Affine::rotation(by.radians()))
            .then(Affine::scale(by.size.width, by.size.height)).then(Affine::translation(-0.5, -0.5));
        let to_unit = forward.invert().unwrap_or(Affine::IDENTITY);
        let map = Homography::unit_to(to);
        let corners = Homography::corners_of(placement);
        [map.apply(to_unit.apply(corners[0])), map.apply(to_unit.apply(corners[1])), map.apply(to_unit.apply(corners[2])), map.apply(to_unit.apply(corners[3]))]
    }
}
