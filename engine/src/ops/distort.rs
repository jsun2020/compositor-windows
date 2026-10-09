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

/// CIPerspectiveTransform's corners are y-up and local to the integer output
/// bounds. Permuting the destination corners applies the layer's saved flips.
fn inverse_map(transform: &LayerTransform, corners: &[Point; 4], min_x: f64, min_y: f64, output_height: u32, source_width: u32, source_height: u32) -> Option<crate::adjust::camera_geometry::Perspective> {
    let corners = std::array::from_fn(|i| {
        let [x, y] = [[0, 0], [1, 0], [1, 1], [0, 1]][i];
        let u = if transform.flip_x { 1 - x } else { x };
        let v = if transform.flip_y { 1 - y } else { y };
        let p = corners[[0, 1, 3, 2][v * 2 + u]];
        Point { x: p.x - min_x, y: min_y + output_height as f64 - p.y }
    });
    crate::adjust::camera_geometry::Perspective::from_corners(&corners, source_width, source_height)
}

/// Resample a convex free distortion through the Mac's Core Image path.
/// The saved sampling setting is preserved in the placed transform, but this
/// filter uses its own interpolation even for a layer saved as Nearest.
/// The nearest flag remains in the API for callers of the earlier port.
pub fn warp(raster: &Raster, transform: &LayerTransform, corners: &[Point; 4], _nearest: bool) -> Result<(Raster, LayerTransform), ProjectError> {
    let (min_x, min_y, w, h) = shape_bounds(corners)?;
    let mut placed = LayerTransform::axis_aligned(Point { x: min_x, y: min_y }, Size { width: w as f64, height: h as f64 });
    placed.sampling = transform.sampling;
    let map = inverse_map(transform, corners, min_x, min_y, h, raster.width, raster.height).ok_or(ProjectError::Invalid)?;
    let mut data = vec![0u8; (w * h * 4) as usize];
    for y in 0..h { for x in 0..w {
        let (sx, sy) = map.coordinate_in_extent(x, y, h, raster.height);
        let sample = crate::core_image::sample(raster, sx, sy);
        let i = ((y * w + x) * 4) as usize;
        data[i..i + 4].copy_from_slice(&sample);
    }}
    Ok((Raster::from_premultiplied(w, h, data), placed))
}

/// Core Image's L8 warp has clear black outside its input texture. The separate
/// affine placed-mask background does not fill the perspective filter's extent.
/// A uniform 1x1 mask passes through unchanged, as in the Mac source.
pub fn warp_mask(mask: &GrayRaster, transform: &LayerTransform, corners: &[Point; 4], _background: u8) -> Result<(GrayRaster, LayerTransform), ProjectError> {
    let (min_x, min_y, w, h) = shape_bounds(corners)?;
    let mut placed = LayerTransform::axis_aligned(Point { x: min_x, y: min_y }, Size { width: w as f64, height: h as f64 });
    placed.sampling = transform.sampling;
    if mask.width == 1 && mask.height == 1 { return Ok((mask.clone(), placed)); }
    let map = inverse_map(transform, corners, min_x, min_y, h, mask.width, mask.height).ok_or(ProjectError::Invalid)?;
    let mut data = vec![0u8; (w * h) as usize];
    for y in 0..h { for x in 0..w {
        let (sx, sy) = map.coordinate_in_extent(x, y, h, mask.height);
        data[(y * w + x) as usize] = crate::core_image::sample_mask(mask, sx, sy);
    }}
    Ok((GrayRaster::from_bytes(w, h, data), placed))
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
