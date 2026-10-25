use crate::*;
use std::collections::HashMap;

/// The bytes of a `.comp` folder: the manifest text and `images/<name>` files.
#[derive(Clone, Debug, PartialEq)]
pub struct Package {
    pub manifest_json: String,
    pub images: Vec<(String, Vec<u8>)>,
}

fn check_budget(width: u32, height: u32, used: &mut u64) -> Result<(), ProjectError> {
    let w = width as i64; let h = height as i64;
    if !(1..=MAX_SIDE).contains(&w) || !(1..=MAX_SIDE).contains(&h) { return Err(ProjectError::TooLarge); }
    let pixels = width as u64 * height as u64;
    if pixels > MAX_PIXELS - *used { return Err(ProjectError::OverBudget); }
    *used += pixels;
    Ok(())
}

fn check_asset_bytes(bytes: &[u8]) -> Result<(), ProjectError> {
    if bytes.len() as u64 > MAX_ASSET_BYTES { return Err(ProjectError::TooLarge); }
    Ok(())
}

pub fn open_package(pkg: &Package) -> Result<Document, ProjectError> {
    let manifest = Manifest::parse(&pkg.manifest_json)?;
    if pkg.images.len() != pkg.images.iter().map(|(n, _)| n).collect::<std::collections::HashSet<_>>().len() {
        return Err(ProjectError::Invalid);
    }
    let files: HashMap<&str, &Vec<u8>> = pkg.images.iter().map(|(n, b)| (n.as_str(), b)).collect();
    // Every image and mask is sized from its PNG header and checked against the budget before any
    // is decoded: a project over the budget is refused without decoding (a 200 MP layer is 800 MB).
    let mut used = 0u64; let mut used_masks = 0u64;
    let mut source_bytes = 0usize;
    for record in &manifest.layers {
        for (name, total) in [(&record.image_file, &mut used), (&record.mask_file, &mut used_masks)] {
            let Some(name) = name else { continue };
            let bytes = files.get(name.as_str()).ok_or(ProjectError::MissingImage)?;
            check_asset_bytes(bytes)?;
            let (w, h) = package_image_size(bytes,name,manifest.version)?;
            check_budget(w, h, total)?;
        }
        if let Some(gif) = &record.gif {
            let source = files.get(gif.source_file.as_str()).ok_or(ProjectError::MissingImage)?;
            check_asset_bytes(source)?;
            source_bytes = source_bytes.checked_add(source.len()).ok_or(ProjectError::OverBudget)?;
            if source_bytes > animation::TOTAL_SOURCE_BYTE_LIMIT { return Err(ProjectError::OverBudget); }
            // All edited frames must be ordinary 8-bit PNGs. Count them before any decompression.
            for frame in &gif.frame_pixels {
                let bytes = files.get(frame.image_file.as_str()).ok_or(ProjectError::MissingImage)?;
                check_asset_bytes(bytes)?;
                let (w, h) = package_png_size(bytes)?;
                if frame.index == gif.selected_frame_index { check_budget(w, h, &mut 0)?; }
                else { check_budget(w, h, &mut used)?; }
            }
            package_png_size(files[record.image_file.as_deref().ok_or(ProjectError::Invalid)?])?;
        }
    }
    let mut layers = Vec::with_capacity(manifest.layers.len());
    let mut thumbnail_pixels = 0u64;
    for record in &manifest.layers {
        // Present: the pass above looked each one up.
        let pixels = match &record.image_file { Some(name) => Some(decode_package_image(files[name.as_str()],name,manifest.version)?), None => None };
        let mask = match &record.mask_file { Some(name) => Some(decode_package_mask(files[name.as_str()])?), None => None };
        let mut layer = Layer::from_record(record, pixels, mask);
        if let Some(gif) = &record.gif {
            let source = animation::GifSource::decode(files[gif.source_file.as_str()]).map_err(|_| ProjectError::Invalid)?;
            thumbnail_pixels += source.thumbnail_pixels();
            if thumbnail_pixels > animation::THUMBNAIL_PIXEL_BUDGET { return Err(ProjectError::OverBudget); }
            let mut animation = animation::GifAnimation::new(source);
            animation.selected_frame_index = gif.selected_frame_index;
            animation.geometry = gif.geometry.clone();
            animation.deleted_frame_indices = gif.deleted_frame_indices.iter().copied().collect();
            animation.frame_order = gif.frame_order.clone();
            if let Some(timing) = &gif.timing { animation.timing = timing.clone(); }
            for frame in &gif.frame_pixels { animation.frame_pixels.insert(frame.index, decode_package_png(files[frame.image_file.as_str()])?); }
            if !animation.valid() { return Err(ProjectError::Invalid); }
            let pixels = animation.frame(animation.selected_frame_index).map_err(|_| ProjectError::MissingImage)?;
            if layer.pixels.as_ref() != Some(&pixels) { return Err(ProjectError::Invalid); }
            // Reuse an override's allocation instead of retaining a second decoded copy.
            layer.pixels = Some(pixels);
            layer.extra.gif = Some(animation);
        }
        layers.push(layer);
    }
    Ok(Document {
        id: manifest.document_id, width: manifest.width as u32, height: manifest.height as u32,
        resolution: manifest.resolution.unwrap_or(DEFAULT_RESOLUTION), layers, active_layer_id: manifest.active_layer_id,
        guides: manifest.guides.clone().unwrap_or_default(), unknown: manifest.unknown.clone(),
        // The Mac never saves a selection (ProjectStore.swift:13-56): an opened project has none.
        selection: None, selection_revision: 1,
    })
}

pub fn save_package(doc: &Document) -> Result<Package, ProjectError> {
    let manifest = doc.manifest();
    manifest.validate()?;
    let mut used = 0u64; let mut used_masks = 0u64;
    let mut images = Vec::new();
    let mut source_bytes = 0usize;
    let mut thumbnail_pixels = 0u64;
    for layer in &doc.layers {
        if let Some(gif) = &layer.extra.gif {
            if !gif.valid() || layer.pixels.as_ref().is_none_or(|p| p.depth() != PixelDepth::Eight || (p.width, p.height) != gif.dimensions()) { return Err(ProjectError::Invalid); }
            source_bytes += gif.source.data.len();
            thumbnail_pixels += gif.source.thumbnail_pixels();
            if source_bytes > animation::TOTAL_SOURCE_BYTE_LIMIT || thumbnail_pixels > animation::THUMBNAIL_PIXEL_BUDGET { return Err(ProjectError::OverBudget); }
            images.push((animation::GifRecord::source_filename(&layer.id), gif.source.data.to_vec()));
            for (index, pixels) in &gif.frame_pixels {
                if *index == gif.selected_frame_index { check_budget(pixels.width, pixels.height, &mut 0)?; }
                else { check_budget(pixels.width, pixels.height, &mut used)?; }
                let bytes = encode_png(pixels, doc.resolution)?;
                check_asset_bytes(&bytes)?;
                images.push((animation::GifRecord::frame_filename(&layer.id, *index), bytes));
            }
        }
        if let Some(raster) = &layer.pixels {
            check_budget(raster.width, raster.height, &mut used)?;
            let hdr=raster.depth()==PixelDepth::Hdr;
            let bytes = if hdr {encode_exr(raster)?} else {encode_deep_png(raster, doc.resolution)?};
            check_asset_bytes(&bytes)?;
            images.push((if hdr {LayerRecord::hdr_filename(&layer.id)} else {LayerRecord::image_filename(&layer.id)}, bytes));
        }
        if let Some(mask) = &layer.mask {
            check_budget(mask.pixels.width, mask.pixels.height, &mut used_masks)?;
            let bytes = encode_gray_png(&mask.pixels)?;
            check_asset_bytes(&bytes)?;
            images.push((LayerRecord::mask_filename(&layer.id), bytes));
        }
    }
    Ok(Package { manifest_json: manifest.to_json_pretty()?, images })
}
