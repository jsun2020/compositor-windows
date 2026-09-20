use crate::{ids, Sampling};
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
