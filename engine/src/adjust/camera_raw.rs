//! Camera Raw settings and pixel pipeline from Compositor v1.4.5.
//! The C algorithm bodies are preserved; Swift settings/curve orchestration is ported here.
use super::settings::{CurvePoint, CurvesSettings};
use crate::{CommandError, Raster};
use serde::{Deserialize, Serialize};
fn clamp(n: f64, lo: f64, hi: f64, fallback: f64) -> f64 {
    if n.is_finite() {
        n.clamp(lo, hi)
    } else {
        fallback
    }
}
fn linear() -> Vec<CurvePoint> {
    vec![CurvePoint { x: 0.0, y: 0.0 }, CurvePoint { x: 1.0, y: 1.0 }]
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WhiteBalance {
    #[serde(rename = "Custom")]
    Custom,
    #[serde(rename = "Auto")]
    Auto,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GlowStyle {
    #[serde(rename = "Diffusion")]
    Diffusion,
    #[serde(rename = "Bloom")]
    Bloom,
    #[serde(rename = "Halation")]
    Halation,
}
impl GlowStyle {
    fn kernel(self) -> i32 {
        match self {
            Self::Diffusion => 0,
            Self::Bloom => 1,
            Self::Halation => 2,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VignetteStyle {
    #[serde(rename = "Highlight Priority")]
    HighlightPriority,
    #[serde(rename = "Color Priority")]
    ColorPriority,
    #[serde(rename = "Paint Overlay")]
    PaintOverlay,
}
impl VignetteStyle {
    fn kernel(self) -> i32 {
        match self {
            Self::HighlightPriority => 0,
            Self::ColorPriority => 1,
            Self::PaintOverlay => 2,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessVersion {
    #[serde(rename = "Version 1")]
    Version1,
    #[serde(rename = "Version 2")]
    Version2,
    #[serde(rename = "Version 3")]
    Version3,
    #[serde(rename = "Version 4")]
    Version4,
    #[serde(rename = "Version 5")]
    Version5,
    #[serde(rename = "Version 6")]
    Version6,
}
impl ProcessVersion {
    fn kernel(self) -> i32 {
        match self {
            Self::Version1 => 1,
            Self::Version2 => 2,
            Self::Version3 => 3,
            Self::Version4 => 4,
            Self::Version5 => 5,
            Self::Version6 => 6,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UprightMode {
    #[serde(rename = "Off")]
    Off,
    #[serde(rename = "Guided")]
    Guided,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Projection {
    #[serde(rename = "Perspective")]
    Perspective,
    #[serde(rename = "Rectilinear")]
    Rectilinear,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct GeometryGuide {
    pub start_x: f64,
    pub start_y: f64,
    pub end_x: f64,
    pub end_y: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct CameraRawSettings {
    pub temperature: f64,
    pub tint: f64,
    pub exposure: f64,
    pub contrast: f64,
    pub highlights: f64,
    pub shadows: f64,
    pub whites: f64,
    pub blacks: f64,
    pub vibrance: f64,
    pub saturation: f64,
    pub texture: f64,
    pub clarity: f64,
    pub dehaze: f64,
    pub glow: f64,
    pub glow_range: f64,
    pub glow_spread: f64,
    pub glow_warmth: f64,
    pub vignette_amount: f64,
    pub vignette_midpoint: f64,
    pub vignette_roundness: f64,
    pub vignette_feather: f64,
    pub vignette_highlights: f64,
    pub grain_amount: f64,
    pub grain_size: f64,
    pub grain_roughness: f64,
    pub white_balance: WhiteBalance,
    pub glow_style: GlowStyle,
    pub vignette_style: VignetteStyle,
    pub curve: CameraRawCurve,
    pub mixer: CameraRawMixer,
    pub grading: CameraRawGrading,
    pub detail: CameraRawDetail,
    pub optics: CameraRawOptics,
    pub geometry: CameraRawGeometry,
    pub calibration: CameraRawCalibration,
    pub seed: u32,
    pub pixel_scale: f64,
}
impl Default for CameraRawSettings {
    fn default() -> Self {
        Self {
            temperature: 0.0,
            tint: 0.0,
            exposure: 0.0,
            contrast: 0.0,
            highlights: 0.0,
            shadows: 0.0,
            whites: 0.0,
            blacks: 0.0,
            vibrance: 0.0,
            saturation: 0.0,
            texture: 0.0,
            clarity: 0.0,
            dehaze: 0.0,
            glow: 0.0,
            glow_range: 0.0,
            glow_spread: 0.0,
            glow_warmth: 0.0,
            vignette_amount: 0.0,
            vignette_midpoint: 50.0,
            vignette_roundness: 0.0,
            vignette_feather: 50.0,
            vignette_highlights: 0.0,
            grain_amount: 0.0,
            grain_size: 25.0,
            grain_roughness: 50.0,
            white_balance: WhiteBalance::Custom,
            glow_style: GlowStyle::Diffusion,
            vignette_style: VignetteStyle::HighlightPriority,
            curve: CameraRawCurve::default(),
            mixer: CameraRawMixer::default(),
            grading: CameraRawGrading::default(),
            detail: CameraRawDetail::default(),
            optics: CameraRawOptics::default(),
            geometry: CameraRawGeometry::default(),
            calibration: CameraRawCalibration::default(),
            seed: 0,
            pixel_scale: 1.0,
        }
    }
}
impl CameraRawSettings {
    pub fn normalized(&self) -> Self {
        let mut s = self.clone();
        s.temperature = clamp(s.temperature, -100.0, 100.0, 0.0);
        s.tint = clamp(s.tint, -100.0, 100.0, 0.0);
        s.exposure = clamp(s.exposure, -5.0, 5.0, 0.0);
        s.contrast = clamp(s.contrast, -100.0, 100.0, 0.0);
        s.highlights = clamp(s.highlights, -100.0, 100.0, 0.0);
        s.shadows = clamp(s.shadows, -100.0, 100.0, 0.0);
        s.whites = clamp(s.whites, -100.0, 100.0, 0.0);
        s.blacks = clamp(s.blacks, -100.0, 100.0, 0.0);
        s.vibrance = clamp(s.vibrance, -100.0, 100.0, 0.0);
        s.saturation = clamp(s.saturation, -100.0, 100.0, 0.0);
        s.texture = clamp(s.texture, -100.0, 100.0, 0.0);
        s.clarity = clamp(s.clarity, -100.0, 100.0, 0.0);
        s.dehaze = clamp(s.dehaze, -100.0, 100.0, 0.0);
        s.glow = clamp(s.glow, 0.0, 100.0, 0.0);
        s.glow_range = clamp(s.glow_range, -100.0, 100.0, 0.0);
        s.glow_spread = clamp(s.glow_spread, -100.0, 100.0, 0.0);
        s.glow_warmth = clamp(s.glow_warmth, -100.0, 100.0, 0.0);
        s.vignette_amount = clamp(s.vignette_amount, -100.0, 100.0, 0.0);
        s.vignette_midpoint = clamp(s.vignette_midpoint, 0.0, 100.0, 50.0);
        s.vignette_roundness = clamp(s.vignette_roundness, -100.0, 100.0, 0.0);
        s.vignette_feather = clamp(s.vignette_feather, 0.0, 100.0, 50.0);
        s.vignette_highlights = clamp(s.vignette_highlights, 0.0, 100.0, 0.0);
        s.grain_amount = clamp(s.grain_amount, 0.0, 100.0, 0.0);
        s.grain_size = clamp(s.grain_size, 0.0, 100.0, 25.0);
        s.grain_roughness = clamp(s.grain_roughness, 0.0, 100.0, 50.0);
        s.pixel_scale = clamp(s.pixel_scale, 0.000001, 1.0, 1.0);
        s.curve = s.curve.normalized();
        s.mixer = s.mixer.normalized();
        s.grading = s.grading.normalized();
        s.detail = s.detail.normalized();
        s.optics = s.optics.normalized();
        s.geometry = s.geometry.normalized();
        s.calibration = s.calibration.normalized();
        s
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct CameraRawCurve {
    pub shadows: f64,
    pub darks: f64,
    pub lights: f64,
    pub highlights: f64,
    pub shadow_split: f64,
    pub dark_split: f64,
    pub light_split: f64,
    pub refine_saturation: f64,
    pub rgb: Vec<CurvePoint>,
    pub red: Vec<CurvePoint>,
    pub green: Vec<CurvePoint>,
    pub blue: Vec<CurvePoint>,
}
impl Default for CameraRawCurve {
    fn default() -> Self {
        Self {
            shadows: 0.0,
            darks: 0.0,
            lights: 0.0,
            highlights: 0.0,
            shadow_split: 25.0,
            dark_split: 50.0,
            light_split: 75.0,
            refine_saturation: 0.0,
            rgb: linear(),
            red: linear(),
            green: linear(),
            blue: linear(),
        }
    }
}
impl CameraRawCurve {
    pub fn normalized(&self) -> Self {
        let mut s = self.clone();
        s.shadows = clamp(s.shadows, -100.0, 100.0, 0.0);
        s.darks = clamp(s.darks, -100.0, 100.0, 0.0);
        s.lights = clamp(s.lights, -100.0, 100.0, 0.0);
        s.highlights = clamp(s.highlights, -100.0, 100.0, 0.0);
        s.shadow_split = clamp(s.shadow_split, 5.0, 90.0, 25.0);
        s.dark_split = clamp(s.dark_split, 7.0, 95.0, 50.0);
        s.light_split = clamp(s.light_split, 9.0, 98.0, 75.0);
        s.refine_saturation = clamp(s.refine_saturation, -100.0, 100.0, 0.0);
        s.dark_split = clamp(s.dark_split, s.shadow_split + 2.0, 95.0, 50.0);
        s.light_split = clamp(s.light_split, s.dark_split + 2.0, 98.0, 75.0);
        s.rgb = repair(&s.rgb);
        s.red = repair(&s.red);
        s.green = repair(&s.green);
        s.blue = repair(&s.blue);
        s
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct CameraRawDetail {
    pub sharpen_amount: f64,
    pub sharpen_radius: f64,
    pub sharpen_detail: f64,
    pub sharpen_masking: f64,
    pub noise_luminance: f64,
    pub noise_luminance_detail: f64,
    pub noise_luminance_contrast: f64,
    pub noise_color: f64,
    pub noise_color_detail: f64,
    pub noise_color_smoothness: f64,
}
impl Default for CameraRawDetail {
    fn default() -> Self {
        Self {
            sharpen_amount: 0.0,
            sharpen_radius: 10.0,
            sharpen_detail: 25.0,
            sharpen_masking: 0.0,
            noise_luminance: 0.0,
            noise_luminance_detail: 50.0,
            noise_luminance_contrast: 0.0,
            noise_color: 0.0,
            noise_color_detail: 50.0,
            noise_color_smoothness: 50.0,
        }
    }
}
impl CameraRawDetail {
    pub fn normalized(&self) -> Self {
        let mut s = self.clone();
        s.sharpen_amount = clamp(s.sharpen_amount, 0.0, 150.0, 0.0);
        s.sharpen_radius = clamp(s.sharpen_radius, 0.0, 100.0, 10.0);
        s.sharpen_detail = clamp(s.sharpen_detail, 0.0, 100.0, 25.0);
        s.sharpen_masking = clamp(s.sharpen_masking, 0.0, 100.0, 0.0);
        s.noise_luminance = clamp(s.noise_luminance, 0.0, 100.0, 0.0);
        s.noise_luminance_detail = clamp(s.noise_luminance_detail, 0.0, 100.0, 50.0);
        s.noise_luminance_contrast = clamp(s.noise_luminance_contrast, 0.0, 100.0, 0.0);
        s.noise_color = clamp(s.noise_color, 0.0, 100.0, 0.0);
        s.noise_color_detail = clamp(s.noise_color_detail, 0.0, 100.0, 50.0);
        s.noise_color_smoothness = clamp(s.noise_color_smoothness, 0.0, 100.0, 50.0);
        s
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct CameraRawOptics {
    pub profile_distortion: f64,
    pub profile_vignetting: f64,
    pub distortion: f64,
    pub purple_amount: f64,
    pub purple_hue_low: f64,
    pub purple_hue_high: f64,
    pub green_amount: f64,
    pub green_hue_low: f64,
    pub green_hue_high: f64,
    pub vignette_amount: f64,
    pub vignette_midpoint: f64,
    pub remove_chromatic_aberration: bool,
    pub enable_lens_profile: bool,
}
impl Default for CameraRawOptics {
    fn default() -> Self {
        Self {
            profile_distortion: 100.0,
            profile_vignetting: 100.0,
            distortion: 0.0,
            purple_amount: 0.0,
            purple_hue_low: 270.0,
            purple_hue_high: 310.0,
            green_amount: 0.0,
            green_hue_low: 60.0,
            green_hue_high: 120.0,
            vignette_amount: 0.0,
            vignette_midpoint: 50.0,
            remove_chromatic_aberration: false,
            enable_lens_profile: false,
        }
    }
}
impl CameraRawOptics {
    pub fn normalized(&self) -> Self {
        let mut s = self.clone();
        s.profile_distortion = clamp(s.profile_distortion, 0.0, 100.0, 100.0);
        s.profile_vignetting = clamp(s.profile_vignetting, 0.0, 100.0, 100.0);
        s.distortion = clamp(s.distortion, -100.0, 100.0, 0.0);
        s.purple_amount = clamp(s.purple_amount, 0.0, 100.0, 0.0);
        s.purple_hue_low = clamp(s.purple_hue_low, 0.0, 360.0, 270.0);
        s.purple_hue_high = clamp(s.purple_hue_high, 0.0, 360.0, 310.0);
        s.green_amount = clamp(s.green_amount, 0.0, 100.0, 0.0);
        s.green_hue_low = clamp(s.green_hue_low, 0.0, 360.0, 60.0);
        s.green_hue_high = clamp(s.green_hue_high, 0.0, 360.0, 120.0);
        s.vignette_amount = clamp(s.vignette_amount, -100.0, 100.0, 0.0);
        s.vignette_midpoint = clamp(s.vignette_midpoint, 0.0, 100.0, 50.0);
        if s.purple_hue_low > s.purple_hue_high {
            std::mem::swap(&mut s.purple_hue_low, &mut s.purple_hue_high);
        }
        if s.green_hue_low > s.green_hue_high {
            std::mem::swap(&mut s.green_hue_low, &mut s.green_hue_high);
        }
        s
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct CameraRawCalibration {
    pub shadow_tint: f64,
    pub red_hue: f64,
    pub red_saturation: f64,
    pub green_hue: f64,
    pub green_saturation: f64,
    pub blue_hue: f64,
    pub blue_saturation: f64,
    pub process: ProcessVersion,
}
impl Default for CameraRawCalibration {
    fn default() -> Self {
        Self {
            shadow_tint: 0.0,
            red_hue: 0.0,
            red_saturation: 0.0,
            green_hue: 0.0,
            green_saturation: 0.0,
            blue_hue: 0.0,
            blue_saturation: 0.0,
            process: ProcessVersion::Version6,
        }
    }
}
impl CameraRawCalibration {
    pub fn normalized(&self) -> Self {
        let mut s = self.clone();
        s.shadow_tint = clamp(s.shadow_tint, -100.0, 100.0, 0.0);
        s.red_hue = clamp(s.red_hue, -100.0, 100.0, 0.0);
        s.red_saturation = clamp(s.red_saturation, -100.0, 100.0, 0.0);
        s.green_hue = clamp(s.green_hue, -100.0, 100.0, 0.0);
        s.green_saturation = clamp(s.green_saturation, -100.0, 100.0, 0.0);
        s.blue_hue = clamp(s.blue_hue, -100.0, 100.0, 0.0);
        s.blue_saturation = clamp(s.blue_saturation, -100.0, 100.0, 0.0);
        s
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct CameraRawWheel {
    pub hue: f64,
    pub saturation: f64,
    pub luminance: f64,
}
impl Default for CameraRawWheel {
    fn default() -> Self {
        Self {
            hue: 0.0,
            saturation: 0.0,
            luminance: 0.0,
        }
    }
}
impl CameraRawWheel {
    pub fn normalized(&self) -> Self {
        let mut s = self.clone();
        s.hue = clamp(s.hue, 0.0, 360.0, 0.0);
        s.saturation = clamp(s.saturation, 0.0, 100.0, 0.0);
        s.luminance = clamp(s.luminance, -100.0, 100.0, 0.0);
        s
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct CameraRawPointColor {
    pub hue: f64,
    pub saturation: f64,
    pub luminance: f64,
    pub hue_shift: f64,
    pub saturation_shift: f64,
    pub luminance_shift: f64,
    pub hue_range: f64,
    pub saturation_range: f64,
    pub luminance_range: f64,
}
impl Default for CameraRawPointColor {
    fn default() -> Self {
        Self {
            hue: 0.0,
            saturation: 0.0,
            luminance: 0.0,
            hue_shift: 0.0,
            saturation_shift: 0.0,
            luminance_shift: 0.0,
            hue_range: 30.0,
            saturation_range: 0.4,
            luminance_range: 0.4,
        }
    }
}
impl CameraRawPointColor {
    pub fn normalized(&self) -> Self {
        let mut s = self.clone();
        s.hue = clamp(s.hue, 0.0, 360.0, 0.0);
        s.saturation = clamp(s.saturation, 0.0, 1.0, 0.0);
        s.luminance = clamp(s.luminance, 0.0, 1.0, 0.0);
        s.hue_shift = clamp(s.hue_shift, -100.0, 100.0, 0.0);
        s.saturation_shift = clamp(s.saturation_shift, -100.0, 100.0, 0.0);
        s.luminance_shift = clamp(s.luminance_shift, -100.0, 100.0, 0.0);
        s.hue_range = clamp(s.hue_range, 5.0, 180.0, 30.0);
        s.saturation_range = clamp(s.saturation_range, 0.05, 1.0, 0.4);
        s.luminance_range = clamp(s.luminance_range, 0.05, 1.0, 0.4);
        s
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct CameraRawGeometry {
    pub vertical: f64,
    pub horizontal: f64,
    pub rotate: f64,
    pub aspect: f64,
    pub scale: f64,
    pub offset_x: f64,
    pub offset_y: f64,
    pub upright: UprightMode,
    pub projection: Projection,
    pub constrain_crop: bool,
    pub guides: Vec<GeometryGuide>,
}
impl Default for CameraRawGeometry {
    fn default() -> Self {
        Self {
            vertical: 0.0,
            horizontal: 0.0,
            rotate: 0.0,
            aspect: 0.0,
            scale: 0.0,
            offset_x: 0.0,
            offset_y: 0.0,
            upright: UprightMode::Off,
            projection: Projection::Perspective,
            constrain_crop: false,
            guides: vec![],
        }
    }
}
impl CameraRawGeometry {
    pub fn normalized(&self) -> Self {
        let mut s = self.clone();
        s.vertical = clamp(s.vertical, -100.0, 100.0, 0.0);
        s.horizontal = clamp(s.horizontal, -100.0, 100.0, 0.0);
        s.rotate = clamp(s.rotate, -45.0, 45.0, 0.0);
        s.aspect = clamp(s.aspect, -100.0, 100.0, 0.0);
        s.scale = clamp(s.scale, -100.0, 100.0, 0.0);
        s.offset_x = clamp(s.offset_x, -100.0, 100.0, 0.0);
        s.offset_y = clamp(s.offset_y, -100.0, 100.0, 0.0);
        s.guides.retain(|g| {
            [g.start_x, g.start_y, g.end_x, g.end_y]
                .iter()
                .all(|v| v.is_finite())
                && (g.end_x - g.start_x).hypot(g.end_y - g.start_y) > 0.01
        });
        s.guides.truncate(4);
        s
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct CameraRawMixer {
    pub hue: [f64; 8],
    pub saturation: [f64; 8],
    pub luminance: [f64; 8],
    pub points: Vec<CameraRawPointColor>,
}
impl Default for CameraRawMixer {
    fn default() -> Self {
        Self {
            hue: [0.0; 8],
            saturation: [0.0; 8],
            luminance: [0.0; 8],
            points: vec![],
        }
    }
}
impl CameraRawMixer {
    pub fn normalized(&self) -> Self {
        let mut s = self.clone();
        for v in s
            .hue
            .iter_mut()
            .chain(s.saturation.iter_mut())
            .chain(s.luminance.iter_mut())
        {
            *v = clamp(*v, -100.0, 100.0, 0.0);
        }
        s.points = self.points.iter().take(8).map(|p| p.normalized()).collect();
        s
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct CameraRawGrading {
    pub shadows: CameraRawWheel,
    pub midtones: CameraRawWheel,
    pub highlights: CameraRawWheel,
    pub global: CameraRawWheel,
    pub blending: f64,
    pub balance: f64,
}
impl Default for CameraRawGrading {
    fn default() -> Self {
        Self {
            shadows: CameraRawWheel::default(),
            midtones: CameraRawWheel::default(),
            highlights: CameraRawWheel::default(),
            global: CameraRawWheel::default(),
            blending: 50.0,
            balance: 0.0,
        }
    }
}
impl CameraRawGrading {
    pub fn normalized(&self) -> Self {
        Self {
            shadows: self.shadows.normalized(),
            midtones: self.midtones.normalized(),
            highlights: self.highlights.normalized(),
            global: self.global.normalized(),
            blending: clamp(self.blending, 0.0, 100.0, 50.0),
            balance: clamp(self.balance, -100.0, 100.0, 0.0),
        }
    }
}
fn repair(points: &[CurvePoint]) -> Vec<CurvePoint> {
    let mut p: Vec<_> = points
        .iter()
        .copied()
        .filter(|p| p.x.is_finite() && p.y.is_finite())
        .collect();
    p.sort_by(|a, b| a.x.total_cmp(&b.x));
    if p.len() < 2 {
        return linear();
    }
    let first = CurvePoint {
        x: 0.0,
        y: p[0].y.clamp(0.0, 1.0),
    };
    let last = CurvePoint {
        x: 1.0,
        y: p.last().unwrap().y.clamp(0.0, 1.0),
    };
    let mut out = vec![first];
    for p in p.iter().skip(1).take(p.len() - 2).take(128) {
        let x = p.x.clamp(0.01, 0.99);
        if x > out.last().unwrap().x + 0.01 {
            out.push(CurvePoint {
                x,
                y: p.y.clamp(0.0, 1.0),
            });
        }
    }
    out.push(last);
    out
}
fn point(tone: f64, points: &[CurvePoint]) -> f64 {
    let mut c = CurvesSettings::default();
    c.channels[0] = points
        .iter()
        .map(|p| CurvePoint {
            x: p.x * 255.0,
            y: p.y * 255.0,
        })
        .collect();
    c.value(tone * 255.0, 0) / 255.0
}
fn bend(t: f64, lo: f64, a: f64, hi: f64, b: f64) -> f64 {
    if t < lo && lo > 0.0 {
        return lo * (t / lo).powf(2.0f64.powf(-a / 100.0 * 1.66));
    }
    if t > hi && hi < 1.0 {
        return 1.0 - (1.0 - hi) * ((1.0 - t) / (1.0 - hi)).powf(2.0f64.powf(b / 100.0 * 1.66));
    }
    t
}
impl CameraRawCurve {
    pub fn tone_table(&self) -> [f32; 256] {
        let anchors: Vec<_> = (0..=32)
            .map(|i| {
                let x = i as f64 / 32.0;
                CurvePoint {
                    x,
                    y: bend(
                        bend(
                            x,
                            self.shadow_split / 100.0,
                            self.shadows,
                            self.light_split / 100.0,
                            self.highlights,
                        ),
                        self.dark_split / 100.0,
                        self.darks,
                        self.dark_split / 100.0,
                        self.lights,
                    ),
                }
            })
            .collect();
        std::array::from_fn(|i| {
            let t = i as f64 / 255.0;
            let p = if [self.shadows, self.darks, self.lights, self.highlights]
                .iter()
                .any(|x| *x != 0.0)
            {
                point(t, &anchors)
            } else {
                t
            };
            point(p, &self.rgb) as f32
        })
    }
}
extern "C" {
    fn adjust_camera_raw(
        rgba: *mut u8,
        width: usize,
        height: usize,
        stride: usize,
        red_gain: f64,
        green_gain: f64,
        blue_gain: f64,
        exposure: f64,
        contrast: f64,
        highlights: f64,
        shadows: f64,
        whites: f64,
        blacks: f64,
        vibrance: f64,
        saturation: f64,
        clipping: i32,
    );
    fn adjust_camera_raw_effects(
        rgba: *mut u8,
        width: usize,
        height: usize,
        stride: usize,
        texture: f64,
        clarity: f64,
        dehaze: f64,
        glow: f64,
        glow_style: i32,
        glow_range: f64,
        glow_spread: f64,
        glow_warmth: f64,
        vignette_amount: f64,
        vignette_midpoint: f64,
        vignette_roundness: f64,
        vignette_feather: f64,
        vignette_highlights: f64,
        vignette_style: i32,
        scale: f64,
    );
    fn adjust_camera_raw_curve_color(
        rgba: *mut u8,
        width: usize,
        height: usize,
        stride: usize,
        tone_lut: *const f32,
        red_lut: *const f32,
        green_lut: *const f32,
        blue_lut: *const f32,
        refine_saturation: f64,
        mixer: *const f32,
        point_count: i32,
        points: *const f32,
        grade: *const f32,
        blending: f64,
        balance: f64,
        visualize: i32,
    );
    fn adjust_camera_raw_detail(
        rgba: *mut u8,
        width: usize,
        height: usize,
        stride: usize,
        sharpen_amount: f64,
        sharpen_radius: f64,
        sharpen_detail: f64,
        sharpen_masking: f64,
        noise_luminance: f64,
        noise_luminance_detail: f64,
        noise_luminance_contrast: f64,
        noise_color: f64,
        noise_color_detail: f64,
        noise_color_smoothness: f64,
        scale: f64,
    );
    fn adjust_camera_raw_optics(
        rgba: *mut u8,
        width: usize,
        height: usize,
        stride: usize,
        remove_chromatic: i32,
        lens_profile: i32,
        profile_distortion: f64,
        profile_vignetting: f64,
        distortion_k: f64,
        purple_amount: f64,
        purple_hue_low: f64,
        purple_hue_high: f64,
        green_amount: f64,
        green_hue_low: f64,
        green_hue_high: f64,
        vignette_amount: f64,
        vignette_midpoint: f64,
        scale: f64,
    );
    fn adjust_camera_raw_calibration(
        rgba: *mut u8,
        width: usize,
        height: usize,
        stride: usize,
        shadow_tint: f64,
        red_hue: f64,
        red_saturation: f64,
        green_hue: f64,
        green_saturation: f64,
        blue_hue: f64,
        blue_saturation: f64,
        process_version: i32,
    );
    fn adjust_grain(
        rgba: *mut u8,
        width: usize,
        height: usize,
        stride: usize,
        amount: f64,
        size: f64,
        roughness: f64,
        seed: u32,
        origin_x: f64,
        origin_y: f64,
        units_per_pixel: f64,
    );
    fn adjust_camera_raw_clip_overlay(
        rgba: *mut u8,
        width: usize,
        height: usize,
        stride: usize,
        shadows: i32,
        highlights: i32,
    );
    fn adjust_camera_raw_sharpen_mask_overlay(
        rgba: *mut u8,
        width: usize,
        height: usize,
        stride: usize,
        sharpen_radius: f64,
        sharpen_detail: f64,
        sharpen_masking: f64,
        scale: f64,
    );
}

impl CameraRawSettings {
    pub fn is_identity(&self) -> bool {
        let s = self.normalized();
        !s.light_color()
            && !s.effects()
            && !s.curve_color()
            && !s.detail_active()
            && !s.optics_active()
            && !s.calibration_active()
            && !s.geometry.active()
    }
    fn light_color(&self) -> bool {
        [
            self.temperature,
            self.tint,
            self.exposure,
            self.contrast,
            self.highlights,
            self.shadows,
            self.whites,
            self.blacks,
            self.vibrance,
            self.saturation,
        ]
        .iter()
        .any(|v| *v != 0.0)
    }
    fn effects(&self) -> bool {
        [
            self.texture,
            self.clarity,
            self.dehaze,
            self.glow,
            self.vignette_amount,
            self.grain_amount,
        ]
        .iter()
        .any(|v| *v != 0.0)
    }
    fn curve_color(&self) -> bool {
        let c = &self.curve;
        let m = &self.mixer;
        let g = &self.grading;
        [
            c.shadows,
            c.darks,
            c.lights,
            c.highlights,
            c.refine_saturation,
        ]
        .iter()
        .any(|v| *v != 0.0)
            || [&c.rgb, &c.red, &c.green, &c.blue]
                .iter()
                .any(|p| **p != linear())
            || m.hue
                .iter()
                .chain(m.saturation.iter())
                .chain(m.luminance.iter())
                .any(|v| *v != 0.0)
            || m.points.iter().any(|p| {
                p.hue_shift != 0.0 || p.saturation_shift != 0.0 || p.luminance_shift != 0.0
            })
            || [&g.shadows, &g.midtones, &g.highlights, &g.global]
                .iter()
                .any(|w| w.saturation != 0.0 || w.luminance != 0.0)
    }
    fn detail_active(&self) -> bool {
        self.detail.sharpen_amount != 0.0
            || self.detail.noise_luminance != 0.0
            || self.detail.noise_color != 0.0
    }
    fn optics_active(&self) -> bool {
        let o = &self.optics;
        o.remove_chromatic_aberration
            || o.enable_lens_profile
            || [
                o.distortion,
                o.purple_amount,
                o.green_amount,
                o.vignette_amount,
            ]
            .iter()
            .any(|v| *v != 0.0)
    }
    fn calibration_active(&self) -> bool {
        let c = &self.calibration;
        [
            c.shadow_tint,
            c.red_hue,
            c.red_saturation,
            c.green_hue,
            c.green_saturation,
            c.blue_hue,
            c.blue_saturation,
        ]
        .iter()
        .any(|v| *v != 0.0)
    }
    pub fn apply(&self, raster: &Raster) -> Result<Raster, CommandError> {
        self.apply_preview(raster, 0, -1, false, false, false)
    }
    /// Preview diagnostics are never part of a committed filter's parameters.
    pub fn apply_preview(
        &self,
        raster: &Raster,
        clipping: i32,
        visualize: i32,
        sharpen_mask: bool,
        shadow_overlay: bool,
        highlight_overlay: bool,
    ) -> Result<Raster, CommandError> {
        let s = self.normalized();
        if s.is_identity()
            && clipping == 0
            && visualize < 0
            && !sharpen_mask
            && !shadow_overlay
            && !highlight_overlay
        {
            return Ok(raster.clone());
        }
        let source = if clipping == 0 && visualize < 0 && !sharpen_mask {
            s.geometry.apply(raster)?
        } else {
            raster.clone()
        };
        let mut data = source.bytes().to_vec();
        let p = data.as_mut_ptr();
        let w = source.width as usize;
        let h = source.height as usize;
        let stride = w * 4;
        super::camera_raw_runtime::begin();
        unsafe {
            if clipping == 0 && !sharpen_mask && s.calibration_active() {
                let c = &s.calibration;
                adjust_camera_raw_calibration(
                    p,
                    w,
                    h,
                    stride,
                    c.shadow_tint,
                    c.red_hue,
                    c.red_saturation,
                    c.green_hue,
                    c.green_saturation,
                    c.blue_hue,
                    c.blue_saturation,
                    c.process.kernel(),
                );
            }
            if s.light_color() || clipping != 0 {
                let warm = s.temperature / 100.0;
                let tint = s.tint / 100.0;
                adjust_camera_raw(
                    p,
                    w,
                    h,
                    stride,
                    1.0 + 0.35 * warm + 0.15 * tint,
                    1.0 - 0.30 * tint,
                    1.0 - 0.35 * warm + 0.15 * tint,
                    s.exposure,
                    s.contrast,
                    s.highlights,
                    s.shadows,
                    s.whites,
                    s.blacks,
                    s.vibrance,
                    s.saturation,
                    clipping,
                );
            }
            if clipping == 0 && !sharpen_mask && (s.curve_color() || visualize >= 0) {
                let c = &s.curve;
                let tone = c.tone_table();
                let table = |points: &[CurvePoint]| -> [f32; 256] {
                    std::array::from_fn(|i| point(i as f64 / 255.0, points) as f32)
                };
                let red = table(&c.red);
                let green = table(&c.green);
                let blue = table(&c.blue);
                let m = &s.mixer;
                let mixer: Vec<f32> = m
                    .hue
                    .iter()
                    .chain(m.saturation.iter())
                    .chain(m.luminance.iter())
                    .map(|v| (*v / 100.0) as f32)
                    .collect();
                let points: Vec<f32> = m
                    .points
                    .iter()
                    .flat_map(|q| {
                        [
                            q.hue / 360.0,
                            q.saturation,
                            q.luminance,
                            q.hue_shift / 100.0,
                            q.saturation_shift / 100.0,
                            q.luminance_shift / 100.0,
                            q.hue_range / 360.0,
                            q.saturation_range,
                            q.luminance_range,
                        ]
                        .map(|v| v as f32)
                    })
                    .collect();
                let g = &s.grading;
                let grade: Vec<f32> = [&g.shadows, &g.midtones, &g.highlights, &g.global]
                    .iter()
                    .flat_map(|q| {
                        [q.hue / 360.0, q.saturation / 100.0, q.luminance / 100.0].map(|v| v as f32)
                    })
                    .collect();
                adjust_camera_raw_curve_color(
                    p,
                    w,
                    h,
                    stride,
                    tone.as_ptr(),
                    red.as_ptr(),
                    green.as_ptr(),
                    blue.as_ptr(),
                    c.refine_saturation / 100.0,
                    mixer.as_ptr(),
                    m.points.len() as i32,
                    points.as_ptr(),
                    grade.as_ptr(),
                    g.blending / 100.0,
                    g.balance / 100.0,
                    visualize,
                );
            }
            if clipping == 0 && !sharpen_mask && s.effects() {
                if [s.texture, s.clarity, s.dehaze, s.glow, s.vignette_amount]
                    .iter()
                    .any(|v| *v != 0.0)
                {
                    adjust_camera_raw_effects(
                        p,
                        w,
                        h,
                        stride,
                        s.texture,
                        s.clarity,
                        s.dehaze,
                        s.glow,
                        s.glow_style.kernel(),
                        s.glow_range,
                        s.glow_spread,
                        s.glow_warmth,
                        s.vignette_amount,
                        s.vignette_midpoint,
                        s.vignette_roundness,
                        s.vignette_feather,
                        s.vignette_highlights,
                        s.vignette_style.kernel(),
                        s.pixel_scale,
                    );
                }
                if s.grain_amount > 0.0 {
                    adjust_grain(
                        p,
                        w,
                        h,
                        stride,
                        s.grain_amount,
                        0.5 + s.grain_size / 100.0 * 19.5,
                        s.grain_roughness,
                        s.seed,
                        0.0,
                        0.0,
                        1.0 / s.pixel_scale,
                    );
                }
            }
            if clipping == 0 {
                let d = &s.detail;
                let o = &s.optics;
                if sharpen_mask {
                    adjust_camera_raw_sharpen_mask_overlay(
                        p,
                        w,
                        h,
                        stride,
                        d.sharpen_radius,
                        d.sharpen_detail,
                        d.sharpen_masking,
                        s.pixel_scale,
                    );
                } else {
                    if s.optics_active() {
                        let k = (o.distortion
                            + if o.enable_lens_profile {
                                o.profile_distortion
                            } else {
                                0.0
                            })
                            / 100.0
                            * super::filters::LENS_STRENGTH;
                        adjust_camera_raw_optics(
                            p,
                            w,
                            h,
                            stride,
                            o.remove_chromatic_aberration as i32,
                            o.enable_lens_profile as i32,
                            o.profile_distortion,
                            o.profile_vignetting,
                            k,
                            o.purple_amount,
                            o.purple_hue_low,
                            o.purple_hue_high,
                            o.green_amount,
                            o.green_hue_low,
                            o.green_hue_high,
                            o.vignette_amount,
                            o.vignette_midpoint,
                            s.pixel_scale,
                        );
                    }
                    if s.detail_active() {
                        adjust_camera_raw_detail(
                            p,
                            w,
                            h,
                            stride,
                            d.sharpen_amount,
                            d.sharpen_radius,
                            d.sharpen_detail,
                            d.sharpen_masking,
                            d.noise_luminance,
                            d.noise_luminance_detail,
                            d.noise_luminance_contrast,
                            d.noise_color,
                            d.noise_color_detail,
                            d.noise_color_smoothness,
                            s.pixel_scale,
                        );
                    }
                }
            }
            if shadow_overlay || highlight_overlay {
                adjust_camera_raw_clip_overlay(
                    p,
                    w,
                    h,
                    stride,
                    shadow_overlay as i32,
                    highlight_overlay as i32,
                );
            }
        }
        if super::camera_raw_runtime::failed() {
            return Err(CommandError::Refused(
                "Camera Raw ran out of working memory; the original layer was preserved.".into(),
            ));
        }
        Ok(Raster::from_premultiplied(
            source.width,
            source.height,
            data,
        ))
    }
    pub fn neutralize(red: f64, green: f64, blue: f64) -> Option<(f64, f64)> {
        if ![red, green, blue]
            .iter()
            .all(|v| v.is_finite() && *v > 0.0001)
        {
            return None;
        }
        let a1 = 0.35 * red;
        let b1 = 0.15 * red + 0.30 * green;
        let c1 = green - red;
        let a2 = -0.35 * blue;
        let b2 = 0.15 * blue + 0.30 * green;
        let c2 = green - blue;
        let det = a1 * b2 - a2 * b1;
        if det.abs() <= 1e-8 {
            return None;
        }
        let warm = (c1 * b2 - c2 * b1) / det;
        let tint = (a1 * c2 - a2 * c1) / det;
        if !warm.is_finite() || !tint.is_finite() {
            None
        } else {
            Some((warm * 100.0, tint * 100.0))
        }
    }
    pub fn auto_balance(raster: &Raster) -> Option<(f64, f64)> {
        let mut sum = [0.0; 3];
        let mut count = 0.0;
        for p in raster.bytes().chunks_exact(4) {
            if p[3] == 0 {
                continue;
            }
            for c in 0..3 {
                let v = (p[c] as f64 / p[3] as f64).min(1.0);
                sum[c] += if v <= 0.04045 {
                    v / 12.92
                } else {
                    ((v + 0.055) / 1.055).powf(2.4)
                };
            }
            count += 1.0;
        }
        if count == 0.0 {
            None
        } else {
            Self::neutralize(sum[0] / count, sum[1] / count, sum[2] / count)
        }
    }
}

impl CameraRawGeometry {
    fn active(&self) -> bool {
        [
            self.vertical,
            self.horizontal,
            self.rotate,
            self.aspect,
            self.scale,
            self.offset_x,
            self.offset_y,
        ]
        .iter()
        .any(|v| *v != 0.0)
            || (self.upright == UprightMode::Guided && !self.guides.is_empty())
    }
    fn corners(&self, w: f64, h: f64) -> [crate::Point; 4] {
        let (mut vertical, mut horizontal, mut rotate) =
            (self.vertical, self.horizontal, self.rotate);
        if self.upright == UprightMode::Guided {
            if let Some(g) = self.guides.first() {
                let angle = (g.end_y - g.start_y)
                    .atan2(g.end_x - g.start_x)
                    .to_degrees();
                let mut correction = -angle;
                if correction > 45.0 {
                    correction -= 90.0
                } else if correction < -45.0 {
                    correction += 90.0
                }
                rotate += correction;
            }
            if let Some(g) = self.guides.get(1) {
                let a = (g.end_y - g.start_y)
                    .atan2(g.end_x - g.start_x)
                    .to_degrees();
                if a.abs() > 45.0 {
                    vertical += if a > 0.0 { 25.0 } else { -25.0 }
                } else {
                    horizontal += if a > 0.0 { 25.0 } else { -25.0 }
                }
            }
        }
        let strength = if self.projection == Projection::Perspective {
            1.0
        } else {
            0.55
        };
        let v = vertical / 100.0 * w * 0.18 * strength;
        let hz = horizontal / 100.0 * h * 0.18 * strength;
        let sx = self.offset_x / 100.0 * w * 0.15;
        let sy = self.offset_y / 100.0 * h * 0.15;
        let cx = w / 2.0 + sx;
        let cy = h / 2.0 + sy;
        let a = 1.0 + self.aspect / 200.0;
        let zoom = 1.0 + self.scale / 100.0;
        let angle = rotate.to_radians();
        // Port Mac's y-up corner calculation, then convert to the engine's y-down grid.
        [
            (-v + sx, h + sy),
            (w + v + sx, h + sy),
            (w + hz + sx, -sy),
            (-hz + sx, -sy),
        ]
        .map(|(x, y)| {
            let dx = x - cx;
            let dy = y - cy;
            let rx = dx * angle.cos() - dy * angle.sin();
            let ry = dx * angle.sin() + dy * angle.cos();
            crate::Point {
                x: cx + rx * a * zoom,
                y: h - (cy + ry / a * zoom),
            }
        })
    }
    /// Perspective sampling interpolates straight colour and alpha separately,
    /// with clear black outside the input extent, then restores premultiplied
    /// storage. Ordinary layer sampling clamps premultiplied texels instead.
    fn transparent_sample(raster: &Raster, x: f64, y: f64) -> [f32; 4] {
        let (w, h) = (raster.width as i64, raster.height as i64);
        if !x.is_finite()
            || !y.is_finite()
            || x <= -0.5
            || y <= -0.5
            || x >= w as f64 + 0.5
            || y >= h as f64 + 0.5
        {
            return [0.0; 4];
        }
        let (fx, fy) = (x - 0.5, y - 0.5);
        let (x0, y0) = (fx.floor() as i64, fy.floor() as i64);
        let (tx, ty) = ((fx - x0 as f64) as f32, (fy - y0 as f64) as f32);
        let fetch = |px: i64, py: i64| {
            if px < 0 || py < 0 || px >= w || py >= h {
                return [0.0; 4];
            }
            let pixel = raster.pixel(px as u32, py as u32);
            let alpha = pixel[3] as f32;
            if alpha == 0.0 {
                return [0.0; 4];
            }
            [
                pixel[0] as f32 / alpha,
                pixel[1] as f32 / alpha,
                pixel[2] as f32 / alpha,
                alpha / 255.0,
            ]
        };
        let (a, b, c, d) = (
            fetch(x0, y0),
            fetch(x0 + 1, y0),
            fetch(x0, y0 + 1),
            fetch(x0 + 1, y0 + 1),
        );
        let mut sample = std::array::from_fn(|i| {
            (a[i] * (1.0 - tx) + b[i] * tx) * (1.0 - ty)
                + (c[i] * (1.0 - tx) + d[i] * tx) * ty
        });
        for channel in 0..3 {
            sample[channel] *= sample[3];
        }
        sample
    }
    fn apply(&self, raster: &Raster) -> Result<Raster, CommandError> {
        if !self.active() {
            return Ok(raster.clone());
        }
        let w = raster.width;
        let h = raster.height;
        let corners = self.corners(w as f64, h as f64);
        if !crate::Homography::is_usable(&corners) {
            return Err(CommandError::Argument(
                "Camera Raw geometry would collapse or cross the image corners.".into(),
            ));
        }
        let inv = crate::Homography::unit_to(&corners)
            .invert()
            .ok_or_else(|| CommandError::Argument("Invalid Camera Raw geometry.".into()))?;
        let mut data = vec![0u8; w as usize * h as usize * 4];
        for y in 0..h {
            for x in 0..w {
                let p = inv.apply(crate::Point {
                    x: x as f64 + 0.5,
                    y: y as f64 + 0.5,
                });
                let px = p.x * w as f64;
                let py = p.y * h as f64;
                let sample = Self::transparent_sample(raster, px, py);
                let i = (y as usize * w as usize + x as usize) * 4;
                for c in 0..4 {
                    data[i + c] = (sample[c] * 255.0).round().clamp(0.0, 255.0) as u8;
                }
            }
        }
        if self.constrain_crop {
            let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
            for y in 0..h {
                for x in 0..w {
                    if data[(y as usize * w as usize + x as usize) * 4 + 3] != 0 {
                        x0 = x0.min(x);
                        y0 = y0.min(y);
                        x1 = x1.max(x + 1);
                        y1 = y1.max(y + 1);
                    }
                }
            }
            if x1 > x0 && y1 > y0 && (x1 - x0 < w || y1 - y0 < h) {
                let result = Raster::from_premultiplied(w, h, data);
                let scale = (w as f64 / (x1 - x0) as f64).min(h as f64 / (y1 - y0) as f64);
                let dw = (x1 - x0) as f64 * scale;
                let dh = (y1 - y0) as f64 * scale;
                let ox = (w as f64 - dw) / 2.0;
                let oy = (h as f64 - dh) / 2.0;
                data = vec![0u8; w as usize * h as usize * 4];
                for y in 0..h {
                    for x in 0..w {
                        let sx = (x as f64 + 0.5 - ox) / scale;
                        let sy = (y as f64 + 0.5 - oy) / scale;
                        if sx < 0.0 || sy < 0.0 || sx >= (x1 - x0) as f64 || sy >= (y1 - y0) as f64
                        {
                            continue;
                        }
                        let sample = crate::compositor::sample(
                            &result,
                            x0 as f64 + sx,
                            y0 as f64 + sy,
                            false,
                        );
                        let i = (y as usize * w as usize + x as usize) * 4;
                        for c in 0..4 {
                            data[i + c] = (sample[c] * 255.0).round().clamp(0.0, 255.0) as u8;
                        }
                    }
                }
            }
        }
        Ok(Raster::from_premultiplied(w, h, data))
    }
}
