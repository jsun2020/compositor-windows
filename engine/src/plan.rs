use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Coverage {
    #[serde(with = "ids::upper")] pub layer_id: Uuid,
    pub mask_revision: u64,
    pub placement: LayerTransform,
    pub corners: Option<[Point; 4]>,
    pub width: u32,
    pub height: u32,
    pub background: u8,
    pub nearest: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LayerDraw {
    #[serde(with = "ids::upper")] pub id: Uuid,
    pub transform: LayerTransform,
    pub corners: Option<[Point; 4]>,
    pub pixels_width: u32,
    pub pixels_height: u32,
    pub pixels_revision: u64,
    pub opacity: f64,
    pub blend: BlendMode,
    /// An adjustment layer whose own mode is not Normal: the Mac blends its result over the
    /// original at full coverage, both made opaque, in `blend` (its Core Graphics mode), then puts
    /// the ORIGINAL alpha back (LiveMaskRenderer.swift:24-46; BrushPixels.c:23-45). `blend` alone
    /// cannot say this: the eight Core-Image-only modes arrive there as Normal. False for every
    /// other draw.
    pub keeps_alpha: bool,
    pub coverages: Vec<Coverage>,
    #[serde(with = "ids::upper_opt")] pub clip: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")] pub adjustment: Option<LayerAdjustment>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PlanNode {
    Layer { draw: LayerDraw },
    Stack { base: LayerDraw, children: Vec<LayerDraw>, #[serde(rename = "folderCoverages")] folder_coverages: Vec<Coverage> },
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderPlan { pub nodes: Vec<PlanNode>, pub sources: Vec<LayerDraw> }

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PreviewEdit {
    Layer { #[serde(with = "ids::upper")] id: Uuid, draft: LayerTransform, #[serde(default)] corners: Option<[Point; 4]> },
    Group { #[serde(deserialize_with = "deserialize_ids")] ids: Vec<Uuid>, #[serde(rename = "box")] bounds: LayerTransform, draft: LayerTransform, #[serde(default)] corners: Option<[Point; 4]> },
    Mask { #[serde(with = "ids::upper")] id: Uuid, draft: LayerTransform },
    Adjustment { #[serde(with = "ids::upper")] id: Uuid, adjustment: LayerAdjustment },
}

fn deserialize_ids<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<Uuid>, D::Error> {
    let v: Vec<String> = Vec::deserialize(d)?;
    v.iter().map(|t| Uuid::parse_str(t).map_err(serde::de::Error::custom)).collect()
}

/// The layer's transform and distortion corners as the pending edit shows them.
pub fn displayed_transform(layer: &Layer, edit: Option<&PreviewEdit>) -> (LayerTransform, Option<[Point; 4]>) {
    match edit {
        Some(PreviewEdit::Layer { id, draft, corners }) if *id == layer.id => (*draft, *corners),
        Some(PreviewEdit::Group { ids, bounds, draft, corners }) if ids.contains(&layer.id) => {
            let moved = layer.transform.following(bounds, draft);
            let carried = corners.map(|c| Homography::carried(&moved, draft, &c)).filter(Homography::is_usable);
            (moved, carried)
        }
        _ => (layer.transform, None),
    }
}

/// Where a mask sits while its layer is being distorted (a corners edit pending): a mask with
/// an explicit placement stays put; a covering, linked mask has no separate placement to report.
fn placement_under_distortion(layer: &Layer, mask: &Mask) -> Option<LayerTransform> {
    if mask.is_linked() && mask.placement.is_none() { None } else { Some(mask.placement.unwrap_or(layer.transform)) }
}

/// Where the layer's mask shows: nil while it covers the (displayed) layer rectangle.
pub fn displayed_mask_placement(layer: &Layer, edit: Option<&PreviewEdit>) -> Option<LayerTransform> {
    let mask = layer.mask.as_ref()?;
    match edit {
        Some(PreviewEdit::Mask { id, draft }) if *id == layer.id => {
            if draft.same_placement(&layer.transform) { None } else { Some(*draft) }
        }
        Some(PreviewEdit::Layer { id, draft, corners }) if *id == layer.id => {
            if corners.is_some() { placement_under_distortion(layer, mask) }
            else { mask.follow(&layer.transform, draft) }
        }
        Some(PreviewEdit::Group { ids, bounds, draft, corners }) if ids.contains(&layer.id) => {
            if corners.is_some() { placement_under_distortion(layer, mask) }
            else { mask.follow(&layer.transform, &layer.transform.following(bounds, draft)) }
        }
        _ => mask.placement,
    }
}

/// The layer's adjustment as the pending edit shows it. An edit that fails `is_valid` shows the
/// stored adjustment instead: the table builders index `ranges[channel]` and a curve's
/// neighbouring points, so a malformed one from a caller would trap the wasm instance.
pub fn displayed_adjustment(layer: &Layer, edit: Option<&PreviewEdit>) -> Option<LayerAdjustment> {
    match edit {
        Some(PreviewEdit::Adjustment { id, adjustment }) if *id == layer.id && adjustment.is_valid() => Some(adjustment.clone()),
        _ => layer.extra.adjustment.clone(),
    }
}

fn own_coverage(layer: &Layer, edit: Option<&PreviewEdit>) -> Option<Coverage> {
    let mask = layer.mask.as_ref()?;
    if !mask.enabled { return None; }
    let (transform, corners) = displayed_transform(layer, edit);
    let placement = displayed_mask_placement(layer, edit);
    // A mask covering its layer takes the layer's distortion; a placed mask keeps its affine placement.
    let corners = if placement.is_none() { corners } else { None };
    Some(Coverage {
        layer_id: layer.id, mask_revision: layer.mask_revision,
        placement: placement.unwrap_or(transform), corners,
        width: mask.pixels.width, height: mask.pixels.height,
        background: mask.background(), nearest: transform.sampling == Sampling::Nearest,
    })
}

fn folder_coverages(by_id: &HashMap<Uuid, &Layer>, layer: &Layer, edit: Option<&PreviewEdit>) -> Vec<Coverage> {
    let mut result = Vec::new();
    let mut parent = layer.parent_id; let mut depth = 0;
    while let Some(pid) = parent {
        if depth >= MAX_NESTING { break; }
        let Some(folder) = by_id.get(&pid) else { break; };
        if let Some(c) = own_coverage(folder, edit) { result.push(c); }
        parent = folder.parent_id; depth += 1;
    }
    result
}

/// A layer's drawn opacity: its own times every enclosing folder's, up to 64 levels
/// (LayerOpacity.effective, LayerGroups.swift:49-64). Per-descendant multiplication, never group
/// compositing: folders are pass-through, so this matches the Mac rather than Photoshop. Every
/// draw in the plan goes through here (plain layers, stack bases and children, mask sources,
/// adjustment strength), which is what makes a clipped child in a dimmed folder dim twice, as
/// the Mac's LiveMaskRenderer does.
fn effective_opacity(by_id: &HashMap<Uuid, &Layer>, layer: &Layer) -> f64 {
    let mut opacity = layer.opacity.clamp(0.0, 1.0);
    let mut parent = layer.parent_id;
    let mut depth = 0;
    while let Some(pid) = parent {
        if depth >= MAX_NESTING { break; }
        let Some(folder) = by_id.get(&pid) else { break };
        opacity *= folder.opacity.clamp(0.0, 1.0);
        parent = folder.parent_id;
        depth += 1;
    }
    opacity
}

pub(crate) fn draw_for(_doc: &Document, by_id: &HashMap<Uuid, &Layer>, layer: &Layer, edit: Option<&PreviewEdit>, with_folders: bool) -> LayerDraw {
    let (transform, corners) = displayed_transform(layer, edit);
    let mut coverages: Vec<Coverage> = own_coverage(layer, edit).into_iter().collect();
    if with_folders { coverages.extend(folder_coverages(by_id, layer, edit)); }
    let (pw, ph) = layer.pixels.as_ref().map_or((0, 0), |p| (p.width, p.height));
    LayerDraw {
        id: layer.id, transform, corners, pixels_width: pw, pixels_height: ph, pixels_revision: layer.pixels_revision,
        opacity: effective_opacity(by_id, layer),
        // An adjustment layer blends through Core Graphics (BlendMode::cg_mode).
        blend: if layer.is_adjustment() { layer.blend_mode.cg_mode() } else { layer.blend_mode },
        keeps_alpha: layer.is_adjustment() && layer.blend_mode != BlendMode::Normal,
        coverages, clip: layer.mask_source_id,
        adjustment: displayed_adjustment(layer, edit),
    }
}

pub fn render_plan(doc: &Document, edit: Option<&PreviewEdit>) -> RenderPlan {
    let by_id: HashMap<Uuid, &Layer> = doc.layers.iter().map(|l| (l.id, l)).collect();
    let ids = doc.render_ids();
    let source_of = |id: Uuid| by_id.get(&id).and_then(|l| l.mask_source_id);
    let parent_of = |id: Uuid| by_id.get(&id).and_then(|l| l.parent_id);
    // Stacks: a base (not clipped) followed by siblings clipped to it.
    let mut stacked: HashSet<Uuid> = HashSet::new();
    let mut stacks: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for (index, base) in ids.iter().enumerate() {
        // An adjustment layer is never a stack base (it has no alpha of its own to share).
        if source_of(*base).is_some() || by_id[base].is_adjustment() { continue; }
        let mut children = Vec::new();
        for child in &ids[index + 1..] {
            if source_of(*child) == Some(*base) && parent_of(*child) == parent_of(*base) { children.push(*child); } else { break; }
        }
        if !children.is_empty() { stacked.extend(children.iter().copied()); stacks.insert(*base, children); }
    }
    let mut nodes = Vec::new();
    let mut needed_sources: Vec<Uuid> = Vec::new();
    let note_source = |draw: &LayerDraw, needed: &mut Vec<Uuid>| { if let Some(s) = draw.clip { if !needed.contains(&s) { needed.push(s); } } };
    for id in &ids {
        if stacked.contains(id) { continue; }
        let layer = by_id[id];
        // A clipped adjustment renders only as part of its base's stack: with the base hidden or
        // out of reach there is nothing beneath it to adjust, as macOS's drawComposite does.
        if layer.is_adjustment() && layer.mask_source_id.is_some() { continue; }
        if let Some(children) = stacks.get(id) {
            let mut base = draw_for(doc, &by_id, layer, edit, false);
            // The group composites in the base's Core Graphics mode (BlendMode::cg_mode). The base
            // itself is drawn into the group as Normal by both renderers, so nothing else reads it.
            base.blend = base.blend.cg_mode();
            let folder = folder_coverages(&by_id, layer, edit);
            let kids: Vec<LayerDraw> = children.iter().map(|c| { let mut d = draw_for(doc, &by_id, by_id[c], edit, false); d.clip = None; d }).collect();
            note_source(&base, &mut needed_sources);
            nodes.push(PlanNode::Stack { base, children: kids, folder_coverages: folder });
        } else {
            let draw = draw_for(doc, &by_id, layer, edit, true);
            note_source(&draw, &mut needed_sources);
            nodes.push(PlanNode::Layer { draw });
        }
    }
    // Sources, including sources of sources, chains capped at 256.
    let mut sources = Vec::new();
    let mut seen = HashSet::new();
    let mut i = 0;
    while i < needed_sources.len() && seen.len() < 256 {
        let id = needed_sources[i]; i += 1;
        if !seen.insert(id) { continue; }
        if let Some(layer) = by_id.get(&id) {
            if layer.is_group || layer.is_adjustment() { continue; }   // never a clipping source
            let draw = draw_for(doc, &by_id, layer, edit, false);
            if let Some(s) = draw.clip { if !needed_sources.contains(&s) { needed_sources.push(s); } }
            sources.push(draw);
        }
    }
    RenderPlan { nodes, sources }
}

/// Appends draws for `id` and its clipping chain to `plan.sources` when they are missing.
pub fn ensure_source(doc: &Document, plan: &mut RenderPlan, id: Uuid) {
    let by_id: HashMap<Uuid, &Layer> = doc.layers.iter().map(|l| (l.id, l)).collect();
    let mut next = Some(id); let mut depth = 0;
    while let (Some(sid), true) = (next, depth < 256) {
        depth += 1;
        if plan.sources.iter().any(|s| s.id == sid) { break; }
        let Some(layer) = by_id.get(&sid) else { break; };
        if layer.is_group || layer.is_adjustment() { break; }
        let draw = draw_for(doc, &by_id, layer, None, false);
        next = draw.clip;
        plan.sources.push(draw);
    }
}
