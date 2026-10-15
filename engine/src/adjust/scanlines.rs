//! Compositor v1.4.9 Scanlines.swift and DitherPixels.c (MIT, 0ab3ff5).
use crate::{CommandError, Raster};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
pub enum ScanlineColors {
    #[default]
    #[serde(rename = "Black & White")]
    BlackWhite,
    Original,
    #[serde(rename = "Two Colors")]
    TwoColors,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct ScanlinesSettings {
    pub line_spacing: f64,
    pub thickness: f64,
    pub glow: f64,
    pub dots: f64,
    pub wobble: f64,
    pub displace: f64,
    pub smoothness: f64,
    pub threshold: f64,
    pub split: f64,
    pub density: f64,
    pub contrast: f64,
    pub black_level: f64,
    pub colors: ScanlineColors,
    pub dark: [f64; 3],
    pub light: [f64; 3],
}
impl Default for ScanlinesSettings {
    fn default() -> Self {
        Self {
            line_spacing: 4.,
            thickness: 70.,
            glow: 0.,
            dots: 0.,
            wobble: 0.,
            displace: 0.,
            smoothness: 50.,
            threshold: 0.,
            split: 0.,
            density: 0.,
            contrast: 0.,
            black_level: 0.,
            colors: ScanlineColors::BlackWhite,
            dark: [0.; 3],
            light: [1.; 3],
        }
    }
}
#[repr(C)]
struct Params {
    spacing: i32,
    thickness: f32,
    dots: f32,
    wobble: f32,
    displace: f32,
    threshold: f32,
    split: f32,
    density: f32,
    contrast: f32,
    black_level: f32,
    smoothness: f32,
    original_colors: i32,
    dark: [u8; 3],
    light: [u8; 3],
}
extern "C" {
    fn scanlines_apply(p: *mut u8, w: usize, h: usize, stride: usize, params: *const Params)
        -> i32;
    fn dither_glow(p: *mut u8, light: *const u8, w: usize, h: usize, stride: usize, amount: f32);
}
impl ScanlinesSettings {
    pub fn normalized(&self) -> Self {
        let mut s = self.clone();
        let d = Self::default();
        for (v, lo, hi, fallback) in [
            (&mut s.line_spacing, 2., 32., d.line_spacing),
            (&mut s.thickness, 5., 100., d.thickness),
            (&mut s.glow, 0., 100., 0.),
            (&mut s.dots, 0., 100., 0.),
            (&mut s.wobble, 0., 64., 0.),
            (&mut s.displace, -100., 100., 0.),
            (&mut s.smoothness, 0., 100., 50.),
            (&mut s.threshold, 0., 100., 0.),
            (&mut s.split, 0., 16., 0.),
            (&mut s.density, -100., 100., 0.),
            (&mut s.contrast, -100., 100., 0.),
            (&mut s.black_level, 0., 100., 0.),
        ] {
            *v = if v.is_finite() {
                v.clamp(lo, hi)
            } else {
                fallback
            };
        }
        s.line_spacing = s.line_spacing.round();
        s.split = s.split.round();
        for (color, fallback) in [(&mut s.dark, 0.), (&mut s.light, 1.)] {
            for v in color {
                *v = if v.is_finite() {
                    v.clamp(0., 1.)
                } else {
                    fallback
                };
            }
        }
        s
    }
    pub fn scaled(&self, factor: f64) -> Self {
        let mut s = self.clone();
        s.line_spacing *= factor;
        s.wobble *= factor;
        s.displace *= factor;
        s.split *= factor;
        s
    }
    pub fn apply(&self, source: &Raster) -> Result<Raster, CommandError> {
        let s = self.normalized();
        let mut bytes = source.bytes().to_vec();
        let w = source.width as usize;
        let h = source.height as usize;
        let two = s.colors == ScanlineColors::TwoColors;
        let p = Params {
            spacing: s.line_spacing as i32,
            thickness: (s.thickness / 100.) as f32,
            dots: (s.dots / 100.) as f32,
            wobble: s.wobble as f32,
            displace: s.displace as f32,
            threshold: (s.threshold / 100.) as f32,
            split: s.split as f32,
            density: (s.density / 100.) as f32,
            contrast: (s.contrast / 100.) as f32,
            black_level: (s.black_level / 100.) as f32,
            smoothness: (s.smoothness / 100.) as f32,
            original_colors: (s.colors == ScanlineColors::Original) as i32,
            dark: if two {
                s.dark.map(|v| (v * 255.).round() as u8)
            } else {
                [0; 3]
            },
            light: if two {
                s.light.map(|v| (v * 255.).round() as u8)
            } else {
                [255; 3]
            },
        };
        super::camera_raw_runtime::begin();
        let okay = unsafe { scanlines_apply(bytes.as_mut_ptr(), w, h, w * 4, &p) };
        if okay == 0 || super::camera_raw_runtime::failed() {
            return Err(CommandError::Refused(
                "Scanlines working memory limit exceeded".into(),
            ));
        }
        if s.glow > 0. {
            let light = super::scanlines_glow::bloom(&bytes, w, h, s.line_spacing as u8)
                .map_err(|e| CommandError::Refused(e.into()))?;
            unsafe {
                dither_glow(
                    bytes.as_mut_ptr(),
                    light.as_ptr(),
                    w,
                    h,
                    w * 4,
                    (s.glow / 100. * 2.5) as f32,
                );
            }
        }
        Ok(Raster::from_premultiplied(
            source.width,
            source.height,
            bytes,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{dither_glow, ScanlineColors, ScanlinesSettings};
    use crate::Raster;

    #[test]
    fn glow_composite_preserves_mac_fused_rounding_at_bright_edge() {
        // Actual Mac 1.4.9 composite return, spacing 10/15. Bloom is supplied
        // independently here so this catches compositing precision only.
        for (mut drawn, light, expected) in [
            (
                [250, 151, 196, 255],
                [57, 58, 58, 234],
                [253, 185, 224, 255],
            ),
            (
                [250, 159, 193, 255],
                [57, 57, 56, 206],
                [253, 192, 221, 255],
            ),
        ] {
            unsafe { dither_glow(drawn.as_mut_ptr(), light.as_ptr(), 1, 1, 4, 0.8) };
            assert_eq!(drawn, expected);
        }
    }

    #[test]
    fn scanline_beam_preserves_mac_fused_rounding_at_coverage_edge() {
        // Independent columns from the synthetic mac149_probes generator.
        // Glow is off; these Mac-returned values isolate beam coverage.
        for (spacing, x, top, row, expected) in [
            (24_u32, 101_u32, 72_u32, 18_usize, [35, 36, 35, 71]),
            (29, 94, 29, 22, [101, 92, 89, 213]),
        ] {
            let mut straight = Vec::new();
            for y in top..top + spacing {
                let alpha = if y < 48 {
                    255
                } else {
                    ((x * 17 + y * 13) % 256) as u8
                };
                straight.extend([
                    ((x * 31 + y * 7) % 256) as u8,
                    ((x * 3 + y * 41) % 256) as u8,
                    ((x * 19 + y * 23) % 256) as u8,
                    alpha,
                ]);
            }
            let source = Raster::from_straight(1, spacing, &straight);
            let settings = ScanlinesSettings {
                line_spacing: spacing as f64,
                colors: ScanlineColors::Original,
                ..Default::default()
            };
            let drawn = settings.apply(&source).unwrap();
            assert_eq!(&drawn.bytes()[row * 4..row * 4 + 4], &expected);
        }
    }
}
