use crate::*;
use uuid::Uuid;

fn layer_with_mask<'a>(doc: &'a mut Document, id: Uuid) -> Result<&'a mut Layer, CommandError> {
    let l = doc.layer_mut(id).ok_or(CommandError::NoLayer)?;
    if l.mask.is_none() { return Err(CommandError::Argument("the layer has no mask".into())); }
    Ok(l)
}

pub fn add_mask(doc: &mut Document, id: Uuid, revealing: bool) -> Result<(), CommandError> {
    let l = doc.layer_mut(id).ok_or(CommandError::NoLayer)?;
    if l.mask.is_some() { return Err(CommandError::Argument("the layer already has a mask".into())); }
    if doc.used_mask_pixels() + 1 > MAX_PIXELS { return Err(CommandError::Project(ProjectError::TooLarge)); }
    let l = doc.layer_mut(id).unwrap();
    l.set_mask(Some(Mask { pixels: GrayRaster::from_bytes(1, 1, vec![if revealing { 255 } else { 0 }]), enabled: true, placement: None, linked: None }));
    Ok(())
}

pub fn delete_mask(doc: &mut Document, id: Uuid) -> Result<(), CommandError> { layer_with_mask(doc, id)?.set_mask(None); Ok(()) }

pub fn set_mask_enabled(doc: &mut Document, id: Uuid, enabled: bool) -> Result<(), CommandError> {
    layer_with_mask(doc, id)?.mask_mut().unwrap().enabled = enabled; Ok(())
}

pub fn set_mask_linked(doc: &mut Document, id: Uuid, linked: bool) -> Result<(), CommandError> {
    layer_with_mask(doc, id)?.mask_mut().unwrap().linked = Some(linked); Ok(())
}

fn replace_pixels(doc: &mut Document, id: Uuid, f: impl FnOnce(&GrayRaster) -> GrayRaster) -> Result<(), CommandError> {
    let l = layer_with_mask(doc, id)?;
    let next = f(&l.mask.as_ref().unwrap().pixels);
    l.mask_mut().unwrap().pixels = next;
    Ok(())
}

/// Layer > Mask > Invert Mask: the same edit as Image > Invert with the mask targeted, so it too
/// stays inside the selection (the Mac has only the one, `invertPixels`).
pub fn invert_mask(doc: &mut Document, id: Uuid) -> Result<(), CommandError> {
    crate::ops::adjust::invert_layer(doc, id, true)
}

pub fn fill_mask(doc: &mut Document, id: Uuid, white: bool) -> Result<(), CommandError> {
    replace_pixels(doc, id, |m| GrayRaster::from_bytes(m.width, m.height, vec![if white { 255 } else { 0 }; (m.width * m.height) as usize]))
}

/// Separable Gaussian blur with edge clamping.
pub fn blur_gray(mask: &GrayRaster, sigma: f64) -> GrayRaster {
    if mask.is_uniform().is_some() || sigma <= 0.0 { return mask.clone(); }
    let radius = (sigma * 3.0).ceil() as i64;
    let kernel: Vec<f32> = (-radius..=radius).map(|i| (-(i * i) as f64 / (2.0 * sigma * sigma)).exp() as f32).collect();
    let sum: f32 = kernel.iter().sum();
    let (w, h) = (mask.width as i64, mask.height as i64);
    let src = mask.bytes();
    let mut tmp = vec![0f32; (w * h) as usize];
    for y in 0..h { for x in 0..w {
        let mut acc = 0f32;
        for (k, weight) in kernel.iter().enumerate() { let sx = (x + k as i64 - radius).clamp(0, w - 1); acc += src[(y * w + sx) as usize] as f32 * weight; }
        tmp[(y * w + x) as usize] = acc / sum;
    }}
    let mut out = vec![0u8; (w * h) as usize];
    for y in 0..h { for x in 0..w {
        let mut acc = 0f32;
        for (k, weight) in kernel.iter().enumerate() { let sy = (y + k as i64 - radius).clamp(0, h - 1); acc += tmp[(sy * w + x) as usize] * weight; }
        out[(y * w + x) as usize] = (acc / sum).round().clamp(0.0, 255.0) as u8;
    }}
    GrayRaster::from_bytes(mask.width, mask.height, out)
}

pub fn blur_mask(doc: &mut Document, id: Uuid, radius: f64) -> Result<(), CommandError> {
    if !radius.is_finite() || !(0.1..=1000.0).contains(&radius) { return Err(CommandError::Argument("blur radius must be 0.1 to 1000".into())); }
    replace_pixels(doc, id, |m| blur_gray(m, radius))
}

/// A 1x1 mask expanded to the layer's pixel grid (the layer's rectangle when it has no pixels).
pub fn expand_uniform(doc: &mut Document, id: Uuid) -> Result<(), CommandError> {
    let l = doc.layer(id).ok_or(CommandError::NoLayer)?;
    let Some(m) = &l.mask else { return Err(CommandError::Argument("the layer has no mask".into())); };
    if !m.is_uniform() { return Ok(()); }
    let (w, h) = l.pixels.as_ref().map_or((l.transform.size.width.round().max(1.0) as u32, l.transform.size.height.round().max(1.0) as u32), |p| (p.width, p.height));
    if (w as u64) * (h as u64) > MAX_PIXELS - doc.used_mask_pixels() { return Err(CommandError::Project(ProjectError::TooLarge)); }
    let v = m.pixels.bytes()[0];
    doc.layer_mut(id).unwrap().mask_mut().unwrap().pixels = GrayRaster::from_bytes(w, h, vec![v; (w * h) as usize]);
    Ok(())
}

pub fn copy_mask(doc: &mut Document, from: Uuid, to: Uuid) -> Result<(), CommandError> {
    if from == to { return Err(CommandError::Argument("same layer".into())); }
    let source = doc.layer(from).ok_or(CommandError::NoLayer)?.clone();
    let mask = source.mask.clone().ok_or(CommandError::Argument("the source has no mask".into()))?;
    let target = doc.layer(to).ok_or(CommandError::NoLayer)?;
    if target.is_group { return Err(CommandError::Argument("folders do not take a copied mask".into())); }
    let placement = mask.placement.unwrap_or(source.transform);
    let value = if mask.is_uniform() { None } else { Some(placement) };
    doc.layer_mut(to).unwrap().set_mask(Some(Mask { pixels: mask.pixels, enabled: mask.enabled, placement: value, linked: mask.linked }));
    Ok(())
}
