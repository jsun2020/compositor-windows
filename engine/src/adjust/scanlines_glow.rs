//! Portable Scanlines Glow components, validated independently against Mac.
//!
//! This is not the general effects blur. The six Gaussian stages reproduce
//! the independently observed Mac 1.4.9 default path;
//! callers supply input-independent kernel constants, never reference pixels.

#[path = "scanlines_glow/catalog.rs"]
mod catalog;
pub(crate) use catalog::for_spacing;
#[path = "scanlines_glow/resize.rs"]
mod resize;
#[path = "scanlines_glow/sampling.rs"]
pub(crate) mod sampling;
pub(crate) use resize::{bloom, reduce};

const MAX_SIDE: usize = 30_128;
// A 100 MP source reduced by at least two, including its Gaussian halo.
const MAX_PIXELS: usize = 27_000_000;
pub(crate) type Error = &'static str;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Format {
    Half,
    Float,
}

enum Storage {
    Half(Vec<[u16; 4]>),
    Float(Vec<[f32; 4]>),
}

pub(crate) struct Plane {
    pub width: usize,
    pub height: usize,
    storage: Storage,
}

fn area(width: usize, height: usize) -> Result<usize, Error> {
    if width == 0 || height == 0 || width > MAX_SIDE || height > MAX_SIDE {
        return Err("Invalid Scanlines Gaussian dimensions");
    }
    width
        .checked_mul(height)
        .filter(|&n| n <= MAX_PIXELS)
        .ok_or("Scanlines Gaussian memory limit exceeded")
}

fn reserved<T>(count: usize) -> Result<Vec<T>, Error> {
    let mut v = Vec::new();
    v.try_reserve_exact(count)
        .map_err(|_| "Scanlines Gaussian allocation failed")?;
    Ok(v)
}

pub(crate) fn half_float(bits: u16) -> f32 {
    debug_assert!(bits < 0x7c00);
    let exponent = u32::from(bits >> 10);
    let mantissa = u32::from(bits & 0x3ff);
    if exponent == 0 {
        (mantissa as f32) * (1.0 / 16_777_216.0)
    } else {
        f32::from_bits(((exponent + 112) << 23) | (mantissa << 13))
    }
}

/// Positive, finite binary32 -> binary16, round toward zero.
pub(crate) fn half_rtz(value: f32) -> u16 {
    debug_assert!(value.is_finite() && (0.0..=65_504.0).contains(&value));
    let bits = value.to_bits();
    let exponent = (bits >> 23) & 0xff;
    if exponent < 103 {
        0
    } else if exponent < 113 {
        (((bits & 0x7f_ffff) | 0x80_0000) >> (126 - exponent)) as u16
    } else {
        (((exponent - 112) << 10) | ((bits >> 13) & 0x3ff)) as u16
    }
}

/// Positive exact sample -> binary16, nearest, with exact midpoints upward.
/// Do not cast an exact binary64 sample to binary32 before testing its midpoint.
pub(crate) fn half_up(value: f64) -> u16 {
    debug_assert!(value.is_finite() && (0.0..=65_504.0).contains(&value));
    let bits = value.to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as i32 - 1023;
    let floor = if exponent < -24 {
        0
    } else if exponent < -14 {
        (((bits & 0x000f_ffff_ffff_ffff) | 0x0010_0000_0000_0000) >> (28 - exponent)) as u16
    } else {
        (((exponent + 15) as u16) << 10) | ((bits >> 42) as u16 & 0x3ff)
    };
    if floor == 0x7bff {
        return floor;
    }
    let lo = f64::from(half_float(floor));
    let hi = f64::from(half_float(floor + 1));
    if value - lo < hi - value {
        floor
    } else {
        floor + 1
    }
}

impl Plane {
    pub fn from_le_bytes(
        width: usize,
        height: usize,
        format: Format,
        bytes: &[u8],
    ) -> Result<Self, Error> {
        let count = area(width, height)?;
        let component_bytes = if format == Format::Half { 2 } else { 4 };
        if bytes.len() != count * 4 * component_bytes {
            return Err("Invalid Scanlines Gaussian buffer length");
        }
        let mut out = Self::empty(width, height, format)?;
        for pixel in bytes.chunks_exact(component_bytes * 4) {
            let mut values = [0.; 4];
            for (c, value) in values.iter_mut().enumerate() {
                let start = c * component_bytes;
                *value = if format == Format::Half {
                    let bits = u16::from_le_bytes([pixel[start], pixel[start + 1]]);
                    if bits >= 0x7c00 {
                        return Err("Invalid Scanlines Gaussian half value");
                    }
                    half_float(bits)
                } else {
                    f32::from_le_bytes(pixel[start..start + 4].try_into().unwrap())
                };
            }
            out.push(values)?;
        }
        Ok(out)
    }

    fn empty(width: usize, height: usize, format: Format) -> Result<Self, Error> {
        let count = area(width, height)?;
        let storage = match format {
            Format::Half => Storage::Half(reserved(count)?),
            Format::Float => Storage::Float(reserved(count)?),
        };
        Ok(Self {
            width,
            height,
            storage,
        })
    }

    pub fn format(&self) -> Format {
        match self.storage {
            Storage::Half(_) => Format::Half,
            Storage::Float(_) => Format::Float,
        }
    }

    fn push(&mut self, pixel: [f32; 4]) -> Result<(), Error> {
        if pixel
            .iter()
            .any(|v| !v.is_finite() || !(0.0..=65_504.0).contains(v))
        {
            return Err("Invalid Scanlines Gaussian component");
        }
        match &mut self.storage {
            Storage::Half(v) => v.push(pixel.map(half_rtz)),
            Storage::Float(v) => v.push(pixel),
        }
        Ok(())
    }

    fn sample(&self, x: i32, y: i32) -> [f32; 4] {
        let x = x.clamp(0, self.width as i32 - 1) as usize;
        let y = y.clamp(0, self.height as i32 - 1) as usize;
        let i = y * self.width + x;
        match &self.storage {
            Storage::Half(v) => v[i].map(half_float),
            Storage::Float(v) => v[i],
        }
    }

    /// Stream the packed representation without making a second plane-sized copy.
    pub fn write_le(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        match &self.storage {
            Storage::Half(v) => {
                for pixel in v {
                    for c in pixel {
                        writer.write_all(&c.to_le_bytes())?;
                    }
                }
            }
            Storage::Float(v) => {
                for pixel in v {
                    for c in pixel {
                        writer.write_all(&c.to_le_bytes())?;
                    }
                }
            }
        }
        Ok(())
    }
}

pub(crate) struct Kernel {
    pub downsample: [f32; 3],
    /// Symmetric center and five or six positive taps.
    pub fir: Vec<f32>,
    pub upsample: f32,
}

impl Kernel {
    fn validate(&self) -> Result<(), Error> {
        if !matches!(self.fir.len(), 6 | 7)
            || self
                .downsample
                .iter()
                .chain(self.fir.iter())
                .chain([&self.upsample])
                .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
            || self.downsample[1] + self.downsample[2] == 0.
            || self.fir[1..].chunks(2).any(|p| p.iter().sum::<f32>() == 0.)
        {
            return Err("Invalid Scanlines Gaussian kernel");
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Downsample,
    Fir,
    Upsample,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Axis {
    X,
    Y,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Stage {
    pub kind: Kind,
    pub axis: Axis,
    pub width: usize,
    pub height: usize,
    pub origin: [i32; 2],
    pub translation: [i32; 2],
    pub output_format: Format,
}

/// Backwards stencil support. No captured stage rectangles or fixture dimensions.
pub(crate) fn plan(
    width: usize,
    height: usize,
    offset: [i32; 2],
    kernel: &Kernel,
    output_format: Format,
) -> Result<[Stage; 6], Error> {
    area(width, height)?;
    kernel.validate()?;
    if offset.iter().any(|v| v.unsigned_abs() > MAX_SIDE as u32) {
        return Err("Invalid Scanlines Gaussian offset");
    }
    let [ox, oy] = offset;
    let radius = kernel.fir.len() as i32 - 1;
    let nx = ox.div_euclid(2) - 1;
    let ny = oy.div_euclid(2) - 1;
    let bx = (ox + width as i32 - 1).div_euclid(2) + 2 - nx;
    let by = (oy + height as i32 - 1).div_euclid(2) + 2 - ny;
    let ax = nx - radius;
    let ay = ny - radius;
    let aw = bx + 2 * radius;
    let ah = by + 2 * radius;
    let fx = 2 * ax - 2;
    let fw = 2 * (ax + aw - 1) + 4 - fx;
    let s = |kind, axis, w: i32, h: i32, origin, translation, output_format| Stage {
        kind,
        axis,
        width: w as usize,
        height: h as usize,
        origin,
        translation,
        output_format,
    };
    let result = [
        s(
            Kind::Downsample,
            Axis::Y,
            fw,
            ah,
            [fx, ay],
            [0, 0],
            Format::Half,
        ),
        s(
            Kind::Downsample,
            Axis::X,
            aw,
            ah,
            [ax, ay],
            [-fx, -ay],
            Format::Half,
        ),
        s(
            Kind::Fir,
            Axis::Y,
            aw,
            by,
            [ax, ny],
            [-ax, -ay],
            Format::Half,
        ),
        s(
            Kind::Fir,
            Axis::X,
            bx,
            by,
            [nx, ny],
            [-ax, -ny],
            Format::Half,
        ),
        s(
            Kind::Upsample,
            Axis::X,
            width as i32,
            by,
            [ox, ny],
            [-nx, -ny],
            Format::Half,
        ),
        s(
            Kind::Upsample,
            Axis::Y,
            width as i32,
            height as i32,
            [ox, oy],
            [-ox, -ny],
            output_format,
        ),
    ];
    for stage in &result {
        area(stage.width, stage.height)?;
    }
    Ok(result)
}

fn phase(value: f32) -> f32 {
    (value * 256.).round_ties_even() / 256.
}

fn along(source: &Plane, axis: Axis, x: i32, y: i32, distance: i32) -> [f32; 4] {
    match axis {
        Axis::X => source.sample(x + distance, y),
        Axis::Y => source.sample(x, y + distance),
    }
}

fn downsample(source: &Plane, stage: &Stage, kernel: &Kernel, gx: i32, gy: i32) -> [f32; 4] {
    let x = (if stage.axis == Axis::X { 2 * gx } else { gx }) + stage.translation[0];
    let y = (if stage.axis == Axis::Y { 2 * gy } else { gy }) + stage.translation[1];
    let s = |d| along(source, stage.axis, x, y, d);
    let [a, b, c] = kernel.downsample;
    let gain = b + c;
    let middle = 2. * a;
    let t = phase(c / gain);
    let round_sample = |v: f32| {
        if source.format() == Format::Half {
            half_float(half_up(v.into()))
        } else {
            v
        }
    };
    let [n2, n1, z, p1, p2, p3] = [s(-2), s(-1), s(0), s(1), s(2), s(3)];
    std::array::from_fn(|c| {
        let left = round_sample(n2[c] * t + n1[c] * (1. - t));
        let right = round_sample(p2[c] * (1. - t) + p3[c] * t);
        let mid = round_sample((z[c] + p1[c]) * 0.5);
        (left + right).mul_add(gain, mid * middle)
    })
}

fn fir(source: &Plane, stage: &Stage, kernel: &Kernel, gx: i32, gy: i32) -> [f32; 4] {
    let x = gx + stage.translation[0];
    let y = gy + stage.translation[1];
    let s = |d| along(source, stage.axis, x, y, d);
    let mut result = s(0).map(|v| v * kernel.fir[0]);
    let radius = kernel.fir.len() - 1;
    for k in (1..=radius).step_by(2) {
        let next = (k + 1).min(radius);
        let gain = kernel.fir[k] + if k < radius { kernel.fir[k + 1] } else { 0. };
        let t = f64::from(phase(if k < radius {
            kernel.fir[k + 1] / gain
        } else {
            0.
        }));
        let [pa, pb, na, nb] = [
            s(k as i32),
            s(next as i32),
            s(-(k as i32)),
            s(-(next as i32)),
        ];
        for c in 0..4 {
            // The verified one-axis sampler rounds its weighted result to
            // binary32 before the positive half midpoint rule.
            let pos = half_float(half_up(f64::from(
                (f64::from(pa[c]) * (1. - t) + f64::from(pb[c]) * t) as f32,
            )));
            let neg = half_float(half_up(f64::from(
                (f64::from(na[c]) * (1. - t) + f64::from(nb[c]) * t) as f32,
            )));
            result[c] = (pos + neg).mul_add(gain, result[c]);
        }
    }
    result
}

fn upsample(source: &Plane, stage: &Stage, kernel: &Kernel, gx: i32, gy: i32) -> [f32; 4] {
    let g = if stage.axis == Axis::X { gx } else { gy };
    let x = (if stage.axis == Axis::X {
        gx.div_euclid(2)
    } else {
        gx
    }) + stage.translation[0];
    let y = (if stage.axis == Axis::Y {
        gy.div_euclid(2)
    } else {
        gy
    }) + stage.translation[1];
    let a = kernel.upsample;
    // Separate Float32 products/additions are intentional. Contracting these
    // expressions or Horner reordering disagrees with the actual stage outputs.
    let center_weight = 0.75 * a + 0.125;
    let major = 0.703125 - 0.5625 * a;
    let minor = 0.171875 - 0.1875 * a;
    let (left_weight, right_weight) = if g.rem_euclid(2) == 1 {
        (minor, major)
    } else {
        (major, minor)
    };
    let left = along(source, stage.axis, x, y, -1);
    let center = source.sample(x, y);
    let right = along(source, stage.axis, x, y, 1);
    std::array::from_fn(|c| {
        right[c].mul_add(
            right_weight,
            left[c].mul_add(left_weight, center[c] * center_weight),
        )
    })
}

pub(crate) fn gaussian(
    source: &Plane,
    width: usize,
    height: usize,
    offset: [i32; 2],
    kernel: &Kernel,
    output_format: Format,
    mut observe: impl FnMut(usize, &Stage, &Plane) -> Result<(), Error>,
) -> Result<Plane, Error> {
    let stages = plan(width, height, offset, kernel, output_format)?;
    let mut previous = None;
    for (i, stage) in stages.iter().enumerate() {
        let input = previous.as_ref().unwrap_or(source);
        let next = render_stage(input, stage, kernel)?;
        observe(i, stage, &next)?;
        // Only the current and next stage are live, not all six full planes.
        previous = Some(next);
    }
    Ok(previous.unwrap())
}

pub(crate) fn render_stage(source: &Plane, stage: &Stage, kernel: &Kernel) -> Result<Plane, Error> {
    kernel.validate()?;
    if stage
        .origin
        .iter()
        .chain(stage.translation.iter())
        .any(|v| v.unsigned_abs() > MAX_SIDE as u32)
    {
        return Err("Invalid Scanlines Gaussian stage coordinates");
    }
    let mut next = Plane::empty(stage.width, stage.height, stage.output_format)?;
    for y in 0..stage.height {
        for x in 0..stage.width {
            let gx = x as i32 + stage.origin[0];
            let gy = y as i32 + stage.origin[1];
            let pixel = match stage.kind {
                Kind::Downsample => downsample(source, stage, kernel, gx, gy),
                Kind::Fir => fir(source, stage, kernel, gx, gy),
                Kind::Upsample => upsample(source, stage, kernel, gx, gy),
            };
            next.push(pixel)?;
        }
    }
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_positive_finite_half_values_and_midpoints() {
        for bits in 0..0x7c00 {
            let value = half_float(bits);
            assert_eq!(half_rtz(value), bits);
            assert_eq!(half_up(value.into()), bits);
            if bits < 0x7bff {
                let midpoint = (f64::from(value) + f64::from(half_float(bits + 1))) * 0.5;
                assert_eq!(half_up(midpoint), bits + 1);
                assert_eq!(half_rtz(midpoint as f32), bits);
                // Neighboring binary64 values must not double-round to a tie.
                assert_eq!(half_up(f64::from_bits(midpoint.to_bits() - 1)), bits);
            }
        }
    }

    #[test]
    fn invalid_input_is_rejected_before_rendering() {
        assert!(Plane::from_le_bytes(0, 1, Format::Half, &[]).is_err());
        assert!(Plane::from_le_bytes(30_129, 1, Format::Half, &[]).is_err());
        assert!(Plane::from_le_bytes(6000, 6000, Format::Half, &[]).is_err());
        assert!(Plane::from_le_bytes(1, 1, Format::Half, &[0; 7]).is_err());
        for bad in [f32::NAN, f32::INFINITY, -1.0, 65_536.0] {
            let bytes: Vec<_> = [bad; 4].iter().flat_map(|v| v.to_le_bytes()).collect();
            assert!(Plane::from_le_bytes(1, 1, Format::Float, &bytes).is_err());
        }
        for bad in [0x7c00_u16, 0x7e00, 0x8000, 0xbc00] {
            let bytes: Vec<_> = [bad; 4].iter().flat_map(|v| v.to_le_bytes()).collect();
            assert!(Plane::from_le_bytes(1, 1, Format::Half, &bytes).is_err());
        }
    }

    fn reference_kernel() -> Kernel {
        for_spacing(2).unwrap().kernel
    }

    #[test]
    fn actual_mac_constant_field_retains_stage_store_rounding() {
        // Mac149-glow-stages-20261014-205225:
        // direct-flat-powers-float-input-default-sigma4.5. Actual upload is
        // constant [1/8,1/4,1/2,1]. The FIR store loses one half ULP; replacing
        // the chain by a normalized convolution incorrectly preserves it.
        let bytes: Vec<_> = [12288_u16, 13312, 14336, 15360]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        let source = Plane::from_le_bytes(1, 1, Format::Half, &bytes).unwrap();
        let before = [12288_u16, 13312, 14336, 15360];
        let after = [12287_u16, 13311, 14335, 15359];
        let output = gaussian(
            &source,
            2,
            2,
            [14, 14],
            &reference_kernel(),
            Format::Half,
            |i, _, p| {
                let expected = if i < 2 { before } else { after };
                if let Storage::Half(pixels) = &p.storage {
                    assert!(pixels.iter().all(|v| *v == expected), "stage {i}");
                } else {
                    panic!("Expected half stage");
                }
                Ok(())
            },
        )
        .unwrap();
        let mut packed = Vec::new();
        output.write_le(&mut packed).unwrap();
        let expected: Vec<_> = after
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .cycle()
            .take(32)
            .collect();
        assert_eq!(packed, expected);
    }

    #[test]
    fn actual_mac_downsample_half_midpoint_witness() {
        // Captured stage1, dense-037-sigma4.578125, output x28 y29.
        let mut pixels = [[0_u16, 0, 0, 15360]; 6];
        pixels[4] = [13335, 12311, 11287, 15360];
        let bytes: Vec<_> = pixels
            .iter()
            .flatten()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        let source = Plane::from_le_bytes(6, 1, Format::Half, &bytes).unwrap();
        let mut kernel = reference_kernel();
        kernel.downsample = [0x3e82f539, 0x3e3c01a4, 0x3d784fa9].map(f32::from_bits);
        let stage = Stage {
            kind: Kind::Downsample,
            axis: Axis::X,
            width: 1,
            height: 1,
            origin: [1, 0],
            translation: [0, 0],
            output_format: Format::Half,
        };
        let output = render_stage(&source, &stage, &kernel).unwrap();
        assert_eq!(
            output.sample(0, 0),
            [10750, 9726, 8702, 15360].map(half_float)
        );
    }

    #[test]
    fn invalid_kernel_and_stage_fail_without_observation() {
        let source = Plane::from_le_bytes(1, 1, Format::Half, &[0; 8]).unwrap();
        let mut kernel = reference_kernel();
        for invalid in [vec![], vec![0.; 6], vec![f32::NAN; 6]] {
            kernel.fir = invalid;
            assert!(gaussian(
                &source,
                1,
                1,
                [0, 0],
                &kernel,
                Format::Half,
                |_, _, _| panic!()
            )
            .is_err());
        }
        let kernel = reference_kernel();
        assert!(gaussian(
            &source,
            1,
            1,
            [i32::MAX, 0],
            &kernel,
            Format::Half,
            |_, _, _| panic!()
        )
        .is_err());
        assert!(gaussian(
            &source,
            MAX_SIDE,
            MAX_SIDE,
            [0, 0],
            &kernel,
            Format::Half,
            |_, _, _| panic!()
        )
        .is_err());
        let error = gaussian(&source, 1, 1, [0, 0], &kernel, Format::Half, |_, _, _| {
            Err("cancelled")
        });
        assert!(matches!(error, Err("cancelled")));
    }

    #[test]
    fn finite_catalog_rejects_other_spacing_domains() {
        for spacing in 0..=u8::MAX {
            match for_spacing(spacing) {
                Ok(state) => {
                    assert!((2..=32).contains(&spacing));
                    state.kernel.validate().unwrap();
                    assert!((2..=24).contains(&state.shrink));
                    assert!((12..=15).contains(&state.margin));
                    assert!((4.0..=5.0).contains(&state.reduced_sigma));
                }
                Err(_) => assert!(!(2..=32).contains(&spacing)),
            }
        }
    }
}
