//! Apply a spatially varying light field to the original layer, retaining its
//! resolution and precision. Each field sample holds fog, RGB scattered light
//! and RGB beam transmission. Mac PixelAdjust and DeepLayer render the kernel
//! with color management disabled. Apply it directly to stored color samples;
//! only the user-selected lamp tint is linearized when the field is prepared.
use crate::*;

pub(crate) fn apply(
    source: &Raster,
    coverage: Option<&GrayRaster>,
    width: u32,
    height: u32,
    field: &[f32],
) -> Result<Raster, CommandError> {
    if width == 0
        || height == 0
        || width > 1280
        || height > 1280
        || field.len() != width as usize * height as usize * 7
        || field.chunks_exact(7).any(|p| {
            !(0.0..=1.0).contains(&p[0])
                || p[1..4].iter().any(|v| !v.is_finite() || *v < 0.)
                || p[4..].iter().any(|v| !(0.0..=1.0).contains(v))
        })
    {
        return Err(CommandError::Argument("Invalid lighting field".into()));
    }
    fn axis(at: u32, target: u32, from: u32) -> (usize, usize, f32) {
        let p =
            ((at as f64 + 0.5) * from as f64 / target as f64 - 0.5).clamp(0., (from - 1) as f64);
        let lo = p.floor() as usize;
        (lo, (lo + 1).min(from as usize - 1), p.fract() as f32)
    }
    let xs: Vec<_> = (0..source.width)
        .map(|x| axis(x, source.width, width))
        .collect();
    let ys: Vec<_> = (0..source.height)
        .map(|y| axis(y, source.height, height))
        .collect();
    let pixel = |x: u32, y: u32| {
        let mut original = source.pixel_f32(x, y);
        let a = original[3];
        let mix = coverage.map_or(1., |c| {
            c.bytes()[(y * source.width + x) as usize] as f32 / 255.
        });
        if a == 0. || mix == 0. {
            return original;
        }
        let (x0, x1, fx) = xs[x as usize];
        let (y0, y1, fy) = ys[y as usize];
        let sample = |c| {
            let row = |y: usize| {
                field[(y * width as usize + x0) * 7 + c] * (1. - fx)
                    + field[(y * width as usize + x1) * 7 + c] * fx
            };
            row(y0) * (1. - fy) + row(y1) * fy
        };
        let fog = sample(0usize);
        for (c, v) in original[..3].iter_mut().enumerate() {
            let scatter = sample(c + 1);
            let transmission = sample(c + 4);
            if fog == 0. && scatter == 0. && transmission == 1. {
                continue;
            }
            let base = *v / a;
            let veiled = base * (1. - fog) + scatter;
            let lit = if veiled >= 1. {
                veiled
            } else {
                1. - (1. - veiled) * transmission
            };
            *v += mix * (a * lit - *v);
        }
        original
    };
    if source.depth() != PixelDepth::Eight {
        return Ok(Raster::from_deep_fn(
            source.width,
            source.height,
            source.depth(),
            pixel,
        ));
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(source.bytes().len())
        .map_err(|_| ProjectError::OverBudget)?;
    for y in 0..source.height {
        for x in 0..source.width {
            bytes.extend(pixel(x, y).map(|v| (v.clamp(0., 1.) * 255.).round() as u8));
        }
    }
    Ok(Raster::from_premultiplied(
        source.width,
        source.height,
        bytes,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_keeps_deep_samples_alpha_and_selection() {
        let src = Raster::from_premultiplied_f32(
            3,
            1,
            vec![
                0.225, 0.132, 0.05, 0.5, 0.2251, 0.132, 0.05, 0.5, 0.225, 0.132, 0.05, 0.5,
            ],
            PixelDepth::Sixteen,
        );
        let mask = GrayRaster::from_bytes(3, 1, vec![255, 255, 0]);
        let out = apply(&src, Some(&mask), 1, 1, &[0., 0.025, 0., 0., 1., 1., 1.]).unwrap();
        assert_eq!(out.depth(), PixelDepth::Sixteen);
        assert!(out.pixel_f32(0, 0)[0] > src.pixel_f32(0, 0)[0]);
        assert!(out.pixel_f32(1, 0)[0] > out.pixel_f32(0, 0)[0]);
        assert_eq!(out.pixel_f32(1, 0)[3], src.pixel_f32(1, 0)[3]);
        assert_eq!(out.pixel_f32(2, 0), src.pixel_f32(2, 0));
    }
}
