use crate::*;
use std::collections::HashSet;
use uuid::Uuid;

pub struct MergePlan { pub ids: Vec<Uuid>, pub removed: Vec<Uuid>, pub name: String, pub parent: Option<Uuid>, pub anchor: Uuid, pub action: &'static str }

pub fn merge_plan(doc: &Document, selected: &[Uuid]) -> Option<MergePlan> {
    let selected: Vec<Uuid> = selected.iter().copied().filter(|id| doc.layer(*id).is_some()).collect();
    if selected.len() > 1 {
        let mut picked: HashSet<Uuid> = selected.iter().copied().collect();
        for id in &selected { picked.extend(doc.descendants(*id)); }
        let ordered: Vec<Uuid> = doc.layers.iter().filter(|l| picked.contains(&l.id)).map(|l| l.id).collect();
        if !ordered.iter().any(|id| !doc.layer(*id).unwrap().is_group) { return None; }
        let top = *ordered.iter().rev().find(|id| selected.contains(id))?;
        let top_layer = doc.layer(top)?;
        return Some(MergePlan { ids: ordered.clone(), removed: ordered, name: top_layer.name.clone(), parent: top_layer.parent_id, anchor: top, action: "Merge Layers" });
    }
    let active = doc.layer(*selected.first()?)?;
    if active.is_group {
        let inside = doc.descendants(active.id);
        if !inside.iter().any(|id| !doc.layer(*id).unwrap().is_group) { return None; }
        let ids: Vec<Uuid> = doc.layers.iter().filter(|l| l.id == active.id || inside.contains(&l.id)).map(|l| l.id).collect();
        return Some(MergePlan { ids: ids.clone(), removed: ids, name: active.name.clone(), parent: active.parent_id, anchor: active.id, action: "Merge Group" });
    }
    let index = doc.index_of(active.id)?;
    let below = doc.layers[..index].iter().rev().find(|l| l.parent_id == active.parent_id)?;
    if below.is_group { return None; }
    Some(MergePlan { ids: vec![below.id, active.id], removed: vec![below.id, active.id], name: below.name.clone(), parent: active.parent_id, anchor: active.id, action: "Merge Down" })
}

/// The Grain kernel divergence (`Document::undrawn`'s "the Compositor 1.2.6 grain roughness")
/// does not block a merge: that grain IS drawn, with the older kernel, so baking what the screen
/// shows is the same thing Phase 3's merge already did for it.
const GRAIN_ROUGHNESS: &str = "the Compositor 1.2.6 grain roughness";

/// The layers composited as the canvas shows them into one pixel layer, trimmed, in their place.
///
/// Refuses first, before anything is mutated, if any layer the merge would composite carries an
/// undrawn feature this build's compositor would bake wrong -- permanently, since the source
/// layers and their `undrawn` entries are deleted by the merge (I1: "never silently drawn wrong"
/// must hold for merge too, not just for display and export).
pub fn merge(doc: &mut Document, selected: &[Uuid]) -> Result<Uuid, CommandError> {
    let plan = merge_plan(doc, selected).ok_or(CommandError::Argument("nothing to merge".into()))?;
    for id in &plan.ids {
        let Some(layer) = doc.layer(*id) else { continue };
        if let Some(feature) = layer.undrawn_features().into_iter().find(|f| f != GRAIN_ROUGHNESS) {
            return Err(CommandError::Argument(format!("Merging would bake {feature}, which this build does not draw yet")));
        }
    }
    let kept: HashSet<Uuid> = plan.ids.iter().copied().collect();
    let mut subset = doc.clone();
    subset.layers = doc.layers.iter().filter(|l| kept.contains(&l.id)).cloned().map(|mut l| {
        if let Some(p) = l.parent_id { if !kept.contains(&p) { l.parent_id = None; } }
        if let Some(s) = l.mask_source_id { if !kept.contains(&s) { l.mask_source_id = None; } }
        l
    }).collect();
    let canvas = Rect { x: 0.0, y: 0.0, width: doc.width as f64, height: doc.height as f64 };
    let flat = composite(&subset, canvas, doc.width, doc.height);
    let (raster, origin) = match compositor::alpha_bounds(&flat) {
        Some((x0, y0, x1, y1)) => (flat.cropped(x0, y0, x1 - x0, y1 - y0), Point { x: x0 as f64, y: y0 as f64 }),
        None => (flat, Point { x: 0.0, y: 0.0 }),
    };
    let mut merged = Layer::with_pixels(&plan.name, raster, origin);
    merged.parent_id = plan.parent;
    let mid = merged.id;
    let removed: HashSet<Uuid> = plan.removed.iter().copied().collect();
    let mut next: Vec<Layer> = doc.layers.iter().filter(|l| !removed.contains(&l.id)).cloned().collect();
    for l in &mut next { if l.mask_source_id.map_or(false, |s| removed.contains(&s)) { l.mask_source_id = Some(mid); } }
    let slot = doc.index_of(plan.anchor).unwrap_or(doc.layers.len());
    let insertion = slot - doc.layers[..slot].iter().filter(|l| removed.contains(&l.id)).count();
    next.insert(insertion.min(next.len()), merged);
    let previous = std::mem::replace(&mut doc.layers, next);
    let previous_active = doc.active_layer_id;
    doc.active_layer_id = Some(mid);
    if let Err(e) = super::hierarchy::validate(doc) {
        doc.layers = previous;
        doc.active_layer_id = previous_active;
        return Err(e);
    }
    Ok(mid)
}
