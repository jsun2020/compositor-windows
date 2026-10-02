use crate::*;
use uuid::Uuid;

fn pixel_layer(doc: &Document, id: Uuid) -> Result<&Layer, CommandError> {
    let l = doc.layer(id).ok_or(CommandError::NoLayer)?;
    if l.is_group { return Err(CommandError::Argument("folders are not transformed directly".into())); }
    Ok(l)
}

/// Moves one layer; its mask follows the placement rule.
pub fn set_transform(doc: &mut Document, id: Uuid, transform: LayerTransform) -> Result<(), CommandError> {
    pixel_layer(doc, id)?;
    if !transform.is_valid() { return Err(CommandError::Argument("transform out of range".into())); }
    let layer = doc.layer_mut(id).unwrap();
    let old = layer.transform;
    if let Some(mask) = &layer.mask {
        let placement = mask.follow(&old, &transform);
        if placement != mask.placement { layer.mask_mut().unwrap().placement = placement; }
    }
    layer.transform = transform;
    ops::shape::redraw(doc,id)?;
    Ok(())
}

/// The upright box around the pixel layers among `ids` (folders contribute their pixel descendants).
pub fn group_box(doc: &Document, ids: &[Uuid]) -> Option<LayerTransform> {
    let members = members(doc, ids);
    let mut pts = Vec::new();
    for id in &members { pts.extend(Homography::corners_of(&doc.layer(*id)?.transform)); }
    if pts.is_empty() { return None; }
    let min_x = pts.iter().map(|p| p.x).fold(f64::INFINITY, f64::min); let max_x = pts.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
    let min_y = pts.iter().map(|p| p.y).fold(f64::INFINITY, f64::min); let max_y = pts.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
    Some(LayerTransform::axis_aligned(Point { x: min_x, y: min_y }, Size { width: (max_x - min_x).max(1.0), height: (max_y - min_y).max(1.0) }))
}

/// Visible pixel layers selected directly or inside selected folders, in array order.
pub fn members(doc: &Document, ids: &[Uuid]) -> Vec<Uuid> {
    let visible = doc.visible_ids();
    doc.layers.iter().filter(|l| {
        if l.is_group || l.pixels.is_none() || !visible.contains(&l.id) { return false; }
        let mut current = Some(l.id); let mut steps = 0;
        while let Some(id) = current { if ids.contains(&id) { return true; } current = doc.layer(id).and_then(|x| x.parent_id); steps += 1; if steps > MAX_NESTING { break; } }
        false
    }).map(|l| l.id).collect()
}

pub fn transform_group(doc: &mut Document, ids: &[Uuid], bounds: &LayerTransform, draft: &LayerTransform) -> Result<(), CommandError> {
    if !draft.is_valid() { return Err(CommandError::Argument("transform out of range".into())); }
    for id in members(doc, ids) {
        let moved = doc.layer(id).unwrap().transform.following(bounds, draft);
        if moved.is_valid() { set_transform(doc, id, moved)?; }
    }
    Ok(())
}

pub fn nudge(doc: &mut Document, ids: &[Uuid], dx: f64, dy: f64) -> Result<(), CommandError> {
    for id in members(doc, ids) {
        let mut t = doc.layer(id).unwrap().transform;
        t.origin.x += dx; t.origin.y += dy;
        set_transform(doc, id, t)?;
    }
    Ok(())
}

pub fn flip_layers(doc: &mut Document, ids: &[Uuid], horizontal: bool) -> Result<(), CommandError> {
    let members = members(doc, ids);
    if members.is_empty() { return Err(CommandError::Argument("nothing to flip".into())); }
    let axis = if members.len() == 1 {
        let c = doc.layer(members[0]).unwrap().transform.center();
        if horizontal { c.x } else { c.y }
    } else {
        let b = group_box(doc, ids).unwrap().center();
        if horizontal { b.x } else { b.y }
    };
    for id in members {
        let flipped = doc.layer(id).unwrap().transform.mirrored(horizontal, axis);
        set_transform(doc, id, flipped)?;
    }
    Ok(())
}

/// An unlinked mask moved on its own: the new placement, pixels untouched.
pub fn set_mask_placement(doc: &mut Document, id: Uuid, placement: LayerTransform) -> Result<(), CommandError> {
    if !placement.is_valid() { return Err(CommandError::Argument("transform out of range".into())); }
    let layer = doc.layer_mut(id).ok_or(CommandError::NoLayer)?;
    if layer.mask.is_none() { return Err(CommandError::Argument("the layer has no mask".into())); }
    let value = if placement.same_placement(&layer.transform) { None } else { Some(placement) };
    if layer.mask.as_ref().unwrap().placement != value { layer.mask_mut().unwrap().placement = value; }
    Ok(())
}
