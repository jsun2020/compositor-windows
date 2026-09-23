use crate::{ids, LayerAdjustment, LayerTransform, ProjectError};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

pub const MANIFEST_FORMAT: &str = "com.compositor.project";
pub const CURRENT_VERSION: u32 = 7;
pub const MAX_SIDE: i64 = 30_000;
pub const MAX_PIXELS: u64 = 100_000_000;
pub const MAX_LAYERS: usize = 10_000;
pub const MAX_NESTING: usize = 64;
pub const MAX_MANIFEST_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_ASSET_BYTES: u64 = 512 * 1024 * 1024;
pub const DEFAULT_RESOLUTION: f64 = 72.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BlendMode {
    #[default]
    #[serde(rename = "Normal")] Normal,
    #[serde(rename = "Multiply")] Multiply,
    #[serde(rename = "Screen")] Screen,
    #[serde(rename = "Overlay")] Overlay,
    #[serde(rename = "Darken")] Darken,
    #[serde(rename = "Lighten")] Lighten,
    #[serde(rename = "Difference")] Difference,
    #[serde(rename = "Color Dodge")] ColorDodge,
    #[serde(rename = "Color Burn")] ColorBurn,
    #[serde(rename = "Hue")] Hue,
    #[serde(rename = "Saturation")] Saturation,
    #[serde(rename = "Color")] Color,
    #[serde(rename = "Luminosity")] Luminosity,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LayerRecord {
    #[serde(with = "ids::upper")] pub id: Uuid,
    pub name: String,
    #[serde(rename = "isVisible")] pub is_visible: bool,
    pub transform: LayerTransform,
    #[serde(rename = "imageFile", default, skip_serializing_if = "Option::is_none")] pub image_file: Option<String>,
    #[serde(rename = "parentID", default, with = "ids::upper_opt", skip_serializing_if = "Option::is_none")] pub parent_id: Option<Uuid>,
    #[serde(rename = "isGroup", default, skip_serializing_if = "Option::is_none")] pub is_group: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub opacity: Option<f64>,
    #[serde(rename = "blendMode", default, skip_serializing_if = "Option::is_none")] pub blend_mode: Option<BlendMode>,
    #[serde(rename = "maskFile", default, skip_serializing_if = "Option::is_none")] pub mask_file: Option<String>,
    #[serde(rename = "maskEnabled", default, skip_serializing_if = "Option::is_none")] pub mask_enabled: Option<bool>,
    #[serde(rename = "maskSourceID", default, with = "ids::upper_opt", skip_serializing_if = "Option::is_none")] pub mask_source_id: Option<Uuid>,
    #[serde(default, with = "adjustment_file", skip_serializing_if = "Option::is_none")] pub adjustment: Option<LayerAdjustment>,
    #[serde(rename = "maskPlacement", default, skip_serializing_if = "Option::is_none")] pub mask_placement: Option<LayerTransform>,
    #[serde(rename = "maskLinked", default, skip_serializing_if = "Option::is_none")] pub mask_linked: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub shape: Option<serde_json::Value>,
}

/// The file form of a layer's adjustment. It differs from the bridge form (plain `LayerAdjustment`
/// serde) in one place only: the Mac declares `HueSaturationSettings.adjustments` and `.bands` as
/// `[ColorRange: _]`, and `ColorRange` is a `String`-backed enum that is not
/// `CodingKeyRepresentable`, so Swift's `Dictionary` encodes each as an unkeyed container of
/// alternating key and value (`["Master", {...}, "Reds", {...}]`) and decodes only that. The TS
/// panels and the GL renderer read those maps as objects, so the conversion happens here, on top
/// of the one `LayerAdjustment` definition, rather than in a second copy of the type. Writing
/// fails (instead of silently writing objects) if the two maps ever stop serializing as objects.
mod adjustment_file {
    use crate::{ColorRange, LayerAdjustment};
    use serde::{de, ser, Deserialize, Deserializer, Serialize, Serializer};
    use serde_json::{Map, Value};

    const HSV: &str = "hsvSettings";
    const MAPS: [&str; 2] = ["adjustments", "bands"];

    /// `[key, value, ...]` in `ColorRange::ALL` order. The Mac's own order is hash-seeded per
    /// process, so byte identity is impossible and a fixed order is the stable choice.
    pub fn serialize<S: Serializer>(value: &Option<LayerAdjustment>, s: S) -> Result<S::Ok, S::Error> {
        let Some(adjustment) = value else { return s.serialize_none() };
        let mut json = serde_json::to_value(adjustment).map_err(ser::Error::custom)?;
        if let Some(hsv) = json.get_mut(HSV) {
            for name in MAPS {
                let Some(Value::Object(mut map)) = hsv.get_mut(name).map(Value::take) else {
                    return Err(ser::Error::custom(format!("{HSV}.{name} is not an object")));
                };
                let mut pairs = Vec::with_capacity(map.len() * 2);
                for range in ColorRange::ALL {
                    let key = serde_json::to_value(range).map_err(ser::Error::custom)?;
                    if let Some(entry) = key.as_str().and_then(|k| map.remove(k)) { pairs.push(key); pairs.push(entry); }
                }
                if !map.is_empty() { return Err(ser::Error::custom(format!("{HSV}.{name} has an unknown range"))); }
                hsv[name] = Value::Array(pairs);
            }
        }
        json.serialize(s)
    }

    /// Accepts the Mac's array form and the object form a 0.3.0 development build wrote. A key
    /// repeated in the array keeps its last value, as Swift's decoder does.
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<LayerAdjustment>, D::Error> {
        let Some(mut json) = Option::<Value>::deserialize(d)? else { return Ok(None) };
        if let Some(hsv) = json.get_mut(HSV).filter(|h| h.is_object()) {
            for name in MAPS {
                let Some(Value::Array(items)) = hsv.get(name) else { continue };
                if items.len() % 2 != 0 { return Err(de::Error::custom(format!("{HSV}.{name} has an odd number of entries"))); }
                let mut map = Map::new();
                for pair in items.chunks(2) {
                    let key = pair[0].as_str().ok_or_else(|| de::Error::custom(format!("{HSV}.{name} has a non-string key")))?;
                    map.insert(key.to_string(), pair[1].clone());
                }
                hsv[name] = Value::Object(map);
            }
        }
        serde_json::from_value(json).map(Some).map_err(de::Error::custom)
    }
}

impl LayerRecord {
    pub fn new(id: Uuid, name: &str, transform: LayerTransform, image_file: Option<String>) -> Self {
        LayerRecord { id, name: name.to_string(), is_visible: true, transform, image_file, parent_id: None,
            is_group: None, opacity: None, blend_mode: None, mask_file: None, mask_enabled: None,
            mask_source_id: None, adjustment: None, mask_placement: None, mask_linked: None, shape: None }
    }
    pub fn image_filename(id: &Uuid) -> String { format!("{}.png", ids::upper_string(id)) }
    pub fn mask_filename(id: &Uuid) -> String { format!("{}.mask.png", ids::upper_string(id)) }
    pub fn is_group(&self) -> bool { self.is_group == Some(true) }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub format: String,
    pub version: u32,
    #[serde(rename = "colorSpace")] pub color_space: String,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub resolution: Option<f64>,
    #[serde(rename = "documentID", with = "ids::upper")] pub document_id: Uuid,
    pub width: i64,
    pub height: i64,
    #[serde(rename = "activeLayerID", default, with = "ids::upper_opt", skip_serializing_if = "Option::is_none")] pub active_layer_id: Option<Uuid>,
    pub layers: Vec<LayerRecord>,
}

#[derive(Deserialize)]
struct Header { format: String, version: u32 }

impl Manifest {
    pub fn new(document_id: Uuid, width: i64, height: i64, active_layer_id: Option<Uuid>, layers: Vec<LayerRecord>) -> Self {
        Manifest { format: MANIFEST_FORMAT.into(), version: CURRENT_VERSION, color_space: "sRGB".into(),
            resolution: None, document_id, width, height, active_layer_id, layers }
    }

    /// Header check first so an unsupported version reports its number, then full decode, then validation.
    pub fn parse(json: &str) -> Result<Manifest, ProjectError> {
        if json.len() > MAX_MANIFEST_BYTES { return Err(ProjectError::TooLarge); }
        let header: Header = serde_json::from_str(json).map_err(|_| ProjectError::Invalid)?;
        if header.format != MANIFEST_FORMAT { return Err(ProjectError::Invalid); }
        if !(1..=CURRENT_VERSION).contains(&header.version) { return Err(ProjectError::Version(header.version)); }
        let manifest: Manifest = serde_json::from_str(json).map_err(|_| ProjectError::Invalid)?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Sorted keys, pretty printed, size-checked.
    pub fn to_json_pretty(&self) -> Result<String, ProjectError> {
        self.validate()?;
        let value = serde_json::to_value(self).map_err(|_| ProjectError::Encode)?;
        let text = serde_json::to_string_pretty(&value).map_err(|_| ProjectError::Encode)?;
        if text.len() > MAX_MANIFEST_BYTES { return Err(ProjectError::TooLarge); }
        Ok(text)
    }

    pub fn validate(&self) -> Result<(), ProjectError> {
        use ProjectError::*;
        if self.format != MANIFEST_FORMAT { return Err(Invalid); }
        if !(1..=CURRENT_VERSION).contains(&self.version) { return Err(Version(self.version)); }
        if self.color_space != "sRGB" { return Err(Invalid); }
        if let Some(r) = self.resolution {
            if !r.is_finite() || !(1.0..=9600.0).contains(&r) { return Err(Invalid); }
        }
        if !(1..=MAX_SIDE).contains(&self.width) || !(1..=MAX_SIDE).contains(&self.height) || self.layers.len() > MAX_LAYERS {
            return Err(TooLarge);
        }
        for layer in &self.layers {
            if let Some(adjustment) = &layer.adjustment {
                if self.version < 7 || layer.is_group() || layer.image_file.is_some() || !adjustment.is_valid() { return Err(Invalid); }
            }
            if let Some(mask) = &layer.mask_file {
                let needed = if layer.is_group() { 6 } else { 4 };
                if self.version < needed || *mask != LayerRecord::mask_filename(&layer.id) { return Err(Invalid); }
            }
            if layer.mask_enabled.is_some() && layer.mask_file.is_none() { return Err(Invalid); }
            if let Some(p) = &layer.mask_placement {
                if !p.is_valid() || layer.mask_file.is_none() { return Err(Invalid); }
            }
            let opacity = layer.opacity.unwrap_or(1.0);
            let blend = layer.blend_mode.unwrap_or_default();
            if !opacity.is_finite() || !(0.0..=1.0).contains(&opacity) { return Err(Invalid); }
            if self.version < 3 && (opacity != 1.0 || blend != BlendMode::Normal) { return Err(Invalid); }
            if layer.is_group() && (opacity != 1.0 || blend != BlendMode::Normal) { return Err(Invalid); }
        }
        validate_hierarchy(&self.layers)?;
        validate_clipping(&self.layers)?;
        if self.version < 5 && self.layers.iter().any(|l| l.mask_source_id.is_some()) { return Err(Invalid); }
        if self.version == 1 && self.layers.iter().any(|l| l.parent_id.is_some() || l.is_group()) { return Err(Invalid); }
        let mut ids = HashSet::new();
        for layer in &self.layers {
            if !ids.insert(layer.id) || !layer.transform.is_valid() || layer.name.trim().is_empty() || layer.name.len() > 16_384 {
                return Err(Invalid);
            }
            if let Some(file) = &layer.image_file {
                if *file != LayerRecord::image_filename(&layer.id) { return Err(Invalid); }
            }
        }
        if let Some(active) = self.active_layer_id {
            if !ids.contains(&active) { return Err(Invalid); }
        }
        Ok(())
    }
}

pub fn validate_hierarchy(layers: &[LayerRecord]) -> Result<(), ProjectError> {
    let mut by_id: HashMap<Uuid, &LayerRecord> = HashMap::new();
    for layer in layers {
        if by_id.insert(layer.id, layer).is_some() || (layer.is_group() && layer.image_file.is_some()) {
            return Err(ProjectError::Invalid);
        }
    }
    for layer in layers {
        let mut seen = HashSet::from([layer.id]);
        let mut parent = layer.parent_id;
        while let Some(id) = parent {
            if seen.len() > MAX_NESTING || !seen.insert(id) { return Err(ProjectError::Invalid); }
            let node = by_id.get(&id).ok_or(ProjectError::Invalid)?;
            if !node.is_group() { return Err(ProjectError::Invalid); }
            parent = node.parent_id;
        }
        if layer.is_group() && seen.len() > MAX_NESTING { return Err(ProjectError::Invalid); }
    }
    Ok(())
}

pub fn validate_clipping(layers: &[LayerRecord]) -> Result<(), ProjectError> {
    let mut records: HashMap<Uuid, &LayerRecord> = HashMap::new();
    for layer in layers {
        if records.insert(layer.id, layer).is_some() { return Err(ProjectError::Invalid); }
    }
    for layer in layers {
        let mut path = HashSet::new();
        let mut current = Some(layer.id);
        while let Some(id) = current {
            if path.len() >= 256 || !path.insert(id) { return Err(ProjectError::Invalid); }
            let record = records.get(&id).ok_or(ProjectError::Invalid)?;
            if let Some(source) = record.mask_source_id {
                if record.is_group() { return Err(ProjectError::Invalid); }
                let src = records.get(&source).ok_or(ProjectError::Invalid)?;
                if src.is_group() || src.adjustment.is_some() { return Err(ProjectError::Invalid); }
            }
            current = record.mask_source_id;
        }
    }
    Ok(())
}

/// Layers whose every ancestor is visible, in manifest order. Groups are included.
pub fn visible_layers(layers: &[LayerRecord]) -> Vec<&LayerRecord> {
    let by_id: HashMap<Uuid, &LayerRecord> = layers.iter().map(|l| (l.id, l)).collect();
    layers.iter().filter(|layer| {
        let mut node = Some(*layer);
        let mut steps = 0;
        while let Some(n) = node {
            if !n.is_visible { return false; }
            steps += 1;
            if steps > MAX_NESTING + 1 { return false; }
            node = n.parent_id.and_then(|p| by_id.get(&p).copied());
        }
        true
    }).collect()
}
