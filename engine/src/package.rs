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
    if pixels > MAX_PIXELS - *used { return Err(ProjectError::TooLarge); }
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
    let mut used = 0u64; let mut used_masks = 0u64;
    let mut layers = Vec::with_capacity(manifest.layers.len());
    for record in &manifest.layers {
        let pixels = match &record.image_file {
            Some(name) => {
                let bytes = files.get(name.as_str()).ok_or(ProjectError::MissingImage)?;
                check_asset_bytes(bytes)?;
                let raster = decode_package_png(bytes)?;
                check_budget(raster.width, raster.height, &mut used)?;
                Some(raster)
            }
            None => None,
        };
        let mask = match &record.mask_file {
            Some(name) => {
                let bytes = files.get(name.as_str()).ok_or(ProjectError::MissingImage)?;
                check_asset_bytes(bytes)?;
                let gray = decode_package_mask(bytes)?;
                check_budget(gray.width, gray.height, &mut used_masks)?;
                Some(gray)
            }
            None => None,
        };
        layers.push(Layer::from_record(record, pixels, mask));
    }
    Ok(Document {
        id: manifest.document_id, width: manifest.width as u32, height: manifest.height as u32,
        resolution: manifest.resolution.unwrap_or(DEFAULT_RESOLUTION), layers, active_layer_id: manifest.active_layer_id,
    })
}

pub fn save_package(doc: &Document) -> Result<Package, ProjectError> {
    let manifest = doc.manifest();
    manifest.validate()?;
    let mut used = 0u64; let mut used_masks = 0u64;
    let mut images = Vec::new();
    for layer in &doc.layers {
        if let Some(raster) = &layer.pixels {
            check_budget(raster.width, raster.height, &mut used)?;
            let bytes = encode_png(raster, doc.resolution)?;
            check_asset_bytes(&bytes)?;
            images.push((LayerRecord::image_filename(&layer.id), bytes));
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
