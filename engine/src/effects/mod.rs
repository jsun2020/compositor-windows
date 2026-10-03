//! Layer effects (Phase 3.5c): their file form (`settings`), how they are drawn (`render`), and
//! how a layer is drawn with them. The Mac makes one image per layer, in the layer's own pixel
//! grid, of the layer with its effects around it, and draws that image through the layer's
//! transform grown in proportion, with the layer's opacity and blend mode and no mask (its mask is
//! already in the image): ImageExporter.swift:43-53, LayerEffects.swift:403-496. This port does the
//! same, and both renderers draw that one image: the CPU compositor samples it, the GPU uploads it
//! as the layer's texture.
pub mod settings;
pub mod render;

use crate::*;
use serde::Serialize;
use std::sync::Mutex;

/// How the layer's own enabled mask lies over the layer's pixel grid, for baking it into the pixels
/// before the effects are made (`clipImage`, LayerMask.swift:110-125).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum EffectsMask {
    /// Covering the layer: the mask stretched over its grid (`clipImage` returns the mask itself).
    Covering { nearest: bool },
    /// Placed apart: sampled where each layer pixel's centre lands on the document, its background
    /// beyond its pixels, exactly as `coverage_at` samples it for a plain draw.
    Placed { coverage: Coverage, layer: LayerTransform, corners: Option<[Point; 4]> },
}

/// A draw of a layer with its effects: what the plan hands both renderers.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectsDraw {
    /// Transparent layer pixels added on every side (`LayerEffects::margin`).
    pub inset: u32,
    /// Changes whenever the image would, given revisions that name the bytes (an engine's documents:
    /// `Engine::edit` never reissues one): the GPU uploads a new texture on a new key. The cache
    /// does not rely on it: it finds images by the buffers themselves.
    pub key: String,
    /// The shown effects, as `LayerEffects::drawn` gives them.
    #[serde(skip)] pub effects: LayerEffects,
    #[serde(skip)] pub mask: Option<EffectsMask>,
}

fn fnv1a(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.bytes() { hash ^= byte as u64; hash = hash.wrapping_mul(0x0000_0100_0000_01b3); }
    hash
}

/// Whether and how the plan draws `layer` with its effects (R 4.2): a layer with pixels, not a
/// folder or an adjustment, whose shown effects are valid (`LayerEffectsRenderer.cached`,
/// LayerEffects.swift:403-408), with a padded image of at most `EFFECTS_SURFACE_LIMIT` pixels
/// (:441). Otherwise None and the layer draws as it is, as the Mac's `try?` makes it. A pending
/// distortion whose grown corners fold over draws the layer plainly until the drag moves on.
pub fn effects_draw(layer: &Layer, edit: Option<&PreviewEdit>) -> Option<EffectsDraw> {
    if layer.is_group || layer.is_adjustment() { return None; }
    let pixels = layer.pixels.as_ref()?;
    let effects = match edit {Some(PreviewEdit::Effects{id,effects}) if *id==layer.id=>effects.as_ref(),_=>layer.extra.effects.as_ref()}?.drawn()?;
    if !effects.fits(pixels.width, pixels.height) { return None; }
    let inset = effects.margin();
    let (transform, corners) = displayed_transform(layer, edit);
    if let Some(c) = &corners {
        if !Homography::is_usable(&grown_corners(c, pixels.width, pixels.height, inset)) { return None; }
    }
    let mask = match layer.mask.as_ref().filter(|m| m.enabled) {
        None => None,
        Some(m) => {
            // A linked mask moves with its layer through a move, turn or resize, so the image made
            // from the stored layer and placement is the image of the displayed ones: a drag reuses
            // it. Only a mask drag, or a distortion (which the mask does not follow), changes it.
            let follows = m.is_linked() && match edit {
                Some(PreviewEdit::Mask { id, .. }) => *id != layer.id,
                Some(PreviewEdit::Layer { corners, .. }) | Some(PreviewEdit::Group { corners, .. }) => corners.is_none(),
                _ => true,
            };
            let edit = if follows { None } else { edit };
            match displayed_mask_placement(layer, edit) {
                None => Some(EffectsMask::Covering { nearest: transform.sampling == Sampling::Nearest }),
                Some(_) => {
                    let (layer_transform, layer_corners) = displayed_transform(layer, edit);
                    own_coverage(layer, edit).map(|coverage| EffectsMask::Placed { coverage, layer: layer_transform, corners: layer_corners })
                }
            }
        }
    };
    // Covering masks, linked masks and no mask leave the displayed transform out, so moving the
    // layer keeps its image.
    let inputs = serde_json::to_string(&(layer.mask_revision, &mask, &effects)).unwrap_or_default();
    let key = format!("{}-{:016x}", layer.pixels_revision, fnv1a(&inputs));
    Some(EffectsDraw { inset, key, effects, mask })
}

/// `LayerEffectsRenderer.placed` (LayerEffects.swift:411-419): the transform grown in proportion to
/// the padded image about the same centre, so the layer's own pixels land where they were. Its
/// rotation, flips and sampling are kept, so the effects turn and flip with the layer.
pub fn grown_transform(t: &LayerTransform, width: u32, height: u32, inset: u32) -> LayerTransform {
    let (w, h) = ((width + 2 * inset) as f64, (height + 2 * inset) as f64);
    let mut grown = *t;
    grown.size = Size { width: t.size.width * w / (w - 2.0 * inset as f64), height: t.size.height * h / (h - 2.0 * inset as f64) };
    let c = t.center();
    grown.origin = Point { x: c.x - grown.size.width / 2.0, y: c.y - grown.size.height / 2.0 };
    grown
}

/// A pending distortion's corners for the padded image: the unit square grown by the inset on
/// every side, through the same perspective.
pub fn grown_corners(c: &[Point; 4], width: u32, height: u32, inset: u32) -> [Point; 4] {
    let (a, b) = (inset as f64 / width as f64, inset as f64 / height as f64);
    let map = Homography::unit_to(c);
    [map.apply(Point { x: -a, y: -b }), map.apply(Point { x: 1.0 + a, y: -b }), map.apply(Point { x: 1.0 + a, y: 1.0 + b }), map.apply(Point { x: -a, y: 1.0 + b })]
}

/// The layer's pixels through its own enabled mask (`LayerEffectsRenderer.masked`,
/// LayerEffects.swift:510-521): the effects follow the shape the mask leaves.
fn shown(pixels: &Raster, mask: &GrayRaster, how: &EffectsMask) -> Raster {
    let (w, h) = (pixels.width, pixels.height);
    let mut data = pixels.bytes().to_vec();
    for y in 0..h { for x in 0..w {
        let (cx, cy) = (x as f64 + 0.5, y as f64 + 0.5);
        let k = match how {
            EffectsMask::Covering { nearest } => gray_sample(mask, cx * mask.width as f64 / w as f64, cy * mask.height as f64 / h as f64, *nearest),
            EffectsMask::Placed { coverage, layer, corners } => {
                let p = match corners {
                    Some(c) => {
                        let (ux, uy) = (cx / w as f64, cy / h as f64);
                        Homography::unit_to(c).apply(Point { x: if layer.flip_x { 1.0 - ux } else { ux }, y: if layer.flip_y { 1.0 - uy } else { uy } })
                    }
                    None => layer.pixel_to_document(w, h).apply(Point { x: cx, y: cy }),
                };
                match to_pixels(&coverage.placement, coverage.corners.as_ref(), coverage.width, coverage.height, p) {
                    Some(q) if q.x >= 0.0 && q.y >= 0.0 && q.x < coverage.width as f64 && q.y < coverage.height as f64 => gray_sample(mask, q.x, q.y, coverage.nearest),
                    Some(_) => coverage.background as f32 / 255.0,
                    None => 1.0,
                }
            }
        };
        let i = ((y * w + x) * 4) as usize;
        for c in 0..4 { data[i + c] = (data[i + c] as f32 * k).round() as u8; }
    }}
    Raster::from_premultiplied(w, h, data)
}

/// The layer with its effects around it: `draw.inset` pixels larger on every side, premultiplied.
pub fn effects_image(layer: &Layer, pixels: &Raster, draw: &EffectsDraw) -> Raster {
    let shown = match (&draw.mask, &layer.mask) { (Some(how), Some(mask)) => shown(pixels, &mask.pixels, how), _ => pixels.clone() };
    let padded = Padded { pixels: shown.bytes(), width: shown.width as usize, height: shown.height as usize, inset: draw.inset as usize };
    let (w, h) = (padded.padded_width() as u32, padded.padded_height() as u32);
    Raster::from_premultiplied(w, h, render_passes(&padded, &EffectPasses::from_effects(&draw.effects)))
}


/// The most effects images an engine keeps (ruling OQ5; the Mac's own cache keeps 8,
/// LayerEffects.swift:387-392).
pub const EFFECTS_CACHE_ENTRIES: usize = 8;
/// The most bytes of effects images an engine keeps (ruling OQ5). An image larger than this on
/// its own is never kept: it is made again for each use, as the Mac never stores an entry over
/// its own limit (LayerEffects.swift:389).
pub const EFFECTS_CACHE_BYTES: usize = 512 * 1024 * 1024;

/// An image and what it was made from: the very pixel and mask buffers, held so that their
/// addresses cannot be reused while the entry lives, and the draw.
struct Entry { pixels: Raster, mask: Option<GrayRaster>, draw: EffectsDraw, image: Raster }

/// The entries, least recently used first, and how many images were made.
#[derive(Default)]
struct Kept { entries: Vec<Entry>, made: u64 }

/// The effects images an engine has made (ruling F-B1), keyed by content: the pixel buffer, the
/// mask buffer (compared by identity, never by revision number) and an equal `EffectsDraw`. It
/// belongs to the `Engine`, not to a document: history snapshots and preview copies share their
/// layers' buffers, so they find the same entries, and no document or snapshot keeps an image.
/// Least recently used goes first once there are more than `max_entries` or their images hold
/// more than `max_bytes`.
pub struct EffectsCache { max_entries: usize, max_bytes: usize, kept: Mutex<Kept> }

impl Default for EffectsCache {
    fn default() -> EffectsCache { EffectsCache::new(EFFECTS_CACHE_ENTRIES, EFFECTS_CACHE_BYTES) }
}

impl EffectsCache {
    pub fn new(max_entries: usize, max_bytes: usize) -> EffectsCache {
        EffectsCache { max_entries, max_bytes, kept: Mutex::new(Kept::default()) }
    }
    fn kept(&self) -> std::sync::MutexGuard<'_, Kept> { self.kept.lock().unwrap_or_else(|e| e.into_inner()) }
    /// `layer` drawn with its effects as `draw` describes them: the kept image when there is one
    /// for these very buffers and this draw, else made now (the lock is not held meanwhile) and
    /// kept if it fits.
    pub fn image(&self, layer: &Layer, draw: &EffectsDraw) -> Option<Raster> {
        let pixels = layer.pixels.as_ref()?;
        let mask = draw.mask.as_ref().and(layer.mask.as_ref()).map(|m| m.pixels.clone());
        let same = |e: &Entry| e.pixels.same_pixels(pixels) && e.draw == *draw
            && match (&e.mask, &mask) { (None, None) => true, (Some(a), Some(b)) => a.same_pixels(b), _ => false };
        {
            let mut kept = self.kept();
            if let Some(i) = kept.entries.iter().position(same) {
                let entry = kept.entries.remove(i);
                let image = entry.image.clone();
                kept.entries.push(entry);
                return Some(image);
            }
        }
        // Dead images go before a new one is made, so they never add to the peak of making it.
        self.prune();
        let image = effects_image(layer, pixels, draw);
        let mut kept = self.kept();
        kept.made += 1;
        if image.bytes().len() <= self.max_bytes {
            kept.entries.push(Entry { pixels: pixels.clone(), mask, draw: draw.clone(), image: image.clone() });
            while kept.entries.len() > self.max_entries || kept.entries.iter().map(|e| e.image.bytes().len()).sum::<usize>() > self.max_bytes {
                kept.entries.remove(0);
            }
        }
        Some(image)
    }
    /// Whether an image for these very buffers and this draw is kept (nothing is made).
    pub fn contains(&self, layer: &Layer, draw: &EffectsDraw) -> bool {
        let Some(pixels) = layer.pixels.as_ref() else { return false };
        let mask = draw.mask.as_ref().and(layer.mask.as_ref()).map(|m| &m.pixels);
        self.kept().entries.iter().any(|e| e.pixels.same_pixels(pixels) && e.draw == *draw
            && match (&e.mask, mask) { (None, None) => true, (Some(a), Some(b)) => a.same_pixels(b), _ => false })
    }
    /// Keeps `image`, made elsewhere (the job worker, from the same layer and draw), as `image` would
    /// have kept it: found by the same buffers and draw from now on. Not kept when it is over the
    /// byte limit, as a made image would not be. Dead entries go first, as `image` drops them before
    /// making one (final review I-2): each keeps the layer raster it was made from alive, and the
    /// worker's full-size images are the only path for large styled layers, so after every edit the
    /// old raster (96 MB at 24 MP) would stay pinned, uncounted by the byte limit, until evicted.
    pub fn insert(&self, layer: &Layer, draw: &EffectsDraw, image: Raster) {
        let Some(pixels) = layer.pixels.as_ref() else { return };
        self.prune();
        if image.bytes().len() > self.max_bytes || self.contains(layer, draw) { return; }
        let mask = draw.mask.as_ref().and(layer.mask.as_ref()).map(|m| m.pixels.clone());
        let mut kept = self.kept();
        kept.entries.push(Entry { pixels: pixels.clone(), mask, draw: draw.clone(), image });
        while kept.entries.len() > self.max_entries || kept.entries.iter().map(|e| e.image.bytes().len()).sum::<usize>() > self.max_bytes {
            kept.entries.remove(0);
        }
    }
    /// Drops the images no one can find again: those whose pixel or mask buffer only the cache
    /// holds (a closed document, an ended preview, a history entry let go). They would otherwise
    /// keep their images and buffers alive and push out images still in use. A buffer counts as
    /// held elsewhere only when it has more holders than the cache's own entries give it: entries
    /// made from one layer at several mask placements share its pixel buffer, and must not keep
    /// one another alive.
    pub fn prune(&self) {
        let mut kept = self.kept();
        let live: Vec<bool> = kept.entries.iter().map(|e| {
            let pixel_holders = kept.entries.iter().filter(|o| o.pixels.same_pixels(&e.pixels)).count();
            let pixels_live = e.pixels.holders() > pixel_holders;
            let mask_live = e.mask.as_ref().map_or(true, |m| {
                let mask_holders = kept.entries.iter()
                    .filter(|o| o.mask.as_ref().map_or(false, |om| om.same_pixels(m))).count();
                m.holders() > mask_holders
            });
            pixels_live && mask_live
        }).collect();
        let mut i = 0;
        kept.entries.retain(|_| { let keep = live[i]; i += 1; keep });
    }
    /// Images kept.
    pub fn len(&self) -> usize { self.kept().entries.len() }
    /// Bytes of the images kept.
    pub fn bytes(&self) -> usize { self.kept().entries.iter().map(|e| e.image.bytes().len()).sum() }
    /// Images made since the cache was created, kept or not.
    pub fn made(&self) -> u64 { self.kept().made }
}
