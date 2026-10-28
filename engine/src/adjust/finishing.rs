//! Mac 1.5.11 finishing filters and their project-format settings.
use crate::{AdjustmentColor, Raster, Rect};
use serde::{Deserialize, Serialize};

fn clamp(v: f64, lo: f64, hi: f64, fallback: f64) -> f64 {
    if v.is_finite() { v.clamp(lo, hi) } else { fallback }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct VignetteLayerSettings {
    pub amount: f64,
    pub color: AdjustmentColor,
    pub midpoint: f64,
    pub roundness: f64,
    pub feather: f64,
    pub highlights: f64,
}
impl Default for VignetteLayerSettings {
    fn default() -> Self { Self { amount: 35., color: AdjustmentColor { red: 0., green: 0., blue: 0. }, midpoint: 50., roundness: 100., feather: 60., highlights: 25. } }
}
impl VignetteLayerSettings {
    pub fn normalized(&self) -> Self {
        Self { amount: clamp(self.amount,0.,100.,35.), color: self.color.clamped(), midpoint: clamp(self.midpoint,0.,100.,50.), roundness: clamp(self.roundness,-100.,100.,100.), feather: clamp(self.feather,0.,100.,60.), highlights: clamp(self.highlights,0.,100.,25.) }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TonalLayerSettings {
    pub amount: f64, pub radius: f64, pub shadows: f64, pub midtones: f64, pub highlights: f64,
}
impl Default for TonalLayerSettings {
    fn default() -> Self { Self { amount: 50., radius: 16., shadows: 40., midtones: 60., highlights: 30. } }
}
impl TonalLayerSettings {
    pub fn normalized(&self) -> Self {
        Self { amount: clamp(self.amount,0.,100.,50.), radius: clamp(self.radius,1.,100.,16.), shadows: clamp(self.shadows,-100.,100.,40.), midtones: clamp(self.midtones,-100.,100.,60.), highlights: clamp(self.highlights,-100.,100.,30.) }
    }
    pub fn is_identity(&self) -> bool { self.amount == 0. || (self.shadows == 0. && self.midtones == 0. && self.highlights == 0.) }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SharpenLayerSettings { pub amount: f64, pub radius: f64 }
impl Default for SharpenLayerSettings {
    fn default() -> Self { Self { amount: 100., radius: 1. } }
}
impl SharpenLayerSettings {
    pub fn normalized(&self) -> Self { Self { amount: clamp(self.amount,0.,300.,100.), radius: clamp(self.radius,0.1,20.,1.) } }
}

extern "C" {
    fn adjust_colored_vignette(rgba: *mut u8, width: usize, height: usize, stride: usize,
        frame_x: f64, frame_y: f64, frame_width: f64, frame_height: f64, fills_clear: i32,
        amount: f64, midpoint: f64, roundness: f64, feather: f64, highlights: f64,
        red: f64, green: f64, blue: f64);
    fn adjust_tonal_contrast(rgba: *mut u8, blurred: *const u8, width: usize, height: usize,
        stride: usize, blurred_stride: usize, amount: f64, shadows: f64, midtones: f64, highlights: f64);
}

/// `frame` is the whole canvas expressed in this raster's pixel coordinates.
/// Adjustment layers always preserve transparency; only a destructive vignette
/// on an originally empty layer may paint clear pixels.
pub fn vignette(r: &Raster, s: &VignetteLayerSettings, frame: Rect, fills_clear: bool) -> Raster {
    if s.amount == 0. { return r.clone(); }
    if r.deep_pixels().is_some() {
        return r.map_deep::<std::convert::Infallible>(true, |plane| Ok(vignette(plane,s,frame,fills_clear))).unwrap();
    }
    let mut data = r.bytes().to_vec();
    // SAFETY: both dimensions and the tightly packed stride describe `data`.
    unsafe { adjust_colored_vignette(data.as_mut_ptr(),r.width as usize,r.height as usize,r.width as usize*4,
        frame.x,frame.y,frame.width,frame.height,i32::from(fills_clear),s.amount,s.midpoint,s.roundness,s.feather,s.highlights,s.color.red,s.color.green,s.color.blue); }
    Raster::from_premultiplied(r.width,r.height,data)
}

pub fn tonal(r: &Raster, s: &TonalLayerSettings) -> Raster {
    if s.is_identity() { return r.clone(); }
    if r.deep_pixels().is_some() {
        return r.map_deep::<std::convert::Infallible>(true, |plane| Ok(tonal(plane,s))).unwrap();
    }
    let blurred = super::filters::gaussian_blur(r,s.radius);
    tonal_with_base(r, &blurred, s)
}

fn tonal_with_base(r: &Raster, blurred: &Raster, s: &TonalLayerSettings) -> Raster {
    assert_eq!((r.width, r.height), (blurred.width, blurred.height));
    let mut data = r.bytes().to_vec();
    // SAFETY: source and blurred buffers have identical tightly packed dimensions.
    unsafe { adjust_tonal_contrast(data.as_mut_ptr(),blurred.bytes().as_ptr(),r.width as usize,r.height as usize,
        r.width as usize*4,r.width as usize*4,s.amount,s.shadows,s.midtones,s.highlights); }
    Raster::from_premultiplied(r.width,r.height,data)
}

/// Unsharp mask of straight sRGB colors; the blurred alpha is normalized back
/// to the source alpha before subtracting, so alpha edges cannot create halos.
pub fn sharpen(r: &Raster, amount: f64, radius: f64, sides: super::filters::CanvasSides) -> Raster {
    if amount == 0. { return r.clone(); }
    let blurred = super::deep_filters::gaussian_edges(r,radius,true,sides);
    let gain = (amount / 100.) as f32;
    let mut values = Vec::with_capacity(r.width as usize*r.height as usize*4);
    for y in 0..r.height { for x in 0..r.width {
        let p = r.pixel_f32(x,y); let b = blurred.pixel_f32(x,y);
        let limit = if r.depth() == crate::PixelDepth::Hdr {65504.} else {p[3]};
        values.extend([0,1,2].map(|c| (p[c]+(p[c]-b[c])*gain).clamp(0.,limit)));
        values.push(p[3]);
    }}
    if r.depth()==crate::PixelDepth::Eight {
        Raster::from_premultiplied(r.width,r.height,values.into_iter().map(|v|(v*255.).round().clamp(0.,255.)as u8).collect())
    } else { Raster::from_premultiplied_f32(r.width,r.height,values,r.depth()) }
}

#[cfg(test)]
mod native_reference {
    use super::*;
    #[test]
    fn tonal_kernel_is_byte_exact_when_given_the_same_core_image_blur() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mac1511");
        let source = Raster::from_premultiplied(96, 64, std::fs::read(root.join("lens-input.rgba")).unwrap());
        for radius in [1., 16., 100.] {
            let base = Raster::from_premultiplied(96, 64, std::fs::read(root.join(format!("tonal-blurred-{}.rgba", radius as i32))).unwrap());
            for amount in [50., 100.] {
                let expected = std::fs::read(root.join(format!("tonal-{}-{}.rgba", radius as i32, amount as i32))).unwrap();
                let actual = tonal_with_base(&source, &base, &TonalLayerSettings { amount, radius, ..Default::default() });
                assert_eq!(actual.bytes(), expected);
            }
        }
    }
}
