use crate::{CommandError, Document, Layer, MAX_LAYERS, MAX_NESTING};
use uuid::Uuid;

pub fn next_layer_name(doc: &Document) -> String {
    let mut n = 1;
    while doc.layers.iter().any(|l| l.name == format!("Layer {n}")) { n += 1; }
    format!("Layer {n}")
}

fn is_inside(doc: &Document, id: Uuid, folder: Uuid) -> bool {
    let mut parent = doc.layer(id).and_then(|l| l.parent_id);
    let mut steps = 0;
    while let Some(p) = parent {
        if p == folder { return true; }
        steps += 1;
        if steps > MAX_NESTING { return false; }
        parent = doc.layer(p).and_then(|l| l.parent_id);
    }
    false
}

/// Inserts a blank layer above the active layer (or at the top), inside the active folder if one is active.
pub fn add_blank_layer(doc: &mut Document) -> Result<Uuid, CommandError> {
    if doc.layers.len() >= MAX_LAYERS { return Err(CommandError::Argument("too many layers".into())); }
    let mut layer = Layer::blank(&next_layer_name(doc), doc.size());
    let active = doc.active_layer_id.and_then(|id| doc.layer(id).cloned());
    layer.parent_id = match &active { Some(a) if a.is_group => Some(a.id), Some(a) => a.parent_id, None => None };
    let mut insertion = doc.active_layer_id.and_then(|id| doc.index_of(id)).map(|i| i + 1).unwrap_or(doc.layers.len());
    if let Some(a) = &active {
        if a.is_group {
            if let Some(top) = doc.layers.iter().rposition(|l| is_inside(doc, l.id, a.id)) { insertion = insertion.max(top + 1); }
        }
    }
    let id = layer.id;
    doc.layers.insert(insertion, layer);
    doc.active_layer_id = Some(id);
    Ok(id)
}

pub fn rename_layer(doc: &mut Document, id: Uuid, name: &str) -> Result<(), CommandError> {
    if name.trim().is_empty() || name.len() > 16_384 { return Err(CommandError::Argument("layer name is empty or too long".into())); }
    doc.layer_mut(id).ok_or(CommandError::NoLayer)?.name = name.to_string();
    Ok(())
}

pub fn set_layer_visible(doc: &mut Document, id: Uuid, visible: bool) -> Result<(), CommandError> {
    doc.layer_mut(id).ok_or(CommandError::NoLayer)?.visible = visible;
    Ok(())
}

/// Removes the layer and, for a folder, everything inside it. The active layer moves to the layer below.
pub fn delete_layer(doc: &mut Document, id: Uuid) -> Result<(), CommandError> {
    let index = doc.index_of(id).ok_or(CommandError::NoLayer)?;
    let removed: Vec<Uuid> = doc.layers.iter().filter(|l| l.id == id || is_inside(doc, l.id, id)).map(|l| l.id).collect();
    doc.layers.retain(|l| !removed.contains(&l.id));
    for layer in &mut doc.layers {
        if layer.mask_source_id.map_or(false, |s| removed.contains(&s)) { layer.mask_source_id = None; }
    }
    if doc.active_layer_id.map_or(false, |a| removed.contains(&a)) {
        doc.active_layer_id = if doc.layers.is_empty() { None }
            else if index > 0 { Some(doc.layers[(index - 1).min(doc.layers.len() - 1)].id) }
            else { Some(doc.layers[0].id) };
    }
    Ok(())
}

pub fn set_active_layer(doc: &mut Document, id: Option<Uuid>) -> Result<(), CommandError> {
    if let Some(id) = id { doc.layer(id).ok_or(CommandError::NoLayer)?; }
    doc.active_layer_id = id;
    Ok(())
}

pub fn import_raster(doc: &mut Document, raster: crate::Raster, name: &str, at: Option<crate::Point>) -> Result<Uuid, CommandError> {
    let pixels = raster.width as u64 * raster.height as u64;
    if raster.width as i64 > crate::MAX_SIDE || raster.height as i64 > crate::MAX_SIDE || pixels > crate::MAX_PIXELS - doc.used_pixels() {
        return Err(CommandError::Import(crate::ImportError::TooLarge));
    }
    if doc.layers.len() >= MAX_LAYERS { return Err(CommandError::Argument("too many layers".into())); }
    let center = at.unwrap_or(crate::Point { x: doc.width as f64 / 2.0, y: doc.height as f64 / 2.0 });
    let origin = crate::Point { x: (center.x - raster.width as f64 / 2.0).floor(), y: (center.y - raster.height as f64 / 2.0).floor() };
    let mut layer = Layer::with_pixels(name, raster, origin);
    let active = doc.active_layer_id.and_then(|id| doc.layer(id).cloned());
    layer.parent_id = match &active { Some(a) if a.is_group => Some(a.id), Some(a) => a.parent_id, None => None };
    let id = layer.id;
    doc.layers.push(layer);
    doc.active_layer_id = Some(id);
    Ok(id)
}
