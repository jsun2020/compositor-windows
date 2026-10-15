//! Source RGBA8 reduction and final half-texture enlargement.
//! Reduction follows the independently verified 69-plane model. It has four
//! retained half-element differences against the composed MPS inputs; those
//! disappear in the Gaussian final planes for the 31 original spacing cases.
//! Do not describe this as a proof of every internal Core Image texture.
use super::{
    area, for_spacing, gaussian, half_float, half_rtz, half_up, reserved, sampling, Error, Format,
    Plane, Storage,
};

const LOD: [u8; 25] = [
    0, 0, 0, 1, 1, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3, 3, 3, 4, 4, 4, 4, 4, 4,
];

impl Plane {
    fn half_at(&self, x: i32, y: i32) -> [u16; 4] {
        let x = x.clamp(0, self.width as i32 - 1) as usize;
        let y = y.clamp(0, self.height as i32 - 1) as usize;
        match &self.storage {
            Storage::Half(v) => v[y * self.width + x],
            Storage::Float(_) => unreachable!("Scanlines resizing requires a half plane"),
        }
    }
    fn quad(&self, x: i32, y: i32) -> [[u16; 4]; 4] {
        [
            self.half_at(x, y),
            self.half_at(x + 1, y),
            self.half_at(x, y + 1),
            self.half_at(x + 1, y + 1),
        ]
    }
}

fn byte_quad(source: &[u8], w: usize, h: usize, x: i32, y: i32) -> [f32; 4] {
    let mut sum = [0_u16; 4];
    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
        let xx = (x + dx).clamp(0, w as i32 - 1) as usize;
        let yy = (y + dy).clamp(0, h as i32 - 1) as usize;
        for c in 0..4 {
            sum[c] += u16::from(source[(yy * w + xx) * 4 + c]);
        }
    }
    sum.map(|v| f32::from(v) / 1020.)
}

fn fraction(value: f64) -> (i32, u16) {
    let floor = value.floor();
    (
        floor as i32,
        ((value - floor) * 256.).round_ties_even() as u16,
    )
}

/// The reduction kernel's exact weighted-sum model, not the final texture
/// sampler. Substituting the texture sampler here fails independent controls.
fn reduce_sample(q: [[u16; 4]; 4], x: u16, y: u16) -> [f32; 4] {
    let x = f64::from(x) / 256.;
    let y = f64::from(y) / 256.;
    std::array::from_fn(|c| {
        let [a, b, cc, d] = q.map(|p| f64::from(half_float(p[c])));
        half_float(half_up(
            (a * (1. - x) + b * x) * (1. - y) + (cc * (1. - x) + d * x) * y,
        ))
    })
}

pub(crate) fn reduce(
    source: &[u8],
    w: usize,
    h: usize,
    shrink: usize,
    margin: i32,
    output_format: Format,
) -> Result<Plane, Error> {
    if w == 0
        || h == 0
        || w > 30_000
        || h > 30_000
        || w.checked_mul(h).filter(|&n| n <= 100_000_000).is_none()
        || source.len() != w * h * 4
        || !(2..=24).contains(&shrink)
        || !(0..=32).contains(&margin)
    {
        return Err("Invalid Scanlines reduction input");
    }
    let rw = w.div_ceil(shrink);
    let rh = h.div_ceil(shrink);
    let mw = rw + 2 * margin as usize;
    let mh = rh + 2 * margin as usize;
    area(mw, mh)?;
    let mut output = Plane::empty(mw, mh, output_format)?;
    let row_phase = h as i32 - (shrink * rh) as i32;
    if shrink == 2 {
        for y in 0..mh {
            for x in 0..mw {
                output.push(byte_quad(
                    source,
                    w,
                    h,
                    2 * (x as i32 - margin),
                    2 * (y as i32 - margin) + row_phase,
                ))?;
            }
        }
        return Ok(output);
    }
    let level = usize::from(LOD[shrink]);
    // One explicit edge cell on each side lets later global mip cells clamp
    // to original edge pixels, rather than a finite grid's averaged edge cell.
    let mut mip = Plane::empty(w.div_ceil(2) + 2, h.div_ceil(2) + 2, Format::Half)?;
    for y in 0..mip.height {
        for x in 0..mip.width {
            mip.push(byte_quad(
                source,
                w,
                h,
                2 * (x as i32 - 1),
                2 * (y as i32 - 1),
            ))?;
        }
    }
    for l in 2..=level {
        let scale = 1 << l;
        let mut next = Plane::empty(w.div_ceil(scale) + 2, h.div_ceil(scale) + 2, Format::Half)?;
        for y in 0..next.height {
            for x in 0..next.width {
                let q = mip.quad(2 * x as i32 - 1, 2 * y as i32 - 1);
                let p = std::array::from_fn(|c| {
                    half_float(half_up(
                        q.iter().map(|p| f64::from(half_float(p[c]))).sum::<f64>() * 0.25,
                    ))
                });
                next.push(p)?;
            }
        }
        mip = next;
    }
    let base = (1 << level) as f64;
    for y in 0..mh {
        let yy = ((y as f64 - f64::from(margin) + 0.5) * shrink as f64 + f64::from(row_phase))
            / base
            - 0.5
            + 1.;
        let (iy, fy) = fraction(yy);
        for x in 0..mw {
            let xx = (x as f64 - f64::from(margin) + 0.5) * shrink as f64 / base - 0.5 + 1.;
            let (ix, fx) = fraction(xx);
            output.push(reduce_sample(mip.quad(ix, iy), fx, fy))?;
        }
    }
    Ok(output)
}

pub(crate) fn bloom(source: &[u8], w: usize, h: usize, spacing: u8) -> Result<Vec<u8>, Error> {
    let state = for_spacing(spacing)?;
    let reduced = reduce(source, w, h, state.shrink, state.margin, Format::Half)?;
    let rw = w.div_ceil(state.shrink);
    let rh = h.div_ceil(state.shrink);
    let blurred = gaussian(
        &reduced,
        rw,
        rh,
        [state.margin; 2],
        &state.kernel,
        Format::Half,
        |_, _, _| Ok(()),
    )?;
    drop(reduced);
    let mut output = reserved(w * h * 4)?;
    for y in 0..h {
        let yy = ((y as f64 + 0.5) / state.shrink as f64 - 0.5 + rh as f64
            - h as f64 / state.shrink as f64)
            .clamp(0., rh as f64 - 1.);
        let (iy, fy) = fraction(yy);
        for x in 0..w {
            let xx = ((x as f64 + 0.5) / state.shrink as f64 - 0.5).clamp(0., rw as f64 - 1.);
            let (ix, fx) = fraction(xx);
            let p = sampling::quad_float(blurred.quad(ix, iy), fx, fy);
            for c in p {
                output.push((f64::from(c) * 255. + 0.5).floor().clamp(0., 255.) as u8);
            }
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reduction_keeps_infinite_original_edges_and_odd_row_phase() {
        let source = vec![0, 0, 0, 0, 255, 128, 64, 255, 64, 32, 16, 128];
        let p = reduce(&source, 1, 3, 2, 1, Format::Half).unwrap();
        assert_eq!(p.sample(0, 0), [0.; 4]);
        let average = [127.5_f32, 64., 32., 127.5].map(|v| half_float(half_rtz(v / 255.)));
        assert_eq!(
            p.sample(1, 2),
            [159.5, 80., 40., 191.5].map(|v| half_float(half_rtz(v / 255.)))
        );
        assert_ne!(p.sample(1, 1), average);
    }
    #[test]
    fn thin_and_transparent_bloom_is_bounded() {
        for (w, h) in [(1, 1), (1, 3), (3, 1), (33, 31)] {
            for spacing in 2..=32 {
                assert_eq!(
                    bloom(&vec![0; w * h * 4], w, h, spacing).unwrap(),
                    vec![0; w * h * 4]
                );
            }
        }
        assert!(bloom(&[], 0, 1, 2).is_err());
        assert!(bloom(&[0; 4], 1, 1, 1).is_err());
    }
}
