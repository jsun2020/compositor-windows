use crate::Document;

/// Mirrors every layer, folder and placed mask across the canvas middle.
pub fn flip_canvas(doc: &mut Document, horizontal: bool) {
    let axis = if horizontal { doc.width as f64 / 2.0 } else { doc.height as f64 / 2.0 };
    for layer in &mut doc.layers {
        layer.transform = layer.transform.mirrored(horizontal, axis);
        if let Some(mask) = &mut layer.mask {
            if let Some(p) = mask.placement { mask.placement = Some(p.mirrored(horizontal, axis)); }
        }
    }
}
