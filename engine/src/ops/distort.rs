use crate::*;
use uuid::Uuid;

fn shape_bounds(corners: &[Point; 4]) -> Result<(f64, f64, u32, u32), ProjectError> {
    if !Homography::is_usable(corners) { return Err(ProjectError::Invalid); }
    let xs = corners.iter().map(|p| p.x); let ys = corners.iter().map(|p| p.y);
    let min_x = xs.clone().fold(f64::INFINITY, f64::min).floor(); let max_x = xs.fold(f64::NEG_INFINITY, f64::max).ceil();
    let min_y = ys.clone().fold(f64::INFINITY, f64::min).floor(); let max_y = ys.fold(f64::NEG_INFINITY, f64::max).ceil();
    let w = (max_x - min_x).max(1.0); let h = (max_y - min_y).max(1.0);
    if w > MAX_SIDE as f64 || h > MAX_SIDE as f64 || w * h > MAX_PIXELS as f64 { return Err(ProjectError::TooLarge); }
    Ok((min_x, min_y, w as u32, h as u32))
}

/// Pixel coordinates (in a `w` x `h` grid shown through `transform`, flips applied) for each output pixel of the shape's bounds.
fn inverse_map(transform: &LayerTransform, corners: &[Point; 4], w: u32, h: u32) -> Option<impl Fn(f64, f64) -> Point> {
    let inv = Homography::unit_to(corners).invert()?;
    let (fx, fy) = (transform.flip_x, transform.flip_y);
    Some(move |x: f64, y: f64| {
        let u = inv.apply(Point { x, y });
        let ux = if fx { 1.0 - u.x } else { u.x }; let uy = if fy { 1.0 - u.y } else { u.y };
        Point { x: ux * w as f64, y: uy * h as f64 }
    })
}

/// `raster`, shown through `transform`, resampled so its corners land on `corners`.
pub fn warp(raster: &Raster, transform: &LayerTransform, corners: &[Point; 4], nearest: bool) -> Result<(Raster, LayerTransform), ProjectError> {
    let (min_x, min_y, w, h) = shape_bounds(corners)?;
    let mut placed = LayerTransform::axis_aligned(Point { x: min_x, y: min_y }, Size { width: w as f64, height: h as f64 });
    placed.sampling = transform.sampling;
    let map = inverse_map(transform, corners, raster.width, raster.height).ok_or(ProjectError::Invalid)?;
    let mut data = vec![0u8; (w * h * 4) as usize];
    for y in 0..h { for x in 0..w {
        let p = map(min_x + x as f64 + 0.5, min_y + y as f64 + 0.5);
        if p.x < 0.0 || p.y < 0.0 || p.x >= raster.width as f64 || p.y >= raster.height as f64 { continue; }
        let s = compositor::sample(raster, p.x, p.y, nearest);
        let i = ((y * w + x) * 4) as usize;
        for c in 0..4 { data[i + c] = (s[c] * 255.0).round().clamp(0.0, 255.0) as u8; }
    }}
    Ok((Raster::from_premultiplied(w, h, data), placed))
}

/// A mask warped like `warp`, `background` outside the shape; a uniform mask passes through.
pub fn warp_mask(mask: &GrayRaster, transform: &LayerTransform, corners: &[Point; 4], background: u8) -> Result<(GrayRaster, LayerTransform), ProjectError> {
    let (min_x, min_y, w, h) = shape_bounds(corners)?;
    let mut placed = LayerTransform::axis_aligned(Point { x: min_x, y: min_y }, Size { width: w as f64, height: h as f64 });
    placed.sampling = transform.sampling;
    if mask.width == 1 && mask.height == 1 { return Ok((mask.clone(), placed)); }
    let map = inverse_map(transform, corners, mask.width, mask.height).ok_or(ProjectError::Invalid)?;
    let nearest = transform.sampling == Sampling::Nearest;
    let mut data = vec![background; (w * h) as usize];
    for y in 0..h { for x in 0..w {
        let p = map(min_x + x as f64 + 0.5, min_y + y as f64 + 0.5);
        if p.x < 0.0 || p.y < 0.0 || p.x >= mask.width as f64 || p.y >= mask.height as f64 { continue; }
        let v = if nearest { mask.bytes()[(p.y as u32 * mask.width + p.x as u32) as usize] as f32 }
                else { gray_bilinear(mask, p.x, p.y) };
        data[(y * w + x) as usize] = v.round().clamp(0.0, 255.0) as u8;
    }}
    Ok((GrayRaster::from_bytes(w, h, data), placed))
}

fn gray_bilinear(mask: &GrayRaster, x: f64, y: f64) -> f32 {
    let w = mask.width as i64; let h = mask.height as i64;
    let fetch = |px: i64, py: i64| mask.bytes()[(py.clamp(0, h - 1) * w + px.clamp(0, w - 1)) as usize] as f32;
    let fx = x - 0.5; let fy = y - 0.5; let xu = fx.floor() as i64; let yu = fy.floor() as i64;
    let tx = (fx - xu as f64) as f32; let ty = (fy - yu as f64) as f32;
    (fetch(xu, yu) * (1.0 - tx) + fetch(xu + 1, yu) * tx) * (1.0 - ty) + (fetch(xu, yu + 1) * (1.0 - tx) + fetch(xu + 1, yu + 1) * tx) * ty
}

/// A warp cropped to its visible pixels; the crop (x0, y0, x1, y1) is in the warp's pixels.
pub fn warp_trimmed(raster: &Raster, transform: &LayerTransform, corners: &[Point; 4], nearest: bool) -> Result<(Raster, LayerTransform, (u32, u32, u32, u32)), ProjectError> {
    let (warped, placed) = warp(raster, transform, corners, nearest)?;
    let full = (0, 0, warped.width, warped.height);
    match compositor::alpha_bounds(&warped) {
        Some(crop) if crop != full => {
            let cropped = warped.cropped(crop.0, crop.1, crop.2 - crop.0, crop.3 - crop.1);
            let mut t = placed;
            t.origin = Point { x: placed.origin.x + crop.0 as f64, y: placed.origin.y + crop.1 as f64 };
            t.size = Size { width: (crop.2 - crop.0) as f64, height: (crop.3 - crop.1) as f64 };
            Ok((cropped, t, crop))
        }
        _ => Ok((warped, placed, full)),
    }
}

fn distort_at(doc: &mut Document, id: Uuid, transform: &LayerTransform, corners: &[Point; 4]) -> Result<(), CommandError> {
    let layer = doc.layer(id).ok_or(CommandError::NoLayer)?.clone();
    let Some(raster) = &layer.pixels else { return Ok(()); };
    let nearest = transform.sampling == Sampling::Nearest;
    let (pixels, placed, crop) = warp_trimmed(raster, transform, corners, nearest)?;
    let mask = match &layer.mask {
        Some(m) if m.placement.is_none() && m.is_linked() => {
            let (warped, _) = warp_mask(&m.pixels, transform, corners, m.background())?;
            let pixels = if warped.width == 1 && warped.height == 1 { warped } else {
                let (x0, y0, x1, y1) = crop;
                let mut data = Vec::with_capacity(((x1 - x0) * (y1 - y0)) as usize);
                for y in y0..y1 { data.extend_from_slice(&warped.bytes()[(y * warped.width + x0) as usize..(y * warped.width + x1) as usize]); }
                GrayRaster::from_bytes(x1 - x0, y1 - y0, data)
            };
            Some(Mask { pixels, enabled: m.enabled, placement: None, linked: m.linked })
        }
        Some(m) if m.is_linked() => {
            // A linked mask placed apart takes the same perspective over its own bounds.
            let placement = m.placement.unwrap().following(&layer.transform, transform);
            let carried = Homography::carried(&placement, transform, corners);
            if Homography::is_usable(&carried) {
                let (warped, moved) = warp_mask(&m.pixels, &placement, &carried, m.background())?;
                Some(Mask { pixels: warped, enabled: m.enabled, placement: Some(moved), linked: m.linked })
            } else { Some(m.clone()) }
        }
        Some(m) => Some(Mask { placement: Some(m.placement.unwrap_or(layer.transform)), ..m.clone() }),
        None => None,
    };
    let l = doc.layer_mut(id).unwrap();
    l.set_pixels(Some(pixels));
    l.transform = placed;
    l.set_mask(mask);
    Ok(())
}

pub fn distort_layer(doc: &mut Document, id: Uuid, transform: &LayerTransform, corners: &[Point; 4]) -> Result<(), CommandError> {
    let layer = doc.layer(id).ok_or(CommandError::NoLayer)?;
    if layer.is_group { return Err(CommandError::Argument("folders are not distorted directly".into())); }
    if !Homography::is_usable(corners) { return Err(CommandError::Argument("twisted or collapsed shape".into())); }
    distort_at(doc, id, transform, corners)
}

pub fn distort_group(doc: &mut Document, ids: &[Uuid], bounds: &LayerTransform, draft: &LayerTransform, corners: &[Point; 4]) -> Result<(), CommandError> {
    if !Homography::is_usable(corners) { return Err(CommandError::Argument("twisted or collapsed shape".into())); }
    if !draft.is_valid() { return Err(CommandError::Argument("transform out of range".into())); }
    for id in super::transform::members(doc, ids) {
        let moved = doc.layer(id).unwrap().transform.following(bounds, draft);
        if !moved.is_valid() { continue; }
        let carried = Homography::carried(&moved, draft, corners);
        if Homography::is_usable(&carried) { distort_at(doc, id, &moved, &carried)?; }
    }
    Ok(())
}
