use crate::{ids, AdjustmentKind, BlendMode, FilterParams, LayerAdjustment, LayerTransform, Point, Sampling};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Command {
    AddBlankLayer,
    RenameLayer { #[serde(with = "ids::upper")] id: Uuid, name: String },
    SetLayerVisible { #[serde(with = "ids::upper")] id: Uuid, visible: bool },
    DeleteLayer { #[serde(with = "ids::upper")] id: Uuid },
    SetActiveLayer { #[serde(with = "ids::upper_opt")] id: Option<Uuid> },
    CanvasSize { width: u32, height: u32, anchor: u8, fill: Option<[f64; 3]> },
    Crop { x: f64, y: f64, width: f64, height: f64 },
    ImageSize { width: u32, height: u32, resolution: f64, sampling: Sampling },
    FlipCanvas { horizontal: bool },
    SetLayerOpacity { #[serde(with = "ids::upper")] id: Uuid, opacity: f64 },
    SetLayersOpacity { #[serde(deserialize_with = "deserialize_ids", serialize_with = "serialize_ids")] ids: Vec<Uuid>, opacity: f64 },
    SetLayerBlendMode { #[serde(with = "ids::upper")] id: Uuid, mode: BlendMode },
    AddGroup,
    GroupLayers { #[serde(deserialize_with = "deserialize_ids", serialize_with = "serialize_ids")] ids: Vec<Uuid> },
    PlaceLayer { #[serde(with = "ids::upper")] id: Uuid, #[serde(default, with = "ids::upper_opt")] parent: Option<Uuid>, #[serde(default, with = "ids::upper_opt")] above: Option<Uuid>, #[serde(default, rename = "atBottom")] at_bottom: bool },
    MoveLayerBy { #[serde(with = "ids::upper")] id: Uuid, offset: i32 },
    DuplicateLayer { #[serde(with = "ids::upper")] id: Uuid },
    DuplicateLayerTo { #[serde(with = "ids::upper")] id: Uuid, #[serde(default, with = "ids::upper_opt")] parent: Option<Uuid>, #[serde(default, with = "ids::upper_opt")] above: Option<Uuid>, #[serde(default, rename = "atBottom")] at_bottom: bool },
    DuplicateLayerTransformed { #[serde(with = "ids::upper")] id: Uuid, transform: LayerTransform },
    DeleteLayers { #[serde(deserialize_with = "deserialize_ids", serialize_with = "serialize_ids")] ids: Vec<Uuid>, #[serde(default)] bake: bool },
    SetLayerTransform { #[serde(with = "ids::upper")] id: Uuid, transform: LayerTransform },
    TransformLayers { #[serde(deserialize_with = "deserialize_ids", serialize_with = "serialize_ids")] ids: Vec<Uuid>, #[serde(rename = "box")] bounds: LayerTransform, draft: LayerTransform },
    FlipLayers { #[serde(deserialize_with = "deserialize_ids", serialize_with = "serialize_ids")] ids: Vec<Uuid>, horizontal: bool },
    NudgeLayers { #[serde(deserialize_with = "deserialize_ids", serialize_with = "serialize_ids")] ids: Vec<Uuid>, dx: f64, dy: f64 },
    DistortLayer { #[serde(with = "ids::upper")] id: Uuid, transform: LayerTransform, corners: [Point; 4] },
    DistortLayers { #[serde(deserialize_with = "deserialize_ids", serialize_with = "serialize_ids")] ids: Vec<Uuid>, #[serde(rename = "box")] bounds: LayerTransform, draft: LayerTransform, corners: [Point; 4] },
    SetMaskPlacement { #[serde(with = "ids::upper")] id: Uuid, placement: LayerTransform },
    AddMask { #[serde(with = "ids::upper")] id: Uuid, revealing: bool },
    DeleteMask { #[serde(with = "ids::upper")] id: Uuid },
    SetMaskEnabled { #[serde(with = "ids::upper")] id: Uuid, enabled: bool },
    SetMaskLinked { #[serde(with = "ids::upper")] id: Uuid, linked: bool },
    InvertMask { #[serde(with = "ids::upper")] id: Uuid },
    FillMask { #[serde(with = "ids::upper")] id: Uuid, white: bool },
    BlurMask { #[serde(with = "ids::upper")] id: Uuid, radius: f64 },
    CopyMask { #[serde(with = "ids::upper")] from: Uuid, #[serde(with = "ids::upper")] to: Uuid },
    ToggleClipping { #[serde(with = "ids::upper")] id: Uuid },
    ReleaseClipping { #[serde(with = "ids::upper")] id: Uuid },
    LinkMask { #[serde(with = "ids::upper")] source: Uuid, #[serde(with = "ids::upper")] target: Uuid },
    MergeLayers { #[serde(deserialize_with = "deserialize_ids", serialize_with = "serialize_ids")] ids: Vec<Uuid> },
    ApplyAdjustment { #[serde(with = "ids::upper")] id: Uuid, adjustment: LayerAdjustment },
    InvertPixels { #[serde(with = "ids::upper")] id: Uuid, #[serde(default)] mask: bool },
    ApplyFilter { #[serde(with = "ids::upper")] id: Uuid, params: FilterParams },
    AddAdjustmentLayer { kind: AdjustmentKind, #[serde(default)] seed: u32, #[serde(default)] shadows: Option<[f64; 3]>, #[serde(default)] highlights: Option<[f64; 3]> },
    SetAdjustment { #[serde(with = "ids::upper")] id: Uuid, adjustment: LayerAdjustment },
}

impl Command {
    pub fn action_name(&self) -> &'static str {
        match self {
            Command::AddBlankLayer => "New Layer",
            Command::RenameLayer { .. } => "Rename Layer",
            Command::SetLayerVisible { .. } => "Layer Visibility",
            Command::DeleteLayer { .. } => "Delete Layer",
            Command::SetActiveLayer { .. } => "Select Layer",
            Command::CanvasSize { .. } => "Canvas Size",
            Command::Crop { .. } => "Crop",
            Command::ImageSize { .. } => "Image Size",
            Command::FlipCanvas { horizontal: true } => "Flip Canvas Horizontal",
            Command::FlipCanvas { horizontal: false } => "Flip Canvas Vertical",
            Command::SetLayerOpacity { .. } => "Layer Opacity",
            Command::SetLayersOpacity { .. } => "Layer Opacity",
            Command::SetLayerBlendMode { .. } => "Layer Blend Mode",
            Command::AddGroup => "New Folder",
            Command::GroupLayers { .. } => "Group Layers",
            Command::PlaceLayer { .. } => "Move Layer",
            Command::MoveLayerBy { .. } => "Reorder Layers",
            Command::DuplicateLayer { .. } => "Duplicate Layer",
            Command::DuplicateLayerTo { .. } => "Duplicate Layer",
            Command::DuplicateLayerTransformed { .. } => "Duplicate Layer",
            Command::DeleteLayers { .. } => "Delete Layers",
            Command::SetLayerTransform { .. } => "Transform Layer",
            Command::TransformLayers { .. } => "Transform Layers",
            Command::FlipLayers { .. } => "Flip Layers",
            Command::NudgeLayers { .. } => "Nudge",
            Command::DistortLayer { .. } => "Distort",
            Command::DistortLayers { .. } => "Distort Layers",
            Command::SetMaskPlacement { .. } => "Transform Layer Mask",
            Command::AddMask { .. } => "Add Mask",
            Command::DeleteMask { .. } => "Delete Layer Mask",
            Command::SetMaskEnabled { .. } => "Enable Layer Mask",
            Command::SetMaskLinked { .. } => "Link Layer Mask",
            Command::InvertMask { .. } => "Invert Mask",
            Command::FillMask { .. } => "Fill Mask",
            Command::BlurMask { .. } => "Blur Mask",
            Command::CopyMask { .. } => "Copy Layer Mask",
            Command::ToggleClipping { .. } => "Clipping Mask",
            Command::ReleaseClipping { .. } => "Release Clipping Mask",
            Command::LinkMask { .. } => "Create Clipping Mask",
            Command::MergeLayers { .. } => "Merge Layers",
            Command::ApplyAdjustment { adjustment, .. } => adjustment.kind.name(),
            Command::InvertPixels { mask: false, .. } => "Invert",
            Command::InvertPixels { mask: true, .. } => "Invert Mask",
            Command::ApplyFilter { params, .. } => params.name(),
            Command::AddAdjustmentLayer { kind, .. } => kind.new_action_name(),
            Command::SetAdjustment { adjustment, .. } => adjustment.kind.edit_action_name(),
        }
    }
}

/// What a command changed, so the renderer re-syncs only what it must.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Dirty {
    /// Layer list, names, order, visibility or transforms changed.
    pub structure: bool,
    /// Canvas size or resolution changed.
    pub canvas: bool,
    /// Layers whose pixels were replaced.
    #[serde(serialize_with = "serialize_ids", deserialize_with = "deserialize_ids")]
    pub layers: Vec<Uuid>,
}

fn serialize_ids<S: serde::Serializer>(ids: &[Uuid], s: S) -> Result<S::Ok, S::Error> {
    use serde::Serialize;
    ids.iter().map(ids::upper_string).collect::<Vec<_>>().serialize(s)
}
fn deserialize_ids<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<Uuid>, D::Error> {
    use serde::Deserialize;
    let v: Vec<String> = Vec::deserialize(d)?;
    v.iter().map(|t| Uuid::parse_str(t).map_err(serde::de::Error::custom)).collect()
}

impl Dirty {
    pub fn everything() -> Dirty { Dirty { structure: true, canvas: true, layers: vec![] } }
    pub fn structure() -> Dirty { Dirty { structure: true, ..Default::default() } }
}
