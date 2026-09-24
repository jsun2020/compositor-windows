use crate::{BlendMode, CommandError, Document};
use uuid::Uuid;

fn pixel_layer<'a>(doc: &'a mut Document, id: Uuid) -> Result<&'a mut crate::Layer, CommandError> {
    let layer = doc.layer_mut(id).ok_or(CommandError::NoLayer)?;
    if layer.is_group { return Err(CommandError::Argument("folders are pass-through, so their blend mode is always Normal".into())); }
    Ok(layer)
}

/// Folders take opacity too (from format v8 on): the Mac multiplies it into every descendant
/// rather than compositing the folder as a unit (see `plan::effective_opacity`), so there is
/// nothing here that needs to refuse a folder.
pub fn set_opacity(doc: &mut Document, id: Uuid, opacity: f64) -> Result<(), CommandError> {
    if !opacity.is_finite() { return Err(CommandError::Argument("opacity must be a number".into())); }
    doc.layer_mut(id).ok_or(CommandError::NoLayer)?.opacity = opacity.clamp(0.0, 1.0);
    Ok(())
}

/// Every layer among `ids`, folders included.
pub fn set_opacity_many(doc: &mut Document, ids: &[Uuid], opacity: f64) -> Result<(), CommandError> {
    if !opacity.is_finite() { return Err(CommandError::Argument("opacity must be a number".into())); }
    for id in ids {
        if let Some(layer) = doc.layer_mut(*id) { layer.opacity = opacity.clamp(0.0, 1.0); }
    }
    Ok(())
}

pub fn set_blend_mode(doc: &mut Document, id: Uuid, mode: BlendMode) -> Result<(), CommandError> {
    pixel_layer(doc, id)?.blend_mode = mode;
    Ok(())
}
