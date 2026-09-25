use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

fn clamp_or(n: f64, lo: f64, hi: f64, fallback: f64) -> f64 { if n.is_finite() { n.clamp(lo, hi) } else { fallback } }

/// Swift's `JSONEncoder` writes a whole-number `Double` without a decimal point (`0`, not `0.0`).
/// `serde_json`'s default `f64` writer always keeps the decimal point, so every numeric field that
/// must match the Mac byte for byte serializes and deserializes through this module instead.
pub(crate) mod mac_number {
    use serde::{ser::Error, Deserialize, Deserializer, Serializer};

    /// A non-finite value is refused rather than written: `serde_json` would emit `null`, which
    /// neither this reader nor the Mac's can read back.
    pub fn serialize<S: Serializer>(value: &f64, serializer: S) -> Result<S::Ok, S::Error> {
        if !value.is_finite() { return Err(S::Error::custom(format!("non-finite adjustment number {value}"))); }
        if value.fract() == 0.0 && value.abs() < 1e15 {
            serializer.serialize_i64(*value as i64)
        } else {
            serializer.serialize_f64(*value)
        }
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
        f64::deserialize(deserializer)
    }
}

/// The optional-field counterpart of `mac_number`, for fields written only when present.
pub(crate) mod mac_number_opt {
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(value: &Option<f64>, s: S) -> Result<S::Ok, S::Error> {
        match value { Some(v) => super::mac_number::serialize(v, s), None => s.serialize_none() }
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<f64>, D::Error> { Option::<f64>::deserialize(d) }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AdjustmentKind {
    #[default] #[serde(rename = "Hue/Saturation")] Hsv,
    #[serde(rename = "Levels")] Levels,
    #[serde(rename = "Curves")] Curves,
    #[serde(rename = "Exposure")] Exposure,
    #[serde(rename = "Gradient Map")] GradientMap,
    #[serde(rename = "Grain")] Grain,
    // Mac 1.2.6 additions (R 3).
    #[serde(rename = "Add Noise")] AddNoise,
    #[serde(rename = "Gaussian Blur")] GaussianBlur,
    #[serde(rename = "Motion Blur")] MotionBlur,
    #[serde(rename = "Invert")] Invert,
    #[serde(rename = "Black & White")] BlackWhite,
    #[serde(rename = "Color Balance")] ColorBalance,
}
impl AdjustmentKind {
    /// Gaussian Blur, Motion Blur and Add Noise adjustment layers need format v9 (ProjectStore.swift:199-204).
    pub fn needs_version_9(self) -> bool { matches!(self, AdjustmentKind::GaussianBlur | AdjustmentKind::MotionBlur | AdjustmentKind::AddNoise) }
    /// Whether the kind opens an editor. Invert has nothing to set (`isEditable`, LayerAdjustment.swift:28).
    pub fn is_editable(self) -> bool { self != AdjustmentKind::Invert }
    /// The kinds that read neighbouring pixels: they blur what lies beneath them as a whole
    /// (compositor::spatial_target) rather than mapping one colour at a time.
    pub fn is_spatial(self) -> bool { matches!(self, AdjustmentKind::GaussianBlur | AdjustmentKind::MotionBlur) }
    pub fn name(self) -> &'static str {
        match self {
            AdjustmentKind::Hsv => "Hue/Saturation", AdjustmentKind::Levels => "Levels", AdjustmentKind::Curves => "Curves", AdjustmentKind::Exposure => "Exposure", AdjustmentKind::GradientMap => "Gradient Map", AdjustmentKind::Grain => "Grain",
            AdjustmentKind::AddNoise => "Add Noise", AdjustmentKind::GaussianBlur => "Gaussian Blur", AdjustmentKind::MotionBlur => "Motion Blur", AdjustmentKind::Invert => "Invert", AdjustmentKind::BlackWhite => "Black & White", AdjustmentKind::ColorBalance => "Color Balance",
        }
    }
    /// Undo names, static so `Command::action_name` can stay `&'static str`.
    pub fn new_action_name(self) -> &'static str {
        match self {
            AdjustmentKind::Hsv => "New Hue/Saturation Adjustment", AdjustmentKind::Levels => "New Levels Adjustment", AdjustmentKind::Curves => "New Curves Adjustment",
            AdjustmentKind::Exposure => "New Exposure Adjustment", AdjustmentKind::GradientMap => "New Gradient Map Adjustment", AdjustmentKind::Grain => "New Grain Adjustment",
            AdjustmentKind::AddNoise => "New Add Noise Adjustment", AdjustmentKind::GaussianBlur => "New Gaussian Blur Adjustment", AdjustmentKind::MotionBlur => "New Motion Blur Adjustment",
            AdjustmentKind::Invert => "New Invert Adjustment", AdjustmentKind::BlackWhite => "New Black & White Adjustment", AdjustmentKind::ColorBalance => "New Color Balance Adjustment",
        }
    }
    pub fn edit_action_name(self) -> &'static str {
        match self {
            AdjustmentKind::Hsv => "Edit Hue/Saturation Adjustment", AdjustmentKind::Levels => "Edit Levels Adjustment", AdjustmentKind::Curves => "Edit Curves Adjustment",
            AdjustmentKind::Exposure => "Edit Exposure Adjustment", AdjustmentKind::GradientMap => "Edit Gradient Map Adjustment", AdjustmentKind::Grain => "Edit Grain Adjustment",
            AdjustmentKind::AddNoise => "Edit Add Noise Adjustment", AdjustmentKind::GaussianBlur => "Edit Gaussian Blur Adjustment", AdjustmentKind::MotionBlur => "Edit Motion Blur Adjustment",
            AdjustmentKind::Invert => "Edit Invert Adjustment", AdjustmentKind::BlackWhite => "Edit Black & White Adjustment", AdjustmentKind::ColorBalance => "Edit Color Balance Adjustment",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LevelsChannel { #[default] #[serde(rename = "RGB")] Rgb, #[serde(rename = "Red")] Red, #[serde(rename = "Green")] Green, #[serde(rename = "Blue")] Blue }
impl LevelsChannel {
    pub const ALL: [LevelsChannel; 4] = [LevelsChannel::Rgb, LevelsChannel::Red, LevelsChannel::Green, LevelsChannel::Blue];
    pub fn index(self) -> usize { self as usize }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct LevelRange {
    #[serde(with = "mac_number")] pub black: f64,
    #[serde(with = "mac_number")] pub gamma: f64,
    #[serde(with = "mac_number")] pub white: f64,
    #[serde(rename = "outputBlack", with = "mac_number")] pub output_black: f64,
    #[serde(rename = "outputWhite", with = "mac_number")] pub output_white: f64,
}
impl Default for LevelRange { fn default() -> Self { LevelRange { black: 0.0, gamma: 1.0, white: 255.0, output_black: 0.0, output_white: 255.0 } } }
impl LevelRange {
    pub fn normalized(&self) -> LevelRange {
        let black = clamp_or(self.black, 0.0, 254.0, 0.0);
        LevelRange { black, white: clamp_or(self.white, black + 1.0, 255.0, 255.0), gamma: clamp_or(self.gamma, 0.1, 9.99, 1.0),
            output_black: clamp_or(self.output_black, 0.0, 255.0, 0.0), output_white: clamp_or(self.output_white, 0.0, 255.0, 255.0) }
    }
    pub fn apply(&self, value: f64) -> f64 {
        let s = self.normalized();
        let input = ((value * 255.0 - s.black) / (s.white - s.black)).clamp(0.0, 1.0);
        (s.output_black + input.powf(1.0 / s.gamma) * (s.output_white - s.output_black)) / 255.0
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LevelsSettings { pub channel: LevelsChannel, pub ranges: Vec<LevelRange> }
impl Default for LevelsSettings { fn default() -> Self { LevelsSettings { channel: LevelsChannel::Rgb, ranges: vec![LevelRange::default(); 4] } } }
impl LevelsSettings {
    pub fn is_valid(&self) -> bool { self.ranges.len() == 4 && self.ranges.iter().all(|r| *r == r.normalized()) }
    pub fn is_identity(&self) -> bool { self.ranges.len() == 4 && self.ranges.iter().all(|r| r.normalized() == LevelRange::default()) }
    /// The channel's own range, then the composite RGB range.
    pub fn apply(&self, value: f64, channel: LevelsChannel) -> f64 { self.ranges[0].apply(self.ranges[channel.index()].apply(value)) }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurvePoint { #[serde(with = "mac_number")] pub x: f64, #[serde(with = "mac_number")] pub y: f64 }
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurvesSettings { pub channel: LevelsChannel, pub channels: Vec<Vec<CurvePoint>> }
impl Default for CurvesSettings {
    fn default() -> Self { CurvesSettings { channel: LevelsChannel::Rgb, channels: vec![vec![CurvePoint { x: 0.0, y: 0.0 }, CurvePoint { x: 255.0, y: 255.0 }]; 4] } }
}
impl CurvesSettings {
    pub fn is_valid(&self) -> bool {
        self.channels.len() == 4 && self.channels.iter().all(|p| {
            (2..=32).contains(&p.len()) && p[0].x == 0.0 && p[p.len() - 1].x == 255.0
                && p.iter().all(|q| q.x.is_finite() && q.y.is_finite() && (0.0..=255.0).contains(&q.x) && (0.0..=255.0).contains(&q.y))
                && p.windows(2).all(|w| w[0].x < w[1].x)
        })
    }
    pub fn is_identity(&self) -> bool { *self == CurvesSettings { channel: self.channel, ..CurvesSettings::default() } }
    /// Shape-preserving cubic Hermite interpolation (Fritsch-Carlson), so the curve never overshoots its handles.
    pub fn value(&self, x: f64, channel: usize) -> f64 {
        let p = &self.channels[channel];
        let i = p.iter().rposition(|q| q.x <= x).unwrap_or(0).min(p.len() - 2);
        let d: Vec<f64> = p.windows(2).map(|w| (w[1].y - w[0].y) / (w[1].x - w[0].x)).collect();
        let slope = |j: usize| -> f64 {
            if j == 0 { d[0] } else if j == p.len() - 1 { d[d.len() - 1] }
            else if d[j - 1] * d[j] <= 0.0 { 0.0 } else { 2.0 / (1.0 / d[j - 1] + 1.0 / d[j]) }
        };
        let h = p[i + 1].x - p[i].x;
        let t = ((x - p[i].x) / h).clamp(0.0, 1.0);
        let y = (2.0 * t * t * t - 3.0 * t * t + 1.0) * p[i].y + (t * t * t - 2.0 * t * t + t) * h * slope(i)
            + (-2.0 * t * t * t + 3.0 * t * t) * p[i + 1].y + (t * t * t - t * t) * h * slope(i + 1);
        y.clamp(0.0, 255.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ColorRange { #[default] Master, Reds, Yellows, Greens, Cyans, Blues, Magentas }
/// Swift's `JSONEncoder` is configured with `.sortedKeys`, which sorts every object's keys
/// alphabetically by their *encoded string*, recursively -- not just the top level. `ColorRange`
/// is declared above in chromatic order (used by `default_band`, `ALL`, `COLORS`), so `Ord` is
/// implemented by hand here to sort by the JSON name instead, keeping `BTreeMap<ColorRange, _>`
/// byte-compatible with the Mac even when serialized directly (not through `serde_json::Value`).
impl ColorRange {
    fn alpha_rank(self) -> u8 {
        match self {
            ColorRange::Blues => 0, ColorRange::Cyans => 1, ColorRange::Greens => 2, ColorRange::Magentas => 3,
            ColorRange::Master => 4, ColorRange::Reds => 5, ColorRange::Yellows => 6,
        }
    }
    pub const ALL: [ColorRange; 7] = [ColorRange::Master, ColorRange::Reds, ColorRange::Yellows, ColorRange::Greens, ColorRange::Cyans, ColorRange::Blues, ColorRange::Magentas];
    pub const COLORS: [ColorRange; 6] = [ColorRange::Reds, ColorRange::Yellows, ColorRange::Greens, ColorRange::Cyans, ColorRange::Blues, ColorRange::Magentas];
    /// Photoshop's starting hue band: falloff start, range start, range end, falloff end.
    pub fn default_band(self) -> HueBand {
        let b = |a: f64, b: f64, c: f64, d: f64| HueBand { falloff_start: a, range_start: b, range_end: c, falloff_end: d };
        match self {
            ColorRange::Master => b(0.0, 0.0, 360.0, 360.0), ColorRange::Reds => b(315.0, 345.0, 15.0, 45.0), ColorRange::Yellows => b(15.0, 45.0, 75.0, 105.0),
            ColorRange::Greens => b(75.0, 105.0, 135.0, 165.0), ColorRange::Cyans => b(135.0, 165.0, 195.0, 225.0), ColorRange::Blues => b(195.0, 225.0, 255.0, 285.0),
            ColorRange::Magentas => b(255.0, 285.0, 315.0, 345.0),
        }
    }
}
impl PartialOrd for ColorRange {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> { Some(self.cmp(other)) }
}
impl Ord for ColorRange {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering { self.alpha_rank().cmp(&other.alpha_rank()) }
}

fn wrap360(v: f64) -> f64 { let r = v % 360.0; if r < 0.0 { r + 360.0 } else { r } }

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
/// Field order matches the alphabetical JSON key order Swift's `.sortedKeys`-configured
/// `JSONEncoder` writes, so a direct `to_string` (not routed through `serde_json::Value`) still
/// matches the Mac byte for byte.
pub struct HueBand {
    #[serde(rename = "falloffEnd", with = "mac_number")] pub falloff_end: f64,
    #[serde(rename = "falloffStart", with = "mac_number")] pub falloff_start: f64,
    #[serde(rename = "rangeEnd", with = "mac_number")] pub range_end: f64,
    #[serde(rename = "rangeStart", with = "mac_number")] pub range_start: f64,
}
impl HueBand {
    /// Degrees from `from` forward to `to`, always 0..360.
    pub fn forward(from: f64, to: f64) -> f64 { wrap360(to - from) }
    /// 1 inside the range, ramping linearly through each shoulder, 0 outside.
    pub fn weight(&self, hue: f64) -> f64 {
        let span = Self::forward(self.falloff_start, self.falloff_end);
        if span <= 0.0 { return 1.0; }
        let position = Self::forward(self.falloff_start, hue);
        if position > span { return 0.0; }
        let ramp_in = Self::forward(self.falloff_start, self.range_start);
        let plateau_end = Self::forward(self.falloff_start, self.range_end);
        if position < ramp_in { return if ramp_in > 0.0 { position / ramp_in } else { 1.0 }; }
        if position <= plateau_end { return 1.0; }
        let ramp_out = span - plateau_end;
        if ramp_out > 0.0 { (span - position) / ramp_out } else { 1.0 }
    }
    pub fn handles(&self) -> [f64; 4] { [self.falloff_start, self.range_start, self.range_end, self.falloff_end] }
    pub fn is_finite(&self) -> bool { self.handles().iter().all(|h| h.is_finite()) }
    pub fn centered_on(&self, hue: f64) -> HueBand {
        let core = Self::forward(self.range_start, self.range_end);
        let leading = Self::forward(self.falloff_start, self.range_start);
        let trailing = Self::forward(self.range_end, self.falloff_end);
        let start = wrap360(hue - core / 2.0);
        HueBand { falloff_start: wrap360(start - leading), range_start: start, range_end: wrap360(start + core), falloff_end: wrap360(start + core + trailing) }
    }
    pub fn include(&mut self, hue: f64) {
        if self.weight(hue) >= 1.0 { return; }
        let shoulder_in = Self::forward(self.falloff_start, self.range_start);
        let shoulder_out = Self::forward(self.range_end, self.falloff_end);
        if Self::forward(hue, self.range_start) <= Self::forward(self.range_end, hue) { self.range_start = hue; self.falloff_start = hue - shoulder_in; }
        else { self.range_end = hue; self.falloff_end = hue + shoulder_out; }
        self.normalize();
    }
    pub fn exclude(&mut self, hue: f64) {
        if self.weight(hue) <= 0.0 { return; }
        let shoulder_in = Self::forward(self.falloff_start, self.range_start);
        let shoulder_out = Self::forward(self.range_end, self.falloff_end);
        if Self::forward(self.falloff_start, hue) <= Self::forward(hue, self.falloff_end) { self.falloff_start = hue + 1.0; self.range_start = hue + 1.0 + shoulder_in; }
        else { self.falloff_end = hue - 1.0; self.range_end = hue - 1.0 - shoulder_out; }
        self.normalize();
    }
    fn normalize(&mut self) {
        self.falloff_start = wrap360(self.falloff_start); self.range_start = wrap360(self.range_start);
        self.range_end = wrap360(self.range_end); self.falloff_end = wrap360(self.falloff_end);
        if Self::forward(self.falloff_start, self.falloff_end) > 350.0 { self.falloff_end = wrap360(self.falloff_start + 350.0); }
    }
    /// Moves one handle, keeping the four in order and the band under a full circle; refused otherwise.
    pub fn set_handle(&mut self, index: usize, degrees: f64) {
        let mut updated = *self;
        let value = wrap360(degrees);
        match index { 0 => updated.falloff_start = value, 1 => updated.range_start = value, 2 => updated.range_end = value, _ => updated.falloff_end = value }
        let span = Self::forward(updated.falloff_start, updated.falloff_end);
        let to_start = Self::forward(updated.falloff_start, updated.range_start);
        let to_end = Self::forward(updated.falloff_start, updated.range_end);
        if span > 1.0 && span <= 350.0 && to_start <= to_end && to_end <= span { *self = updated; }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
/// Field order matches the alphabetical JSON key order Swift's `.sortedKeys`-configured
/// `JSONEncoder` writes, so a direct `to_string` (not routed through `serde_json::Value`) still
/// matches the Mac byte for byte.
pub struct RangeAdjustment {
    #[serde(default, with = "mac_number")] pub hue: f64,
    #[serde(default, with = "mac_number")] pub lightness: f64,
    #[serde(default, with = "mac_number")] pub saturation: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HueSaturationSettings {
    pub range: ColorRange,
    pub colorize: bool,
    #[serde(rename = "invertRange")] pub invert_range: bool,
    pub adjustments: BTreeMap<ColorRange, RangeAdjustment>,
    pub bands: BTreeMap<ColorRange, HueBand>,
}
impl Default for HueSaturationSettings { fn default() -> Self { HueSaturationSettings::new(0.0, 0.0, 0.0, false, ColorRange::Master) } }
impl HueSaturationSettings {
    pub fn new(hue: f64, saturation: f64, lightness: f64, colorize: bool, range: ColorRange) -> Self {
        let mut adjustments = BTreeMap::new();
        adjustments.insert(range, RangeAdjustment { hue, saturation, lightness });
        HueSaturationSettings { range, colorize, invert_range: false, adjustments, bands: ColorRange::ALL.iter().map(|r| (*r, r.default_band())).collect() }
    }
    /// Photoshop's starting point when Colorize is switched on.
    pub fn colorize_start() -> Self { HueSaturationSettings::new(0.0, 25.0, 0.0, true, ColorRange::Master) }
    pub fn adjustment(&self, range: ColorRange) -> RangeAdjustment { self.adjustments.get(&range).copied().unwrap_or_default() }
    pub fn band(&self, range: ColorRange) -> HueBand { self.bands.get(&range).copied().unwrap_or_else(|| range.default_band()) }
    pub fn is_identity(&self) -> bool { !self.colorize && self.adjustments.values().all(|a| *a == RangeAdjustment::default()) }
    /// How much a range applies to one hue: Master everywhere, others through their band.
    pub fn weight(&self, range: ColorRange, hue: f64) -> f64 {
        if range == ColorRange::Master { return 1.0; }
        let w = self.band(range).weight(hue);
        if self.invert_range && range == self.range { 1.0 - w } else { w }
    }
    pub fn is_valid(&self) -> bool {
        self.adjustments.values().all(|a| a.hue.is_finite() && a.hue.abs() <= 360.0 && a.saturation.is_finite() && a.saturation.abs() <= 100.0 && a.lightness.is_finite() && a.lightness.abs() <= 100.0)
            && self.bands.values().all(HueBand::is_finite)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AdjustmentColor {
    #[serde(with = "mac_number")] pub red: f64,
    #[serde(with = "mac_number")] pub green: f64,
    #[serde(with = "mac_number")] pub blue: f64,
}
impl AdjustmentColor {
    pub fn is_valid(&self) -> bool { [self.red, self.green, self.blue].iter().all(|c| c.is_finite() && (0.0..=1.0).contains(c)) }
    pub fn clamped(&self) -> Self { AdjustmentColor { red: clamp_or(self.red, 0.0, 1.0, 0.0), green: clamp_or(self.green, 0.0, 1.0, 0.0), blue: clamp_or(self.blue, 0.0, 1.0, 0.0) } }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExposureSettings {
    #[serde(with = "mac_number")] pub exposure: f64,
    #[serde(with = "mac_number")] pub offset: f64,
    #[serde(with = "mac_number")] pub gamma: f64,
}
impl Default for ExposureSettings { fn default() -> Self { ExposureSettings { exposure: 0.0, offset: 0.0, gamma: 1.0 } } }
impl ExposureSettings {
    pub fn is_valid(&self) -> bool { (-20.0..=20.0).contains(&self.exposure) && (-0.5..=0.5).contains(&self.offset) && (0.01..=9.99).contains(&self.gamma) }
    pub fn normalized(&self) -> Self { ExposureSettings { exposure: clamp_or(self.exposure, -20.0, 20.0, 0.0), offset: clamp_or(self.offset, -0.5, 0.5, 0.0), gamma: clamp_or(self.gamma, 0.01, 9.99, 1.0) } }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct GradientMapSettings { pub shadows: AdjustmentColor, pub highlights: AdjustmentColor, pub reversed: bool }
impl Default for GradientMapSettings {
    fn default() -> Self { GradientMapSettings { shadows: AdjustmentColor { red: 0.0, green: 0.0, blue: 0.0 }, highlights: AdjustmentColor { red: 1.0, green: 1.0, blue: 1.0 }, reversed: false } }
}
impl GradientMapSettings {
    pub fn is_valid(&self) -> bool { self.shadows.is_valid() && self.highlights.is_valid() }
    pub fn normalized(&self) -> Self { GradientMapSettings { shadows: self.shadows.clamped(), highlights: self.highlights.clamped(), reversed: self.reversed } }
    /// The colours for the darkest and lightest tones, in the order they apply.
    pub fn ends(&self) -> (AdjustmentColor, AdjustmentColor) { if self.reversed { (self.highlights, self.shadows) } else { (self.shadows, self.highlights) } }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct GrainSettings {
    #[serde(with = "mac_number")] pub amount: f64,
    #[serde(with = "mac_number")] pub size: f64,
    #[serde(with = "mac_number")] pub roughness: f64,
    pub seed: u32,
}
impl Default for GrainSettings { fn default() -> Self { GrainSettings { amount: 25.0, size: 1.5, roughness: 50.0, seed: 0 } } }
impl GrainSettings {
    pub fn is_valid(&self) -> bool { (0.0..=100.0).contains(&self.amount) && (0.5..=20.0).contains(&self.size) && (0.0..=100.0).contains(&self.roughness) }
    pub fn normalized(&self) -> Self { GrainSettings { amount: clamp_or(self.amount, 0.0, 100.0, 25.0), size: clamp_or(self.size, 0.5, 20.0, 1.5), roughness: clamp_or(self.roughness, 0.0, 100.0, 50.0), seed: self.seed } }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BlackWhiteSettings {
    #[serde(with = "mac_number")] pub blues: f64,
    #[serde(with = "mac_number")] pub cyans: f64,
    #[serde(with = "mac_number")] pub greens: f64,
    #[serde(with = "mac_number")] pub magentas: f64,
    #[serde(with = "mac_number")] pub reds: f64,
    pub tint: bool,
    #[serde(rename = "tintHue", with = "mac_number")] pub tint_hue: f64,
    #[serde(rename = "tintSaturation", with = "mac_number")] pub tint_saturation: f64,
    #[serde(with = "mac_number")] pub yellows: f64,
}
impl Default for BlackWhiteSettings {
    fn default() -> Self { BlackWhiteSettings { reds: 40.0, yellows: 60.0, greens: 40.0, cyans: 60.0, blues: 20.0, magentas: 80.0,
        tint: false, tint_hue: 40.0, tint_saturation: 20.0 } }
}
impl BlackWhiteSettings {
    pub fn is_valid(&self) -> bool {
        [self.reds, self.yellows, self.greens, self.cyans, self.blues, self.magentas].iter().all(|w| (-200.0..=300.0).contains(w))
            && (0.0..=360.0).contains(&self.tint_hue) && (0.0..=100.0).contains(&self.tint_saturation)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ColorBalanceSettings {
    #[serde(rename = "highlightCyanRed", with = "mac_number")] pub highlight_cyan_red: f64,
    #[serde(rename = "highlightMagentaGreen", with = "mac_number")] pub highlight_magenta_green: f64,
    #[serde(rename = "highlightYellowBlue", with = "mac_number")] pub highlight_yellow_blue: f64,
    #[serde(rename = "midCyanRed", with = "mac_number")] pub mid_cyan_red: f64,
    #[serde(rename = "midMagentaGreen", with = "mac_number")] pub mid_magenta_green: f64,
    #[serde(rename = "midYellowBlue", with = "mac_number")] pub mid_yellow_blue: f64,
    #[serde(rename = "preserveLuminosity")] pub preserve_luminosity: bool,
    #[serde(rename = "shadowCyanRed", with = "mac_number")] pub shadow_cyan_red: f64,
    #[serde(rename = "shadowMagentaGreen", with = "mac_number")] pub shadow_magenta_green: f64,
    #[serde(rename = "shadowYellowBlue", with = "mac_number")] pub shadow_yellow_blue: f64,
}
impl Default for ColorBalanceSettings {
    fn default() -> Self { ColorBalanceSettings { highlight_cyan_red: 0.0, highlight_magenta_green: 0.0, highlight_yellow_blue: 0.0,
        mid_cyan_red: 0.0, mid_magenta_green: 0.0, mid_yellow_blue: 0.0, preserve_luminosity: true,
        shadow_cyan_red: 0.0, shadow_magenta_green: 0.0, shadow_yellow_blue: 0.0 } }
}
impl ColorBalanceSettings {
    fn values(&self) -> [f64; 9] {
        [self.shadow_cyan_red, self.shadow_magenta_green, self.shadow_yellow_blue, self.mid_cyan_red, self.mid_magenta_green,
         self.mid_yellow_blue, self.highlight_cyan_red, self.highlight_magenta_green, self.highlight_yellow_blue]
    }
    pub fn is_valid(&self) -> bool { self.values().iter().all(|v| (-100.0..=100.0).contains(v)) }
    pub fn is_zero(&self) -> bool { self.values().iter().all(|v| *v == 0.0) }
}

/// The Mac's `LayerAdjustment`: legacy scalar HSV fields plus optional range-aware settings, so
/// files written before those settings existed decode and re-encode byte for byte.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LayerAdjustment {
    pub kind: AdjustmentKind,
    #[serde(default, with = "mac_number")] pub hue: f64,
    #[serde(default, with = "mac_number")] pub saturation: f64,
    #[serde(default, with = "mac_number")] pub lightness: f64,
    #[serde(default)] pub colorize: bool,
    #[serde(rename = "hsvSettings", default, skip_serializing_if = "Option::is_none")] pub hsv_settings: Option<HueSaturationSettings>,
    #[serde(default)] pub levels: LevelsSettings,
    #[serde(default)] pub curves: CurvesSettings,
    #[serde(rename = "exposureSettings", default, skip_serializing_if = "Option::is_none")] pub exposure_settings: Option<ExposureSettings>,
    #[serde(rename = "gradientMapSettings", default, skip_serializing_if = "Option::is_none")] pub gradient_map_settings: Option<GradientMapSettings>,
    #[serde(rename = "grainSettings", default, skip_serializing_if = "Option::is_none")] pub grain_settings: Option<GrainSettings>,
    #[serde(rename = "blackWhiteSettings", default, skip_serializing_if = "Option::is_none")] pub black_white_settings: Option<BlackWhiteSettings>,
    #[serde(rename = "colorBalanceSettings", default, skip_serializing_if = "Option::is_none")] pub color_balance_settings: Option<ColorBalanceSettings>,
    #[serde(rename = "blurRadius", default, with = "mac_number_opt", skip_serializing_if = "Option::is_none")] pub blur_radius: Option<f64>,
    #[serde(rename = "motionAngle", default, with = "mac_number_opt", skip_serializing_if = "Option::is_none")] pub motion_angle: Option<f64>,
    #[serde(rename = "motionDistance", default, with = "mac_number_opt", skip_serializing_if = "Option::is_none")] pub motion_distance: Option<f64>,
    #[serde(rename = "noiseAmount", default, with = "mac_number_opt", skip_serializing_if = "Option::is_none")] pub noise_amount: Option<f64>,
    #[serde(rename = "noiseGaussian", default, skip_serializing_if = "Option::is_none")] pub noise_gaussian: Option<bool>,
    #[serde(rename = "noiseMonochromatic", default, skip_serializing_if = "Option::is_none")] pub noise_monochromatic: Option<bool>,
    #[serde(rename = "noiseSeed", default, skip_serializing_if = "Option::is_none")] pub noise_seed: Option<u32>,
}
impl LayerAdjustment {
    pub fn new(kind: AdjustmentKind) -> Self {
        LayerAdjustment { kind, hue: 0.0, saturation: 0.0, lightness: 0.0, colorize: false, hsv_settings: None, levels: LevelsSettings::default(),
            curves: CurvesSettings::default(), exposure_settings: None, gradient_map_settings: None, grain_settings: None,
            black_white_settings: None, color_balance_settings: None, blur_radius: None, motion_angle: None, motion_distance: None,
            noise_amount: None, noise_gaussian: None, noise_monochromatic: None, noise_seed: None }
    }
    pub fn resolved_hsv(&self) -> HueSaturationSettings {
        self.hsv_settings.clone().unwrap_or_else(|| HueSaturationSettings::new(self.hue, self.saturation, self.lightness, self.colorize, ColorRange::Master))
    }
    pub fn exposure(&self) -> ExposureSettings { self.exposure_settings.unwrap_or_default() }
    pub fn gradient_map(&self) -> GradientMapSettings { self.gradient_map_settings.unwrap_or_default() }
    pub fn grain(&self) -> GrainSettings { self.grain_settings.unwrap_or_default() }
    pub fn black_white(&self) -> BlackWhiteSettings { self.black_white_settings.unwrap_or_default() }
    pub fn color_balance(&self) -> ColorBalanceSettings { self.color_balance_settings.unwrap_or_default() }
    pub fn gaussian_radius(&self) -> f64 { self.blur_radius.unwrap_or(10.0) }
    pub fn motion_angle_degrees(&self) -> f64 { self.motion_angle.unwrap_or(0.0) }
    pub fn motion_distance_pixels(&self) -> f64 { self.motion_distance.unwrap_or(10.0) }
    pub fn noise_amount_percent(&self) -> f64 { self.noise_amount.unwrap_or(10.0) }
    pub fn noise_is_gaussian(&self) -> bool { self.noise_gaussian.unwrap_or(false) }
    pub fn noise_is_monochromatic(&self) -> bool { self.noise_monochromatic.unwrap_or(false) }
    pub fn noise_seed_or_zero(&self) -> u32 { self.noise_seed.unwrap_or(0) }
    pub fn is_valid(&self) -> bool {
        self.hue.is_finite() && self.saturation.is_finite() && self.lightness.is_finite() && self.hue.abs() <= 360.0 && self.saturation.abs() <= 100.0 && self.lightness.abs() <= 100.0
            && self.resolved_hsv().is_valid() && self.levels.is_valid() && self.curves.is_valid()
            && self.exposure().is_valid() && self.gradient_map().is_valid() && self.grain().is_valid()
            && self.black_white().is_valid() && self.color_balance().is_valid()
            && (0.1..=250.0).contains(&self.gaussian_radius()) && (-90.0..=90.0).contains(&self.motion_angle_degrees())
            && (1.0..=2000.0).contains(&self.motion_distance_pixels()) && (0.1..=400.0).contains(&self.noise_amount_percent())
    }
    /// Whether applying this adjustment would change nothing (used to skip no-op commits).
    pub fn is_identity(&self) -> bool {
        match self.kind {
            AdjustmentKind::Hsv => self.resolved_hsv().is_identity(),
            AdjustmentKind::Levels => self.levels.is_identity(),
            AdjustmentKind::Curves => self.curves.is_identity(),
            AdjustmentKind::Exposure => self.exposure() == ExposureSettings::default(),
            AdjustmentKind::GradientMap => false,
            AdjustmentKind::Grain => self.grain().amount <= 0.0,
            AdjustmentKind::Invert | AdjustmentKind::BlackWhite | AdjustmentKind::GaussianBlur | AdjustmentKind::MotionBlur | AdjustmentKind::AddNoise => false,
            AdjustmentKind::ColorBalance => self.color_balance().is_zero(),
        }
    }
}
