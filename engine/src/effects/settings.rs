//! Layer effects as Compositor for Mac 1.2.10 reads and writes them (Document/LayerEffects.swift:5-210,
//! R 2.1): six optional effects, each a struct whose members are all REQUIRED on decode except
//! `enabled`, because Swift's synthesized `Codable` decodes a non-optional `var` with `decode`,
//! whatever its initial value (R 0). Written as the Mac writes them: an absent optional is
//! omitted, a whole-number Double carries no fraction (`mac_number`), and the manifest writer
//! sorts the keys.
//!
//! Unknown keys, inside an effect or beside the six: the Mac's synthesized decode reads its own
//! keys and ignores the rest, so they never make a file invalid there, and its encode drops them.
//! This port keeps them (`unknown`, flattened), as it keeps unknown layer and manifest keys
//! (3.5a: nothing read is lost), and names them in the notice, since a later Mac may draw them.
use crate::adjust::settings::mac_number;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// The largest padded effects image the Mac draws: `DocumentLimits.maxSurfacePixels`, checked in
/// `LayerEffectsRenderer.render` (LayerEffects.swift:441). Above it `cached` turns the throw into
/// nil (:405) and the layer draws without its effects (1.2.10 delta 2.3).
pub const EFFECTS_SURFACE_LIMIT: u64 = 200_000_000;

fn unit(v: f64) -> bool { v.is_finite() && (0.0..=1.0).contains(&v) }
fn colour(red: f64, green: f64, blue: f64) -> bool { unit(red) && unit(green) && unit(blue) }
/// `ShadowEffect.isValid` and `InnerShadowEffect.isValid` (LayerEffects.swift:44-49, 83-88).
fn shadow_valid(angle: f64, distance: f64, blur: f64) -> bool {
    [angle, distance, blur].iter().all(|v| v.is_finite()) && (-360.0..=360.0).contains(&angle)
        && (0.0..=5000.0).contains(&distance) && (0.0..=500.0).contains(&blur)
}
fn shown(enabled: Option<bool>) -> bool { enabled != Some(false) }

/// A line around what the layer shows, outside its edge or inside it (LayerEffects.swift:6-22).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StrokeEffect {
    /// Absent until the user toggles the effect; absent means shown (`isEnabled`).
    #[serde(default, skip_serializing_if = "Option::is_none")] pub enabled: Option<bool>,
    #[serde(with = "mac_number")] pub size: f64,
    #[serde(with = "mac_number")] pub red: f64,
    #[serde(with = "mac_number")] pub green: f64,
    #[serde(with = "mac_number")] pub blue: f64,
    #[serde(with = "mac_number")] pub opacity: f64,
    pub inside: bool,
    #[serde(flatten)] pub unknown: Map<String, Value>,
}

/// The layer's shape behind it, offset and softened: Drop Shadow (LayerEffects.swift:25-50).
/// `angle` is where the light comes from, degrees counterclockwise from the right.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShadowEffect {
    #[serde(default, skip_serializing_if = "Option::is_none")] pub enabled: Option<bool>,
    #[serde(with = "mac_number")] pub angle: f64,
    #[serde(with = "mac_number")] pub distance: f64,
    #[serde(with = "mac_number")] pub blur: f64,
    #[serde(with = "mac_number")] pub red: f64,
    #[serde(with = "mac_number")] pub green: f64,
    #[serde(with = "mac_number")] pub blue: f64,
    #[serde(with = "mac_number")] pub opacity: f64,
    #[serde(flatten)] pub unknown: Map<String, Value>,
}

/// A flat colour over everything the layer shows (LayerEffects.swift:53-64).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ColorOverlayEffect {
    #[serde(default, skip_serializing_if = "Option::is_none")] pub enabled: Option<bool>,
    #[serde(with = "mac_number")] pub red: f64,
    #[serde(with = "mac_number")] pub green: f64,
    #[serde(with = "mac_number")] pub blue: f64,
    #[serde(with = "mac_number")] pub opacity: f64,
    #[serde(flatten)] pub unknown: Map<String, Value>,
}

/// A shadow cast inside the layer's own edges (LayerEffects.swift:67-89).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InnerShadowEffect {
    #[serde(default, skip_serializing_if = "Option::is_none")] pub enabled: Option<bool>,
    #[serde(with = "mac_number")] pub angle: f64,
    #[serde(with = "mac_number")] pub distance: f64,
    #[serde(with = "mac_number")] pub blur: f64,
    #[serde(with = "mac_number")] pub red: f64,
    #[serde(with = "mac_number")] pub green: f64,
    #[serde(with = "mac_number")] pub blue: f64,
    #[serde(with = "mac_number")] pub opacity: f64,
    #[serde(flatten)] pub unknown: Map<String, Value>,
}

/// A soft glow around the outside of what the layer shows (LayerEffects.swift:92-106).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OuterGlowEffect {
    #[serde(default, skip_serializing_if = "Option::is_none")] pub enabled: Option<bool>,
    #[serde(with = "mac_number")] pub size: f64,
    #[serde(with = "mac_number")] pub red: f64,
    #[serde(with = "mac_number")] pub green: f64,
    #[serde(with = "mac_number")] pub blue: f64,
    #[serde(with = "mac_number")] pub opacity: f64,
    #[serde(flatten)] pub unknown: Map<String, Value>,
}

/// A glow inside the layer's own edges, from its boundary inward (LayerEffects.swift:109-123).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InnerGlowEffect {
    #[serde(default, skip_serializing_if = "Option::is_none")] pub enabled: Option<bool>,
    #[serde(with = "mac_number")] pub size: f64,
    #[serde(with = "mac_number")] pub red: f64,
    #[serde(with = "mac_number")] pub green: f64,
    #[serde(with = "mac_number")] pub blue: f64,
    #[serde(with = "mac_number")] pub opacity: f64,
    #[serde(flatten)] pub unknown: Map<String, Value>,
}

/// What a layer draws around itself (LayerEffects.swift:127-210). The Mac never writes it empty
/// (`setEffects` stores nil, :308), but an empty object reads and writes back as `{}`.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayerEffects {
    #[serde(default, skip_serializing_if = "Option::is_none")] pub stroke: Option<StrokeEffect>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub shadow: Option<ShadowEffect>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub color_overlay: Option<ColorOverlayEffect>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub inner_shadow: Option<InnerShadowEffect>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub outer_glow: Option<OuterGlowEffect>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub inner_glow: Option<InnerGlowEffect>,
    #[serde(flatten)] pub unknown: Map<String, Value>,
}

impl StrokeEffect {
    pub fn is_valid(&self) -> bool { self.size.is_finite() && (0.0..=500.0).contains(&self.size) && unit(self.opacity) && colour(self.red, self.green, self.blue) }
}
impl ShadowEffect {
    pub fn is_valid(&self) -> bool { shadow_valid(self.angle, self.distance, self.blur) && unit(self.opacity) && colour(self.red, self.green, self.blue) }
}
impl ColorOverlayEffect {
    pub fn is_valid(&self) -> bool { unit(self.opacity) && colour(self.red, self.green, self.blue) }
}
impl InnerShadowEffect {
    pub fn is_valid(&self) -> bool { shadow_valid(self.angle, self.distance, self.blur) && unit(self.opacity) && colour(self.red, self.green, self.blue) }
}
impl OuterGlowEffect {
    pub fn is_valid(&self) -> bool { self.size.is_finite() && (0.0..=500.0).contains(&self.size) && unit(self.opacity) && colour(self.red, self.green, self.blue) }
}
impl InnerGlowEffect {
    pub fn is_valid(&self) -> bool { self.size.is_finite() && (0.0..=500.0).contains(&self.size) && unit(self.opacity) && colour(self.red, self.green, self.blue) }
}

impl LayerEffects {
    /// No effect at all (`isEmpty`); unknown keys do not count.
    pub fn is_empty(&self) -> bool {
        self.stroke.is_none() && self.shadow.is_none() && self.color_overlay.is_none()
            && self.inner_shadow.is_none() && self.outer_glow.is_none() && self.inner_glow.is_none()
    }
    /// Every effect present is valid (`isValid`, LayerEffects.swift:135-139).
    pub fn is_valid(&self) -> bool {
        self.stroke.as_ref().map_or(true, StrokeEffect::is_valid) && self.shadow.as_ref().map_or(true, ShadowEffect::is_valid)
            && self.color_overlay.as_ref().map_or(true, ColorOverlayEffect::is_valid)
            && self.inner_shadow.as_ref().map_or(true, InnerShadowEffect::is_valid)
            && self.outer_glow.as_ref().map_or(true, OuterGlowEffect::is_valid)
            && self.inner_glow.as_ref().map_or(true, InnerGlowEffect::is_valid)
    }
    /// Only the shown effects (`visible`, LayerEffects.swift:202-209): `enabled: false` hides one.
    pub fn visible(&self) -> LayerEffects {
        LayerEffects {
            stroke: self.stroke.clone().filter(|e| shown(e.enabled)),
            shadow: self.shadow.clone().filter(|e| shown(e.enabled)),
            color_overlay: self.color_overlay.clone().filter(|e| shown(e.enabled)),
            inner_shadow: self.inner_shadow.clone().filter(|e| shown(e.enabled)),
            outer_glow: self.outer_glow.clone().filter(|e| shown(e.enabled)),
            inner_glow: self.inner_glow.clone().filter(|e| shown(e.enabled)),
            unknown: Map::new(),
        }
    }
    /// What the Mac draws, or None when it draws the layer plainly: the shown effects, when there
    /// is one and every one is valid (`LayerEffectsRenderer.cached`, LayerEffects.swift:403-408).
    pub fn drawn(&self) -> Option<LayerEffects> {
        let visible = self.visible();
        (!visible.is_empty() && visible.is_valid()).then_some(visible)
    }
    /// Layer pixels the effects need on every side (`LayerEffectsRenderer.margin`,
    /// LayerEffects.swift:421-432): an outside stroke's size, a drop shadow's distance plus three
    /// blurs, three outer-glow sizes, whichever is largest, rounded up, plus 2. An inside stroke,
    /// colour overlay, inner shadow and inner glow add nothing.
    pub fn margin(&self) -> u32 {
        let effects = self.visible();
        let mut margin = 0.0f64;
        if let Some(s) = &effects.stroke { if !s.inside { margin = margin.max(s.size); } }
        if let Some(s) = &effects.shadow { margin = margin.max(s.distance + s.blur * 3.0); }
        if let Some(g) = &effects.outer_glow { margin = margin.max(g.size * 3.0); }
        margin.ceil() as u32 + 2
    }
    /// Whether the padded image for a `width` x `height` layer stays within
    /// `EFFECTS_SURFACE_LIMIT` (LayerEffects.swift:440-441).
    pub fn fits(&self, width: u32, height: u32) -> bool {
        let m = 2 * self.margin() as u64;
        (width as u64 + m) * (height as u64 + m) <= EFFECTS_SURFACE_LIMIT
    }
    /// Every length (stroke size, shadow distances and blurs, glow sizes) of each valid effect
    /// times `factor`, kept within the Mac's valid ranges so the effects stay drawn. An invalid
    /// effect is carried as it is: clamped, it would turn valid, and a layer the Mac draws plainly
    /// would suddenly draw all its effects.
    pub fn scaled(&self, factor: f64) -> LayerEffects {
        let mut e = self.clone();
        if let Some(s) = e.stroke.as_mut().filter(|s| s.is_valid()) { s.size = (s.size * factor).min(500.0); }
        if let Some(s) = e.shadow.as_mut().filter(|s| s.is_valid()) { s.distance = (s.distance * factor).min(5000.0); s.blur = (s.blur * factor).min(500.0); }
        if let Some(s) = e.inner_shadow.as_mut().filter(|s| s.is_valid()) { s.distance = (s.distance * factor).min(5000.0); s.blur = (s.blur * factor).min(500.0); }
        if let Some(g) = e.outer_glow.as_mut().filter(|g| g.is_valid()) { g.size = (g.size * factor).min(500.0); }
        if let Some(g) = e.inner_glow.as_mut().filter(|g| g.is_valid()) { g.size = (g.size * factor).min(500.0); }
        e
    }
    /// The effects carried onto a layer whose pixels Image Size redraws upright on the new canvas's
    /// grid (Phase 3.5c ruling). `linear` takes one old layer pixel onto the new grid
    /// ([a, b, c, d]: x' = a x + c y, y' = b x + d y). Lengths scale by the square root of its
    /// area, and each shadow's offset goes through it whole, so a rotation or flip baked into the
    /// pixels leaves the shadow falling where it fell. An invalid effect is carried as it is, as
    /// `scaled` carries it.
    pub fn resampled(&self, linear: [f64; 4]) -> LayerEffects {
        let [a, b, c, d] = linear;
        let mut e = self.scaled((a * d - b * c).abs().sqrt());
        // Whole numbers stay whole: the trigonometry leaves 89.99999999999999 where 90 was.
        let tidy = |v: f64| (v * 1e9).round() / 1e9;
        let aim = |angle: f64, distance: f64| -> (f64, f64) {
            if distance == 0.0 { return (angle, 0.0); }
            let radians = angle * std::f64::consts::PI / 180.0;
            let (ox, oy) = (-radians.cos() * distance, radians.sin() * distance);
            let (nx, ny) = (a * ox + c * oy, b * ox + d * oy);
            (tidy(ny.atan2(-nx).to_degrees()), tidy(nx.hypot(ny)).min(5000.0))
        };
        if let (Some(new), Some(old)) = (&mut e.shadow, self.shadow.as_ref().filter(|s| s.is_valid())) { (new.angle, new.distance) = aim(old.angle, old.distance); }
        if let (Some(new), Some(old)) = (&mut e.inner_shadow, self.inner_shadow.as_ref().filter(|s| s.is_valid())) { (new.angle, new.distance) = aim(old.angle, old.distance); }
        e
    }
    /// Keys a later version of Compositor wrote, here or inside an effect.
    pub fn has_unknown(&self) -> bool {
        !self.unknown.is_empty()
            || self.stroke.as_ref().is_some_and(|e| !e.unknown.is_empty())
            || self.shadow.as_ref().is_some_and(|e| !e.unknown.is_empty())
            || self.color_overlay.as_ref().is_some_and(|e| !e.unknown.is_empty())
            || self.inner_shadow.as_ref().is_some_and(|e| !e.unknown.is_empty())
            || self.outer_glow.as_ref().is_some_and(|e| !e.unknown.is_empty())
            || self.inner_glow.as_ref().is_some_and(|e| !e.unknown.is_empty())
    }
}
