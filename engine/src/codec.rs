use crate::{ExportError, GrayRaster, ImportError, ProjectError, Raster};
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader};
use std::io::Cursor;

pub struct DecodedImage {
    pub raster: Raster,
}

impl std::fmt::Debug for DecodedImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DecodedImage")
            .field("raster", &self.raster)
            .finish()
    }
}

const SUPPORTED: [ImageFormat; 5] = [ImageFormat::Png, ImageFormat::Jpeg, ImageFormat::Tiff, ImageFormat::WebP, ImageFormat::Bmp];

/// Decodes an import, applies EXIF orientation and returns premultiplied RGBA8.
pub fn decode_image(bytes: &[u8]) -> Result<DecodedImage, ImportError> {
    let format = image::guess_format(bytes).map_err(|_| ImportError::Unreadable)?;
    if !SUPPORTED.contains(&format) { return Err(ImportError::Unsupported); }
    let reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut decoder = reader.into_decoder().map_err(|_| ImportError::Unreadable)?;
    let orientation = decoder.orientation().unwrap_or(image::metadata::Orientation::NoTransforms);
    let (w, h) = decoder.dimensions();
    if w == 0 || h == 0 || w > 30_000 || h > 30_000 { return Err(ImportError::TooLarge); }
    let mut img = DynamicImage::from_decoder(decoder).map_err(|_| ImportError::Unreadable)?;
    img.apply_orientation(orientation);
    let rgba = img.into_rgba8();
    let (w, h) = rgba.dimensions();
    Ok(DecodedImage { raster: Raster::from_straight(w, h, rgba.as_raw()) })
}

fn png_header(bytes: &[u8]) -> Result<png::Info<'static>, ProjectError> {
    if image::guess_format(bytes).ok() != Some(ImageFormat::Png) { return Err(ProjectError::MissingImage); }
    let decoder = png::Decoder::new(Cursor::new(bytes));
    let reader = decoder.read_info().map_err(|_| ProjectError::MissingImage)?;
    let info = reader.info();
    if info.bit_depth as u8 > 8 || info.animation_control.is_some() { return Err(ProjectError::MissingImage); }
    Ok(info.clone())
}

/// A package image's size, read from its PNG header alone (the checks `decode_package_png` makes first).
pub fn package_png_size(bytes: &[u8]) -> Result<(u32, u32), ProjectError> {
    let info = png_header(bytes)?;
    Ok((info.width, info.height))
}

/// Package assets are PNG only, at most 8 bits per channel, one frame.
pub fn decode_package_png(bytes: &[u8]) -> Result<Raster, ProjectError> {
    png_header(bytes)?;
    let img = image::load_from_memory_with_format(bytes, ImageFormat::Png).map_err(|_| ProjectError::MissingImage)?;
    let rgba = img.into_rgba8();
    let (w, h) = rgba.dimensions();
    Ok(Raster::from_straight(w, h, rgba.as_raw()))
}

pub fn decode_package_mask(bytes: &[u8]) -> Result<GrayRaster, ProjectError> {
    let info = png_header(bytes)?;
    if info.color_type != png::ColorType::Grayscale { return Err(ProjectError::Invalid); }
    let img = image::load_from_memory_with_format(bytes, ImageFormat::Png).map_err(|_| ProjectError::MissingImage)?;
    let gray = img.into_luma8();
    let (w, h) = gray.dimensions();
    Ok(GrayRaster::from_bytes(w, h, gray.into_raw()))
}

fn dpi_to_ppm(dpi: f64) -> u32 { (dpi / 0.0254).round().max(1.0) as u32 }

pub fn encode_png(raster: &Raster, dpi: f64) -> Result<Vec<u8>, ProjectError> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, raster.width, raster.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_pixel_dims(Some(png::PixelDimensions { xppu: dpi_to_ppm(dpi), yppu: dpi_to_ppm(dpi), unit: png::Unit::Meter }));
        let mut writer = encoder.write_header().map_err(|_| ProjectError::Encode)?;
        writer.write_image_data(&raster.to_straight()).map_err(|_| ProjectError::Encode)?;
    }
    Ok(out)
}

pub fn encode_gray_png(mask: &GrayRaster) -> Result<Vec<u8>, ProjectError> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, mask.width, mask.height);
        encoder.set_color(png::ColorType::Grayscale);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|_| ProjectError::Encode)?;
        writer.write_image_data(mask.bytes()).map_err(|_| ProjectError::Encode)?;
    }
    Ok(out)
}

/// Flattens onto `matte` (0..1 RGB), encodes at `quality` (0..1) with JFIF density.
pub fn encode_jpeg(raster: &Raster, quality: f64, matte: [f64; 3], dpi: f64) -> Result<Vec<u8>, ExportError> {
    let m: [u32; 3] = [0, 1, 2].map(|i| (matte[i].clamp(0.0, 1.0) * 255.0).round() as u32);
    let mut rgb = Vec::with_capacity((raster.width * raster.height * 3) as usize);
    for px in raster.bytes().chunks_exact(4) {
        let a = px[3] as u32;
        for c in 0..3 { rgb.push((px[c] as u32 + (m[c] * (255 - a) + 127) / 255).min(255) as u8); }
    }
    let q = (quality.clamp(0.0, 1.0) * 100.0).round().max(1.0) as u8;
    let mut out = Vec::new();
    let mut encoder = jpeg_encoder::Encoder::new(&mut out, q);
    encoder.set_density(jpeg_encoder::Density::Inch { x: dpi.round() as u16, y: dpi.round() as u16 });
    encoder.encode(&rgb, raster.width as u16, raster.height as u16, jpeg_encoder::ColorType::Rgb).map_err(|_| ExportError::Encode)?;
    Ok(out)
}

/// Horizontal DPI from a PNG pHYs chunk, if present in metres.
pub fn png_dpi(bytes: &[u8]) -> Option<f64> {
    let decoder = png::Decoder::new(Cursor::new(bytes));
    let reader = decoder.read_info().ok()?;
    let dims = reader.info().pixel_dims?;
    match dims.unit { png::Unit::Meter => Some(dims.xppu as f64 * 0.0254), _ => None }
}
