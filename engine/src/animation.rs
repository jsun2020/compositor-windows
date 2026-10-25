//! Immutable GIF sources and stable editable frame identities (Mac project format 17).
//! The decoder retains one disposal canvas, not one full raster per source frame.
use crate::{ImportError, Raster, MAX_PIXELS};
use serde::{Deserialize, Serialize};
use std::{collections::{BTreeMap, BTreeSet}, io::Cursor, sync::{Arc, Mutex}};
pub mod export;

pub const MAX_FRAMES: usize = 4096;
pub const IDENTITY_LIMIT: usize = 8192;
pub const THUMBNAIL_PIXEL_BUDGET: u64 = 16_000_000;
pub const SOURCE_BYTE_LIMIT: usize = 512 * 1024 * 1024;
pub const TOTAL_SOURCE_BYTE_LIMIT: usize = MAX_PIXELS as usize * 4;

#[derive(Clone, Debug)]
pub struct SourceFrame { pub duration: f64, pub thumbnail: Raster }

pub struct GifSource {
    pub data: Arc<[u8]>,
    pub width: u32,
    pub height: u32,
    pub frames: Vec<SourceFrame>,
    reader: Mutex<Option<Reader>>,
}
impl std::fmt::Debug for GifSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GifSource").field("bytes", &self.data.len()).field("frames", &self.frames.len()).finish()
    }
}
impl PartialEq for GifSource { fn eq(&self, rhs: &Self) -> bool { std::ptr::eq(self, rhs) } }

fn decoder(data: Arc<[u8]>, metadata_only: bool) -> Result<gif::Decoder<Cursor<Arc<[u8]>>>, ImportError> {
    let mut options = gif::DecodeOptions::new();
    options.set_color_output(gif::ColorOutput::RGBA);
    options.set_memory_limit(gif::MemoryLimit::Bytes((MAX_PIXELS * 4).try_into().unwrap()));
    options.check_frame_consistency(true);
    options.skip_frame_decoding(metadata_only);
    options.read_info(Cursor::new(data)).map_err(|_| ImportError::Unreadable)
}

struct Reader {
    decoder: gif::Decoder<Cursor<Arc<[u8]>>>,
    base: Vec<u8>,
    last: Option<(usize, Raster)>,
}
impl Reader {
    fn new(data: Arc<[u8]>) -> Result<Self, ImportError> {
        let decoder = decoder(data, false)?;
        let size = decoder.width() as usize * decoder.height() as usize * 4;
        Ok(Self { decoder, base: vec![0; size], last: None })
    }
    fn next(&mut self) -> Result<Raster, ImportError> {
        let (width, height) = (self.decoder.width() as usize, self.decoder.height() as usize);
        let frame = self.decoder.read_next_frame().map_err(|_| ImportError::Unreadable)?.ok_or(ImportError::Unreadable)?;
        let mut pixels = self.base.clone();
        let (left, top, w, h) = (frame.left as usize, frame.top as usize, frame.width as usize, frame.height as usize);
        if left + w > width || top + h > height || frame.buffer.len() != w * h * 4 { return Err(ImportError::Unreadable); }
        for y in 0..h { for x in 0..w {
            let src = (y * w + x) * 4;
            let dst = ((top + y) * width + left + x) * 4;
            if frame.buffer[src + 3] != 0 { pixels[dst..dst + 4].copy_from_slice(&frame.buffer[src..src + 4]); }
        } }
        match frame.dispose {
            gif::DisposalMethod::Previous => {},
            gif::DisposalMethod::Background => {
                self.base.copy_from_slice(&pixels);
                for y in top..top + h { self.base[(y * width + left) * 4..(y * width + left + w) * 4].fill(0); }
            },
            _ => self.base.copy_from_slice(&pixels),
        }
        let raster = Raster::from_premultiplied(width as u32, height as u32, pixels);
        let index = self.last.as_ref().map_or(0, |(i, _)| i + 1);
        self.last = Some((index, raster.clone()));
        Ok(raster)
    }
}

impl GifSource {
    pub fn decode(bytes: &[u8]) -> Result<Arc<Self>, ImportError> {
        if bytes.len() > SOURCE_BYTE_LIMIT || bytes.len() > TOTAL_SOURCE_BYTE_LIMIT { return Err(ImportError::OverBudget); }
        let data: Arc<[u8]> = bytes.into();
        let mut metadata = decoder(data.clone(), true)?;
        let (width, height) = (metadata.width() as u32, metadata.height() as u32);
        if width == 0 || height == 0 || width > 30_000 || height > 30_000 { return Err(ImportError::TooLarge); }
        if width as u64 * height as u64 > MAX_PIXELS { return Err(ImportError::OverBudget); }
        let mut delays = Vec::new();
        while let Some(frame) = metadata.read_next_frame().map_err(|_| ImportError::Unreadable)? {
            if delays.len() == MAX_FRAMES { return Err(ImportError::OverBudget); }
            delays.push(if frame.delay == 0 { 0.1 } else { frame.delay.max(2) as f64 / 100.0 });
        }
        if delays.is_empty() { return Err(ImportError::Unreadable); }
        drop(metadata);
        let scale = (96.0 / width.max(height) as f64).min(1.0);
        let (tw, th) = ((width as f64 * scale).round().max(1.0) as u32, (height as f64 * scale).round().max(1.0) as u32);
        if tw as u64 * th as u64 * delays.len() as u64 > THUMBNAIL_PIXEL_BUDGET { return Err(ImportError::OverBudget); }
        let mut reader = Reader::new(data.clone())?;
        let mut frames = Vec::with_capacity(delays.len());
        for duration in delays {
            let raster = reader.next()?;
            let image = image::RgbaImage::from_raw(width, height, raster.bytes().to_vec()).unwrap();
            let thumbnail = image::imageops::resize(&image, tw, th, image::imageops::FilterType::Triangle);
            frames.push(SourceFrame { duration, thumbnail: Raster::from_premultiplied(tw, th, thumbnail.into_raw()) });
        }
        // Do not retain the final canvas from scanning thumbnails. First playback begins at zero.
        Ok(Arc::new(Self { data, width, height, frames, reader: Mutex::new(None) }))
    }
    pub fn frame(&self, index: usize) -> Result<Raster, ImportError> {
        if index >= self.frames.len() { return Err(ImportError::Unreadable); }
        let mut guard = self.reader.lock().unwrap();
        if let Some((i, pixels)) = guard.as_ref().and_then(|r| r.last.as_ref()) {
            if *i == index { return Ok(pixels.clone()); }
        }
        if guard.as_ref().is_none_or(|r| r.last.as_ref().is_some_and(|(i, _)| *i > index)) {
            *guard = Some(Reader::new(self.data.clone())?);
        }
        let reader = guard.as_mut().unwrap();
        loop {
            let pixels = reader.next()?;
            if reader.last.as_ref().unwrap().0 == index { return Ok(pixels); }
        }
    }
    pub fn thumbnail_pixels(&self) -> u64 { self.frames.iter().map(|f| f.thumbnail.width as u64 * f.thumbnail.height as u64).sum() }
    pub fn storage_bytes(&self) -> usize { self.data.len() + self.thumbnail_pixels() as usize * 4 }
    pub fn forget_reader(&self) { *self.reader.lock().unwrap() = None; }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameReference { pub index: usize, pub source_index: usize }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DelayMode { #[default] #[serde(rename = "Frame delay")] PerFrame, #[serde(rename = "Same for all")] Uniform }
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameDelay { pub index: usize, pub duration: f64 }
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameTiming { pub mode: DelayMode, pub uniform_delay: f64, pub frame_delays: Vec<FrameDelay> }
impl FrameTiming {
    pub fn from_source(source: &GifSource) -> Self {
        let mut counts = BTreeMap::<u64, usize>::new();
        for frame in &source.frames { *counts.entry((frame.duration * 100.0).round() as u64).or_default() += 1; }
        let most = *counts.values().max().unwrap();
        let winners: Vec<_> = counts.iter().filter(|(_, n)| **n == most).collect();
        let uniform_delay = if winners.len() == 1 { *winners[0].0 as f64 / 100.0 }
            else { source.frames.iter().map(|f| f.duration).sum::<f64>() / source.frames.len() as f64 };
        Self { mode: DelayMode::PerFrame, uniform_delay, frame_delays: vec![] }
    }
    pub fn valid(&self, indices: &BTreeSet<usize>) -> bool {
        let valid_delay = |d: f64| d.is_finite() && (0.02..=655.35).contains(&d);
        let unique: BTreeSet<_> = self.frame_delays.iter().map(|f| f.index).collect();
        valid_delay(self.uniform_delay) && unique.len() == self.frame_delays.len() && unique.is_subset(indices)
            && self.frame_delays.iter().all(|f| valid_delay(f.duration))
    }
    pub fn delay(&self, index: usize, source: f64) -> f64 {
        if self.mode == DelayMode::Uniform { self.uniform_delay }
        else { self.frame_delays.iter().find(|f| f.index == index).map_or(source, |f| f.duration) }
    }
    pub fn set(&mut self, index: usize, duration: f64) {
        self.frame_delays.retain(|f| f.index != index);
        self.frame_delays.push(FrameDelay { index, duration });
        self.frame_delays.sort_by_key(|f| f.index);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GifAnimation {
    pub source: Arc<GifSource>,
    pub selected_frame_index: usize,
    pub geometry: Option<FrameGeometry>,
    pub frame_pixels: BTreeMap<usize, Raster>,
    pub deleted_frame_indices: BTreeSet<usize>,
    pub frame_order: Option<Vec<FrameReference>>,
    pub timing: FrameTiming,
}
impl GifAnimation {
    pub fn new(source: Arc<GifSource>) -> Self {
        let timing = FrameTiming::from_source(&source);
        Self { source, selected_frame_index: 0, geometry: None, frame_pixels: BTreeMap::new(), deleted_frame_indices: BTreeSet::new(), frame_order: None, timing }
    }
    pub fn references(&self) -> Vec<FrameReference> {
        self.frame_order.clone().unwrap_or_else(|| (0..self.source.frames.len()).filter(|i| !self.deleted_frame_indices.contains(i))
            .map(|index| FrameReference { index, source_index: index }).collect())
    }
    pub fn valid(&self) -> bool {
        if self.geometry.as_ref().is_some_and(|g| !g.valid()) { return false; }
        let (width, height) = self.dimensions();
        let n = self.source.frames.len();
        if self.deleted_frame_indices.iter().any(|i| *i >= n) { return false; }
        let frames = self.references();
        let indices: BTreeSet<_> = frames.iter().map(|f| f.index).collect();
        if frames.is_empty() || frames.len() > MAX_FRAMES || indices.len() != frames.len() || !indices.contains(&self.selected_frame_index) { return false; }
        if frames.iter().any(|f| f.index >= IDENTITY_LIMIT || f.source_index >= n || (f.index < n && (f.index != f.source_index || self.deleted_frame_indices.contains(&f.index)))) { return false; }
        if indices.iter().filter(|i| **i < n).copied().collect::<BTreeSet<_>>() != (0..n).filter(|i| !self.deleted_frame_indices.contains(i)).collect() { return false; }
        self.timing.valid(&indices) && self.frame_pixels.iter().all(|(i, p)| indices.contains(i)
            && p.depth() == crate::PixelDepth::Eight && p.width == width && p.height == height)
    }
    pub fn frame(&self, index: usize) -> Result<Raster, ImportError> {
        let reference = self.references().into_iter().find(|f| f.index == index).ok_or(ImportError::Unreadable)?;
        match self.frame_pixels.get(&index) {
            Some(p) => Ok(p.clone()),
            None => { let p = self.source.frame(reference.source_index)?; Ok(self.geometry.as_ref().map_or_else(|| p.clone(), |g| g.render(&p))) }
        }
    }
    pub fn dimensions(&self) -> (u32, u32) { self.geometry.as_ref().map_or((self.source.width, self.source.height), |g| (g.width, g.height)) }
    pub fn duration(&self, frame: FrameReference) -> f64 { self.timing.delay(frame.index, self.source.frames[frame.source_index].duration) }
    pub fn duplicate(&mut self, index: usize) -> bool {
        let mut order = self.references();
        let Some(position) = order.iter().position(|f| f.index == index) else { return false; };
        if order.len() >= MAX_FRAMES { return false; }
        let Some(new_index) = (self.source.frames.len()..IDENTITY_LIMIT).find(|i| order.iter().all(|f| f.index != *i)) else { return false; };
        order.insert(position + 1, FrameReference { index: new_index, source_index: order[position].source_index });
        if let Some(p) = self.frame_pixels.get(&index) { self.frame_pixels.insert(new_index, p.clone()); }
        if let Some(d) = self.timing.frame_delays.iter().find(|f| f.index == index).map(|f| f.duration) { self.timing.set(new_index, d); }
        self.selected_frame_index = new_index;
        self.frame_order = Some(order);
        true
    }
    pub fn delete(&mut self, index: usize) -> bool {
        let mut order = self.references();
        let Some(position) = order.iter().position(|f| f.index == index) else { return false; };
        if order.len() == 1 { return false; }
        order.remove(position);
        if index < self.source.frames.len() { self.deleted_frame_indices.insert(index); }
        self.frame_pixels.remove(&index);
        self.timing.frame_delays.retain(|f| f.index != index);
        if self.selected_frame_index == index { self.selected_frame_index = order[position.min(order.len() - 1)].index; }
        if self.frame_order.is_some() { self.frame_order = Some(order); }
        true
    }
    pub fn move_frame(&mut self, index: usize, position: usize) -> bool {
        let mut order = self.references();
        let Some(old) = order.iter().position(|f| f.index == index) else { return false; };
        if position >= order.len() { return false; }
        let reference = order.remove(old);
        order.insert(position, reference);
        self.frame_order = Some(order);
        true
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FrameGeometry {
    pub width: u32, pub height: u32,
    pub a: f64, pub b: f64, pub c: f64, pub d: f64, pub tx: f64, pub ty: f64,
    #[serde(default)] pub sampling: crate::Sampling,
}
impl FrameGeometry {
    pub fn new(width: u32, height: u32, map: crate::Affine, sampling: crate::Sampling) -> Self {
        Self { width, height, a: map.a, b: map.b, c: map.c, d: map.d, tx: map.tx, ty: map.ty, sampling }
    }
    pub fn map(&self) -> crate::Affine { crate::Affine { a: self.a, b: self.b, c: self.c, d: self.d, tx: self.tx, ty: self.ty } }
    pub fn valid(&self) -> bool {
        self.width > 0 && self.height > 0 && self.width <= 30_000 && self.height <= 30_000
            && self.width as u64 * self.height as u64 <= MAX_PIXELS
            && [self.a, self.b, self.c, self.d, self.tx, self.ty].iter().all(|v| v.is_finite() && v.abs() <= 1_000_000.0)
            && (self.a * self.d - self.b * self.c).abs() > 1e-15
    }
    pub fn render(&self, source: &Raster) -> Raster {
        let det = self.a * self.d - self.b * self.c;
        let spacing_x = ((self.d / det / self.width as f64 * source.width as f64).powi(2)
            + (self.b / det / self.width as f64 * source.height as f64).powi(2)).sqrt();
        let spacing_y = ((self.c / det / self.height as f64 * source.width as f64).powi(2)
            + (self.a / det / self.height as f64 * source.height as f64).powi(2)).sqrt();
        let level = if self.sampling == crate::Sampling::Nearest { 0 } else { crate::compositor::prefilter_level(source.width, source.height, spacing_x.max(spacing_y)) };
        let reduced = source.reduced(level);
        let mut output = vec![0; self.width as usize * self.height as usize * 4];
        // Normalized affine inverse; unlike Affine::invert, GIF permits determinants down to 1e-15.
        for y in 0..self.height { for x in 0..self.width {
            let u = (x as f64 + 0.5) / self.width as f64 - self.tx;
            let v = (y as f64 + 0.5) / self.height as f64 - self.ty;
            let px = (self.d * u - self.c * v) / det * source.width as f64;
            let py = (-self.b * u + self.a * v) / det * source.height as f64;
            if px < 0.0 || py < 0.0 || px >= source.width as f64 || py >= source.height as f64 { continue; }
            let value = crate::compositor::sample(&reduced, px * reduced.width as f64 / source.width as f64,
                py * reduced.height as f64 / source.height as f64, self.sampling == crate::Sampling::Nearest);
            let offset = ((y * self.width + x) * 4) as usize;
            for c in 0..4 { output[offset + c] = (value[c] * 255.0).round().clamp(0.0, 255.0) as u8; }
        } }
        Raster::from_premultiplied(self.width, self.height, output)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FramePixelRecord { pub index: usize, pub image_file: String }
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GifRecord {
    pub source_file: String,
    pub selected_frame_index: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub geometry: Option<FrameGeometry>,
    #[serde(default)] pub frame_pixels: Vec<FramePixelRecord>,
    #[serde(default)] pub deleted_frame_indices: Vec<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub frame_order: Option<Vec<FrameReference>>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub timing: Option<FrameTiming>,
}
impl GifRecord {
    pub fn source_filename(id: &uuid::Uuid) -> String { format!("{}.gif", crate::ids::upper_string(id)) }
    pub fn frame_filename(id: &uuid::Uuid, index: usize) -> String { format!("{}.frame-{index}.png", crate::ids::upper_string(id)) }
    pub fn valid(&self, id: &uuid::Uuid) -> bool {
        self.source_file == Self::source_filename(id) && self.selected_frame_index < IDENTITY_LIMIT
            && self.geometry.as_ref().is_none_or(FrameGeometry::valid)
            && self.frame_pixels.len() <= MAX_FRAMES && self.deleted_frame_indices.len() <= MAX_FRAMES
            && self.frame_pixels.iter().all(|f| f.index < IDENTITY_LIMIT && f.image_file == Self::frame_filename(id, f.index))
            && self.frame_pixels.iter().map(|f| f.index).collect::<BTreeSet<_>>().len() == self.frame_pixels.len()
            && self.deleted_frame_indices.iter().all(|i| *i < MAX_FRAMES)
            && self.deleted_frame_indices.iter().collect::<BTreeSet<_>>().len() == self.deleted_frame_indices.len()
            && self.frame_order.as_ref().is_none_or(|v| !v.is_empty() && v.len() <= MAX_FRAMES)
    }
}
impl GifAnimation {
    pub fn record(&self, id: &uuid::Uuid) -> GifRecord {
        GifRecord { source_file: GifRecord::source_filename(id), selected_frame_index: self.selected_frame_index,
            geometry: self.geometry.clone(), frame_pixels: self.frame_pixels.keys().map(|&index| FramePixelRecord { index, image_file: GifRecord::frame_filename(id, index) }).collect(),
            deleted_frame_indices: self.deleted_frame_indices.iter().copied().collect(), frame_order: self.frame_order.clone(), timing: Some(self.timing.clone()) }
    }
}

/// Capture a destructive pixel edit in the currently displayed frame. Growth/cropping is mapped
/// back onto the animation's fixed grid; editing one frame must not move every other frame.
pub(crate) fn capture_edits(before: &crate::Document, after: &mut crate::Document) -> Result<(), crate::CommandError> {
    for layer in &mut after.layers {
        let Some(old) = before.layer(layer.id) else { continue; };
        let Some(gif) = &old.extra.gif else { continue; };
        if layer.extra.gif.as_ref() != Some(gif) || layer.pixels == old.pixels { continue; }
        let (width, height) = gif.dimensions();
        let pixels = if let Some(pixels) = &layer.pixels {
            let map = old.transform.unit_to_document().invert().ok_or_else(|| crate::CommandError::Argument("Invalid GIF placement".into()))?
                .then(layer.transform.unit_to_document());
            FrameGeometry::new(width, height, map, old.transform.sampling).render(pixels)
        } else { Raster::new_transparent(width, height) };
        if layer.transform != old.transform {
            // The filter may have padded the shared layer mask for its temporary grown raster.
            // GIF pixel edits retain the original layer rectangle and its original mask.
            layer.mask = old.mask.clone();
            layer.mask_revision = old.mask_revision;
        }
        layer.transform = old.transform;
        layer.pixels = Some(pixels.clone());
        layer.extra.gif.as_mut().unwrap().frame_pixels.insert(gif.selected_frame_index, pixels);
    }
    validate_document(after)
}

pub(crate) fn validate_document(doc: &crate::Document) -> Result<(), crate::CommandError> {
    if !doc.layers.iter().any(|l| l.extra.gif.is_some()) { return Ok(()); }
    let used = doc.used_pixels();
    let mut bytes = 0;
    let mut thumbnails = 0;
    for layer in &doc.layers {
        if let Some(gif) = &layer.extra.gif {
            if !gif.valid() { return Err(crate::CommandError::Argument("Invalid GIF timeline".into())); }
            bytes += gif.source.data.len(); thumbnails += gif.source.thumbnail_pixels();
        }
    }
    if used > MAX_PIXELS || bytes > TOTAL_SOURCE_BYTE_LIMIT || thumbnails > THUMBNAIL_PIXEL_BUDGET {
        return Err(crate::CommandError::Refused("Animation exceeds the document's pixel, source-data or thumbnail budget".into()));
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum GifEdit {
    Duplicate { index: usize }, Delete { index: usize }, Move { index: usize, position: usize },
    Timing { timing: FrameTiming },
}
pub(crate) fn edit(doc: &mut crate::Document, id: uuid::Uuid, edit: GifEdit) -> Result<(), crate::CommandError> {
    let layer = doc.layer_mut(id).ok_or(crate::CommandError::NoLayer)?;
    let gif = layer.extra.gif.as_mut().ok_or_else(|| crate::CommandError::Argument("Layer has no animation".into()))?;
    let changed = match edit {
        GifEdit::Duplicate { index } => gif.duplicate(index),
        GifEdit::Delete { index } => gif.delete(index),
        GifEdit::Move { index, position } => gif.move_frame(index, position),
        GifEdit::Timing { timing } => {
            if !timing.valid(&gif.references().iter().map(|f| f.index).collect()) { return Err(crate::CommandError::Argument("Invalid GIF timing".into())); }
            gif.timing = timing; true
        }
    };
    if !changed { return Err(crate::CommandError::Refused("The animation must contain between 1 and 4096 frames".into())); }
    let pixels = gif.frame(gif.selected_frame_index)?;
    layer.set_pixels(Some(pixels));
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineFrame { pub index: usize, pub source_index: usize, pub duration: f64, pub edited: bool }
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Timeline { pub selected_frame_index: usize, pub frames: Vec<TimelineFrame>, pub timing: FrameTiming }
impl GifAnimation {
    pub fn timeline(&self) -> Timeline {
        Timeline { selected_frame_index: self.selected_frame_index, timing: self.timing.clone(),
            frames: self.references().into_iter().map(|f| TimelineFrame { index: f.index, source_index: f.source_index,
                duration: self.duration(f), edited: self.frame_pixels.contains_key(&f.index) }).collect() }
    }
}
