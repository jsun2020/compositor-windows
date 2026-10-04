use crate::*;
use uuid::Uuid;

/// Bilinear or nearest sample in premultiplied float RGBA (0..1). Outside the raster clamps to the edge; coverage is decided by the caller.
pub fn sample(raster: &Raster, x: f64, y: f64, nearest: bool) -> [f32; 4] {
    sample_with_phase(raster, x, y, nearest, false)
}

fn sample_with_phase(raster: &Raster, x: f64, y: f64, nearest: bool, quantized: bool) -> [f32; 4] {
    let w = raster.width as i64; let h = raster.height as i64;
    let fetch = |px: i64, py: i64| -> [f32; 4] {
        if px < 0 || py < 0 || px >= w || py >= h { return [0.0; 4]; }
        let p = raster.pixel(px as u32, py as u32);
        [p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0, p[3] as f32 / 255.0]
    };
    if nearest {
        let px = x.floor() as i64; let py = y.floor() as i64;
        return fetch(px, py);
    }
    let fx = x - 0.5; let fy = y - 0.5;
    let xu = fx.floor() as i64; let yu = fy.floor() as i64;
    let phase = |t: f64| {
        // Core Graphics enlargement phases, measured independently on Mac 1.2.10
        // and the white-alpha Mac 1.4.5 horizontal/vertical/grid exports.
        const WEIGHTS: [f32; 9] = [0.0, 0.0625, 0.125, 0.25, 0.5, 0.75, 0.875, 0.9375, 1.0];
        if quantized { WEIGHTS[(t * 8.0).round().clamp(0.0, 8.0) as usize] } else { t as f32 }
    };
    let tx = phase(fx - xu as f64); let ty = phase(fy - yu as f64);
    let x0 = xu.clamp(0, w - 1); let x1 = (xu + 1).clamp(0, w - 1);
    let y0 = yu.clamp(0, h - 1); let y1 = (yu + 1).clamp(0, h - 1);
    let a = fetch(x0, y0); let b = fetch(x1, y0); let c = fetch(x0, y1); let d = fetch(x1, y1);
    let mut out = [0f32; 4];
    if quantized {
        // CG resamples bytes vertically first, then horizontally. Each pass keeps
        // the nearer texel and subtracts its truncated weighted contribution.
        // The independent alpha grid and colored-text return match this exactly.
        let interpolate = |a: f32, b: f32, t: f64| {
            // An inverse affine map can put an exact half one f64 ulp above it.
            // CG chooses the lower texel at the tie; preserve that choice.
            let (near, far, minor) = if t > 0.5 + 1e-10 { (b, a, 1.0 - t) } else { (a, b, t) };
            let weight = phase(minor);
            near + (far * weight).floor() - (near * weight).floor()
        };
        for i in 0..4 {
            let left = interpolate((a[i] * 255.0).round(), (c[i] * 255.0).round(), fy - yu as f64);
            let right = interpolate((b[i] * 255.0).round(), (d[i] * 255.0).round(), fy - yu as f64);
            out[i] = interpolate(left, right, fx - xu as f64) / 255.0;
        }
        return out;
    }
    for i in 0..4 {
        out[i] = (a[i] * (1.0 - tx) + b[i] * tx) * (1.0 - ty) + (c[i] * (1.0 - tx) + d[i] * tx) * ty;
    }
    out
}

/// A 30000-pixel side reaches 1 pixel in 15 halvings; the cap only bounds a bad caller.
pub const MAX_PREFILTER_LEVEL: u32 = 16;

/// How many sharp halvings a raster of this size takes before its final resample: halve until
/// one output pixel covers at most 2 source pixels. The GL renderer uploads the raster at this
/// same level, so the two renderers prefilter identically (`prefilterLevel` in the app mirrors
/// this loop).
pub fn prefilter_level(width: u32, height: u32, pixels_per_output: f64) -> u32 {
    let (mut w, mut h) = (width, height);
    let mut factor = pixels_per_output;
    let mut level = 0;
    while factor > 2.0 && w > 1 && h > 1 { w = (w / 2).max(1); h = (h / 2).max(1); level += 1; factor /= 2.0; }
    level
}

/// Whether a draw prefilters at all: never for Nearest, and never through a distortion (the
/// homography resamples the full raster). macOS applies the same rule in `LayerRenderer.reduced`,
/// which prefilters for Smooth as well as High quality.
pub fn prefilters(sampling: Sampling, distorted: bool) -> bool { sampling != Sampling::Nearest && !distorted }

/// Sharp halvings for large reductions: reduce until one output pixel covers at most 2 source pixels.
fn prefiltered(raster: &Raster, pixels_per_output: f64) -> (Raster, f64) {
    let level = prefilter_level(raster.width, raster.height, pixels_per_output);
    (raster.reduced(level), 0.5f64.powi(level as i32))
}

/// (x0, y0, x1, y1) of the pixels with alpha > 0, x1/y1 exclusive; None when fully transparent.
pub fn alpha_bounds(r: &Raster) -> Option<(u32, u32, u32, u32)> {
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0u32, 0u32);
    // A row at a time over its alpha bytes: the first and last pixel with any alpha.
    for (y, row) in r.bytes().chunks_exact(r.width as usize * 4).enumerate() {
        let Some(first) = row.chunks_exact(4).position(|p| p[3] > 0) else { continue };
        let last = row.chunks_exact(4).rposition(|p| p[3] > 0).unwrap();
        x0 = x0.min(first as u32); x1 = x1.max(last as u32 + 1);
        y0 = y0.min(y as u32); y1 = y1.max(y as u32 + 1);
    }
    if x1 == 0 { None } else { Some((x0, y0, x1, y1)) }
}

/// Maps a document point into a layer's pixel grid: through the distortion when there is one, else the affine inverse.
pub(crate) fn to_pixels(transform: &LayerTransform, corners: Option<&[Point; 4]>, w: u32, h: u32, p: Point) -> Option<Point> {
    if let Some(c) = corners {
        let inv = Homography::unit_to(c).invert()?;
        let u = inv.apply(p);
        let ux = if transform.flip_x { 1.0 - u.x } else { u.x };
        let uy = if transform.flip_y { 1.0 - u.y } else { u.y };
        Some(Point { x: ux * w as f64, y: uy * h as f64 })
    } else {
        transform.pixel_to_document(w, h).invert().map(|inv| inv.apply(p))
    }
}

/// A draw's raster reduced for the output scale, and the factor mapping full-resolution pixel
/// coordinates onto it. One entry per clipping source, built once per `draw_layer` rather than
/// per pixel.
pub type SourceRasters = std::collections::HashMap<Uuid, (Raster, f64, bool, f64, f64)>;

/// The reduced rasters for `source` and everything up its clipping chain, at `out_per_doc`
/// output pixels per document unit. macOS reduces a clipping source exactly like any other
/// draw: `LiveMaskRenderer` paints it through the same `drawOwn` closure the canvas uses, which
/// goes through `LayerRenderer.draw` and its sharp halvings (EditorCanvas.swift).
pub fn clip_source_rasters(doc: &Document, plan: &RenderPlan, source: Uuid, out_per_doc: f64, cache: &EffectsCache) -> SourceRasters {
    clip_source_rasters_xy(doc, plan, source, out_per_doc, out_per_doc, cache)
}

fn clip_source_rasters_xy(doc: &Document, plan: &RenderPlan, source: Uuid, out_per_doc: f64, out_y: f64, cache: &EffectsCache) -> SourceRasters {
    let mut out = SourceRasters::new();
    let mut next = Some(source);
    let mut depth = 0;
    while let Some(id) = next {
        if depth > 256 || out.contains_key(&id) { break; }
        depth += 1;
        let Some(draw) = plan.sources.iter().find(|s| s.id == id) else { break; };
        if let Some(raster) = draw_raster(doc, draw, cache) {
            let (raster, scale) = reduced_for(&raster, draw, out_per_doc);
            let nearest = nearest_for_draw(draw, out_per_doc, scale);
            out.insert(id, (raster, scale, nearest, out_per_doc, out_y));
        }
        next = draw.clip;
    }
    out
}

/// The raster a draw samples at this output scale, and the coordinate factor onto it.
fn reduced_for(raster: &Raster, draw: &LayerDraw, out_per_doc: f64) -> (Raster, f64) {
    if !prefilters(draw.transform.sampling, draw.corners.is_some()) { return (raster.clone(), 1.0); }
    let source_per_output = raster.width as f64 / (draw.transform.size.width * out_per_doc);
    prefiltered(raster, source_per_output)
}

/// The raster a draw samples: the layer's pixels, or, when the plan draws its effects, the layer
/// with its effects around it (`EffectsCache::image`), `effects.inset` pixels larger each side.
pub fn draw_raster(doc: &Document, draw: &LayerDraw, cache: &EffectsCache) -> Option<Raster> {
    let layer = doc.layer(draw.id)?;
    match &draw.effects { Some(fx) => cache.image(layer, fx), None => layer.pixels.clone() }
}

/// The layer's premultiplied colour at a document point (no opacity, no coverage); transparent
/// outside. Samples the full-resolution raster; `sample_draw_reduced` takes a prefiltered one.
pub fn sample_draw(doc: &Document, draw: &LayerDraw, p: Point, cache: &EffectsCache) -> [f32; 4] {
    match draw_raster(doc, draw, cache) { Some(raster) => sample_draw_reduced(draw, p, Some(&(raster, 1.0, nearest_for_draw(draw, 1.0, 1.0), 1.0, 1.0))), None => [0.0; 4] }
}

/// Mac 1.4.5 LayerRenderer.interpolation: an upright image whose final
/// reduction lands pixel for pixel copies texels, even at a fractional origin.
/// This affects drawing only; the layer's saved sampling setting stays intact.
fn nearest_for_draw(draw: &LayerDraw, out_per_doc: f64, scale: f64) -> bool {
    draw.transform.sampling == Sampling::Nearest || (
        draw.corners.is_none() && draw.transform.rotation % 360.0 == 0.0 &&
        draw.pixels_width > 0 &&
        (draw.transform.size.width * out_per_doc / draw.pixels_width as f64 / scale - 1.0).abs() < 0.001
    )
}

/// Automatic pixel copies still have Core Graphics' antialiased rectangle
/// edge (setShouldAntialias follows the saved setting, not the chosen filter).
fn copied_edge_coverage(draw: &LayerDraw, p: Point, out_x: f64, out_y: f64) -> f32 {
    let axis = |at: f64, origin: f64, size: f64, out: f64| {
        ((at + 0.5 / out).min(origin + size) - (at - 0.5 / out).max(origin)).max(0.0) * out
    };
    let coverage = axis(p.x, draw.transform.origin.x, draw.transform.size.width, out_x).min(1.0) *
        axis(p.y, draw.transform.origin.y, draw.transform.size.height, out_y).min(1.0);
    // The Mac 1.4.5 upright probe uses an 8-bit coverage mask: half coverage is
    // 127/255, not 0.5. Quantize before multiplying premultiplied colour/alpha.
    ((coverage * 255.0).floor() / 255.0) as f32
}

fn sample_placed(draw: &LayerDraw, source: &Raster, px: Point, p: Point, scale: f64, nearest: bool, out_x: f64, out_y: f64, enlargement_phases: bool) -> [f32; 4] {
    let antialiased_copy = nearest && draw.transform.sampling != Sampling::Nearest;
    let coverage = if antialiased_copy { copied_edge_coverage(draw, p, out_x, out_y) } else { 1.0 };
    if coverage <= 0.0 { return [0.0; 4]; }
    if !antialiased_copy && (px.x < 0.0 || px.y < 0.0 || px.x >= draw.pixels_width as f64 || px.y >= draw.pixels_height as f64) { return [0.0; 4]; }
    let (x, y) = if antialiased_copy {
        ((px.x * scale).clamp(0.0, source.width as f64 - 0.0001), (px.y * scale).clamp(0.0, source.height as f64 - 0.0001))
    } else { (px.x * scale, px.y * scale) };
    let enlarged = enlargement_phases && draw.corners.is_none() && draw.transform.size.width * out_x / draw.pixels_width as f64 / scale > 1.001;
    let mut color = sample_with_phase(source, x, y, nearest, enlarged);
    if antialiased_copy && coverage < 1.0 {
        // Core Graphics materializes the covered premultiplied source as bytes
        // before source-over. Keeping fractional bytes until after blending
        // changes overlapping edges by a channel (Mac-created return oracle).
        for v in &mut color { *v = (*v * coverage * 255.0).round() / 255.0; }
    }
    color
}

/// `reduced` is the raster the draw samples and the factor onto it; None samples nothing, as a
/// draw without a raster has nothing to sample. The raster is looked up once by the caller
/// (`clip_source_rasters`), never here for every pixel.
fn sample_draw_reduced(draw: &LayerDraw, p: Point, reduced: Option<&(Raster, f64, bool, f64, f64)>) -> [f32; 4] {
    // The plan's own size of the raster the draw samples (padded when it has effects).
    let (w, h) = (draw.pixels_width, draw.pixels_height);
    if w == 0 || h == 0 { return [0.0; 4]; }
    let Some((source, scale, nearest, out_x, out_y)) = reduced else { return [0.0; 4]; };
    let Some(px) = to_pixels(&draw.transform, draw.corners.as_ref(), w, h, p) else { return [0.0; 4]; };
    sample_placed(draw, source, px, p, *scale, *nearest, *out_x, *out_y, true)
}

pub(crate) fn gray_sample(mask: &GrayRaster, x: f64, y: f64, nearest: bool) -> f32 {
    let w = mask.width as i64; let h = mask.height as i64;
    let fetch = |px: i64, py: i64| mask.bytes()[(py.clamp(0, h - 1) * w + px.clamp(0, w - 1)) as usize] as f32 / 255.0;
    if nearest { return fetch(x.floor() as i64, y.floor() as i64); }
    let fx = x - 0.5; let fy = y - 0.5;
    let xu = fx.floor() as i64; let yu = fy.floor() as i64;
    let tx = (fx - xu as f64) as f32; let ty = (fy - yu as f64) as f32;
    let a = fetch(xu, yu); let b = fetch(xu + 1, yu); let c = fetch(xu, yu + 1); let d = fetch(xu + 1, yu + 1);
    (a * (1.0 - tx) + b * tx) * (1.0 - ty) + (c * (1.0 - tx) + d * tx) * ty
}

/// One coverage mask's value at a document point: its pixels inside, its background outside.
pub fn coverage_at(doc: &Document, cov: &Coverage, p: Point) -> f32 {
    let Some(mask) = doc.layer(cov.layer_id).and_then(|l| l.mask.as_ref()) else { return 1.0; };
    let Some(px) = to_pixels(&cov.placement, cov.corners.as_ref(), cov.width, cov.height, p) else { return 1.0; };
    if px.x < 0.0 || px.y < 0.0 || px.x >= cov.width as f64 || px.y >= cov.height as f64 { return cov.background as f32 / 255.0; }
    gray_sample(&mask.pixels, px.x, px.y, cov.nearest)
}

fn coverages_at(doc: &Document, covs: &[Coverage], p: Point) -> f32 {
    covs.iter().fold(1.0, |k, c| k * coverage_at(doc, c, p))
}

/// A clipping source's coverage at a document point: its alpha times opacity, own mask and its own
/// clipping chain. `reduced` carries the prefiltered raster per source, as `clip_source_rasters`
/// builds it; a source missing from it has no raster and covers nothing.
pub fn source_coverage_at(doc: &Document, plan: &RenderPlan, source: Uuid, p: Point, reduced: &SourceRasters) -> f32 {
    fn inner(doc: &Document, plan: &RenderPlan, source: Uuid, p: Point, depth: u32, reduced: &SourceRasters) -> f32 {
        if depth > 256 { return 1.0; }
        let Some(draw) = plan.sources.iter().find(|s| s.id == source) else { return 1.0; };
        let a = sample_draw_reduced(draw, p, reduced.get(&source))[3] * draw.opacity as f32 * coverages_at(doc, &draw.coverages, p);
        match draw.clip { Some(c) => a * inner(doc, plan, c, p, depth + 1, reduced), None => a }
    }
    inner(doc, plan, source, p, 0, reduced)
}

struct Target<'a> { data: &'a mut [u8], w: u32, h: u32, region: Rect, enlargement_phases: bool }

impl<'a> Target<'a> {
    fn doc_point(&self, ox: u32, oy: u32) -> Point {
        Point { x: self.region.x + (ox as f64 + 0.5) * self.region.width / self.w as f64, y: self.region.y + (oy as f64 + 0.5) * self.region.height / self.h as f64 }
    }
    /// Output-pixel bounding box of a draw, padded by one.
    fn bbox(&self, draw: &LayerDraw) -> (u32, u32, u32, u32) {
        let b = match draw.corners { Some(c) => { let xs = c.iter().map(|p| p.x); let ys = c.iter().map(|p| p.y);
            let (x0, x1) = (xs.clone().fold(f64::INFINITY, f64::min), xs.fold(f64::NEG_INFINITY, f64::max));
            let (y0, y1) = (ys.clone().fold(f64::INFINITY, f64::min), ys.fold(f64::NEG_INFINITY, f64::max));
            Rect { x: x0, y: y0, width: x1 - x0, height: y1 - y0 } } None => draw.transform.bounds() };
        let sx = self.w as f64 / self.region.width; let sy = self.h as f64 / self.region.height;
        let x0 = (((b.x - self.region.x) * sx).floor() as i64 - 1).clamp(0, self.w as i64) as u32;
        let y0 = (((b.y - self.region.y) * sy).floor() as i64 - 1).clamp(0, self.h as i64) as u32;
        let x1 = (((b.max_x() - self.region.x) * sx).ceil() as i64 + 1).clamp(0, self.w as i64) as u32;
        let y1 = (((b.max_y() - self.region.y) * sy).ceil() as i64 + 1).clamp(0, self.h as i64) as u32;
        (x0, y0, x1, y1)
    }
}

/// Draws one layer with its opacity, coverages and clip, blended with its mode.
fn draw_layer(doc: &Document, plan: &RenderPlan, target: &mut Target, draw: &LayerDraw, blend: BlendMode, use_clip: bool, cache: &EffectsCache) {
    if let Some(adjustment) = &draw.adjustment {
        if adjustment.kind.is_spatial() { return spatial_target(doc, plan, target, draw, adjustment, blend, use_clip, cache); }
        return adjust_target(doc, plan, target, draw, blend, use_clip, cache);
    }
    // A draw outside the region makes no raster: an eyedropper's 1x1 region would otherwise make
    // the effects image of every styled layer in the document (the plan's box needs none).
    let (x0, y0, x1, y1) = target.bbox(draw);
    if x0 >= x1 || y0 >= y1 { return; }
    let Some(raster) = draw_raster(doc, draw, cache) else { return; };
    // Prefilter large affine reductions (never for Nearest, never for distortions).
    let out_per_doc = target.w as f64 / target.region.width;
    let (source, scale) = reduced_for(&raster, draw, out_per_doc);
    // The clipping chain reduces at the same scale, built once here rather than per pixel.
    let clip_sources = match (use_clip, draw.clip) {
        (true, Some(c)) => clip_source_rasters_xy(doc, plan, c, out_per_doc, target.h as f64 / target.region.height, cache),
        _ => SourceRasters::new(),
    };
    let nearest = nearest_for_draw(draw, out_per_doc, scale);
    for oy in y0..y1 { for ox in x0..x1 {
        let p = target.doc_point(ox, oy);
        let Some(px) = to_pixels(&draw.transform, draw.corners.as_ref(), raster.width, raster.height, p) else { continue; };
        let mut s = sample_placed(draw, &source, px, p, scale, nearest, out_per_doc, target.h as f64 / target.region.height, target.enlargement_phases);
        if s[3] <= 0.0 { continue; }
        let mut k = draw.opacity as f32 * coverages_at(doc, &draw.coverages, p);
        if use_clip { if let Some(c) = draw.clip { k *= source_coverage_at(doc, plan, c, p, &clip_sources); } }
        if k <= 0.0 { continue; }
        for v in &mut s { *v *= k; }
        let i = ((oy * target.w + ox) * 4) as usize;
        compose_u8(&mut target.data[i..i + 4], s, blend);
    }}
}

/// An adjustment layer: the colours already in the target, mapped where this layer's coverage
/// reaches. Nothing is sampled from the layer itself - it has no pixels - and the target's alpha
/// is kept, so a soft edge below stays exactly as soft (macOS's LiveMaskRenderer.adjust).
fn adjust_target(doc: &Document, plan: &RenderPlan, target: &mut Target, draw: &LayerDraw, blend: BlendMode, use_clip: bool, cache: &EffectsCache) {
    let Some(adjustment) = &draw.adjustment else { return; };
    let prepared = PreparedAdjustment::prepare(adjustment);
    let out_per_doc = target.w as f64 / target.region.width;
    let clip_sources = match (use_clip, draw.clip) {
        (true, Some(c)) => clip_source_rasters_xy(doc, plan, c, out_per_doc, target.h as f64 / target.region.height, cache),
        _ => SourceRasters::new(),
    };
    for oy in 0..target.h { for ox in 0..target.w {
        let i = ((oy * target.w + ox) * 4) as usize;
        let alpha = target.data[i + 3] as f32;
        if alpha <= 0.0 { continue; }
        let p = target.doc_point(ox, oy);
        let mut k = draw.opacity as f32 * coverages_at(doc, &draw.coverages, p);
        if use_clip { if let Some(c) = draw.clip { k *= source_coverage_at(doc, plan, c, p, &clip_sources); } }
        if k <= 0.0 { continue; }
        // Full strength in Normal: the Mac's own 8-bit kernel, so an export matches it to the level.
        // Not for a layer whose own mode is not Normal (`keeps_alpha`): the Mac takes its
        // full-coverage path there.
        if k >= 1.0 && blend == BlendMode::Normal && !draw.keeps_alpha {
            let px = [target.data[i], target.data[i + 1], target.data[i + 2], target.data[i + 3]];
            if let Some(out) = prepared.pixel(px, p) { target.data[i..i + 4].copy_from_slice(&out); continue; }
        }
        let original = [target.data[i] as f32 / alpha, target.data[i + 1] as f32 / alpha, target.data[i + 2] as f32 / alpha];
        let mut adjusted = prepared.color(original, p);
        if blend != BlendMode::Normal { adjusted = blend_rgb(blend, original, adjusted); }
        for c in 0..3 {
            let mixed = original[c] + (adjusted[c].clamp(0.0, 1.0) - original[c]) * k;
            target.data[i + c] = (mixed * alpha).round().clamp(0.0, alpha) as u8;
        }
    }}
}

/// `original` moved toward `result` by `k`, premultiplied: the Mac's CIBlendWithMask at the layer's
/// opacity, then the copy through its masks (R 3.4 steps 4-5). Colour never exceeds alpha.
fn toward(original: [u8; 4], result: [u8; 4], k: f32) -> [u8; 4] {
    if k >= 1.0 { return result; }
    let lerp = |o: u8, r: u8| (o as f32 + (r as f32 - o as f32) * k).round().clamp(0.0, 255.0);
    let alpha = lerp(original[3], result[3]);
    [lerp(original[0], result[0]).min(alpha) as u8, lerp(original[1], result[1]).min(alpha) as u8,
     lerp(original[2], result[2]).min(alpha) as u8, alpha as u8]
}

/// A spatial adjustment's result blended onto the original in `mode` at full coverage, keeping the
/// original's alpha (LiveMaskRenderer.swift:24-46): both made opaque as `layer_unpremultiply_opaque`
/// does (a clear pixel becomes black), blended, then `layer_restore_alpha`.
fn blended_keeping_alpha(mode: BlendMode, original: [u8; 4], adjusted: [u8; 4]) -> [u8; 4] {
    let opaque = |p: [u8; 4]| -> [f32; 3] {
        let a = p[3] as u32;
        [0, 1, 2].map(|c| if a == 0 { 0.0 } else { ((p[c] as u32 * 255 + a / 2) / a).min(255) as f32 / 255.0 })
    };
    let b = blend_rgb(mode, opaque(original), opaque(adjusted));
    let a = original[3] as u32;
    let c = |v: f32| (((v.clamp(0.0, 1.0) * 255.0).round() as u32 * a + 127) / 255) as u8;
    [c(b[0]), c(b[1]), c(b[2]), original[3]]
}

/// A Gaussian or Motion Blur adjustment layer (R 3.4): everything composited so far, blurred as a
/// whole with alpha, then put back where this layer's coverage reaches, weighted by it. Anything
/// outside the canvas is transparent to the blur and takes nothing from it, as the Mac's
/// canvas-sized context is empty there.
fn spatial_target(doc: &Document, plan: &RenderPlan, target: &mut Target, draw: &LayerDraw, adjustment: &LayerAdjustment, blend: BlendMode, use_clip: bool, cache: &EffectsCache) {
    let scale = target.w as f64 / target.region.width;
    let on_canvas = |p: Point| p.x >= 0.0 && p.y >= 0.0 && p.x < doc.width as f64 && p.y < doc.height as f64;
    let mut input = target.data.to_vec();
    for oy in 0..target.h { for ox in 0..target.w {
        if !on_canvas(target.doc_point(ox, oy)) {
            let i = ((oy * target.w + ox) * 4) as usize;
            input[i..i + 4].fill(0);
        }
    }}
    // Handed over, not borrowed: the blur writes its result over this copy where it can, so the
    // target and one working copy are the only full frames held.
    let input = Raster::from_premultiplied(target.w, target.h, input);
    let b = spatial_blur(adjustment, scale);
    let blurred = match adjustment.kind {
        AdjustmentKind::GaussianBlur => blur_for_layer(input, b.sigma),
        AdjustmentKind::MotionBlur => streak_for_layer(input, b.angle, b.distance),
        _ => return,
    };
    let blurred = blurred.bytes();
    let clip_sources = match (use_clip, draw.clip) {
        (true, Some(c)) => clip_source_rasters_xy(doc, plan, c, scale, target.h as f64 / target.region.height, cache),
        _ => SourceRasters::new(),
    };
    for oy in 0..target.h { for ox in 0..target.w {
        let i = ((oy * target.w + ox) * 4) as usize;
        let p = target.doc_point(ox, oy);
        if !on_canvas(p) { continue; }
        let mut k = draw.opacity as f32 * coverages_at(doc, &draw.coverages, p);
        if use_clip { if let Some(c) = draw.clip { k *= source_coverage_at(doc, plan, c, p, &clip_sources); } }
        if k <= 0.0 { continue; }
        let original = [target.data[i], target.data[i + 1], target.data[i + 2], target.data[i + 3]];
        let mut result = [blurred[i], blurred[i + 1], blurred[i + 2], blurred[i + 3]];
        // The layer's own mode is not Normal: full coverage in that mode, original alpha kept
        // (`keeps_alpha`).
        if draw.keeps_alpha { result = blended_keeping_alpha(blend, original, result); }
        target.data[i..i + 4].copy_from_slice(&toward(original, result, k));
    }}
}

fn draw_stack(doc: &Document, plan: &RenderPlan, target: &mut Target, base: &LayerDraw, children: &[LayerDraw], folder: &[Coverage], cache: &EffectsCache) {
    let (w, h) = (target.w, target.h);
    let mut temp = vec![0u8; (w * h * 4) as usize];
    {
        let mut t = Target { data: &mut temp, w, h, region: target.region, enlargement_phases: target.enlargement_phases };
        draw_layer(doc, plan, &mut t, base, BlendMode::Normal, true, cache);
    }
    let base_alpha: Vec<u8> = temp.chunks_exact(4).map(|p| p[3]).collect();
    // The whole surface opaque, a clear pixel becoming opaque black, as layer_unpremultiply_opaque
    // does (BrushPixels.c:23-35) before the children draw (LiveMaskRenderer.swift:97-101). The
    // base alpha put back below zeroes what lies beyond the base again; only a clipped blur can
    // tell, by spreading that black inward.
    for px in temp.chunks_exact_mut(4) {
        let a = px[3] as u32;
        if a > 0 { for c in 0..3 { px[c] = ((px[c] as u32 * 255 + a / 2) / a).min(255) as u8; } }
        px[3] = 255;
    }
    {
        let mut t = Target { data: &mut temp, w, h, region: target.region, enlargement_phases: target.enlargement_phases };
        for child in children { draw_layer(doc, plan, &mut t, child, child.blend, false, cache); }
    }
    // Restore the base alpha, then composite with the base's blend mode under its folder masks.
    for (i, px) in temp.chunks_exact_mut(4).enumerate() {
        let a = base_alpha[i] as u32;
        for c in 0..3 { px[c] = ((px[c] as u32 * a + 127) / 255) as u8; }
        px[3] = a as u8;
    }
    for oy in 0..h { for ox in 0..w {
        let i = ((oy * w + ox) * 4) as usize;
        if temp[i + 3] == 0 { continue; }
        let p = target.doc_point(ox, oy);
        let k = coverages_at(doc, folder, p);
        if k <= 0.0 { continue; }
        let s = [temp[i] as f32 / 255.0 * k, temp[i + 1] as f32 / 255.0 * k, temp[i + 2] as f32 / 255.0 * k, temp[i + 3] as f32 / 255.0 * k];
        compose_u8(&mut target.data[i..i + 4], s, base.blend);
    }}
}

/// Layers to draw, bottom to top: visible, with every ancestor visible, groups excluded.
/// Hierarchy order, like `render_ids`, which it resolves to layers.
pub fn render_layers(doc: &Document) -> Vec<&Layer> {
    doc.render_ids().into_iter().filter_map(|id| doc.layer(id)).collect()
}

/// How a partial render grows its region so its blurs see everything within their reach.
struct Padding { region: Rect, width: u32, height: u32, left: u32, top: u32 }

/// The region grown on each axis by `spatial_span`: by the plan's pad, onto the halving lattice
/// anchored at the canvas's top-left corner, never past the canvas's span rounded out to that
/// lattice (beyond the canvas the Mac's context is empty, R 3.4, and `spatial_target` zeroes it).
/// None when nothing changes: no blur, or a region already on the lattice out to the canvas edges.
fn padding(doc: &Document, plan: &RenderPlan, region: Rect, w: u32, h: u32) -> Option<Padding> {
    if !(plan.spatial_margin > 0.0) || w == 0 || h == 0 || !(region.width > 0.0) || !(region.height > 0.0) { return None; }
    let (sx, sy) = (w as f64 / region.width, h as f64 / region.height);
    let grid = spatial_grid(plan, sx);
    // Whole output pixels from the canvas's corner to the region's. Exact, and so is the lattice,
    // whenever the region starts on the canvas's own output grid, as export, merge, the histogram,
    // the eyedroppers (engine.rs:394, :425, both `floor`ed 1 x 1 regions) and a whole-pixel pan do.
    // Rounded, not floored: the CPU renderer's `k * docPerPx` origin (cpu-renderer.ts:29-31) often
    // lands one ulp below the whole pixel k, which a floor would put one pixel off the lattice.
    let (x0, y0) = ((region.x * sx).round() as i64, (region.y * sy).round() as i64);
    let (xs, xe) = spatial_span(x0, x0 + w as i64, (doc.width as f64 * sx).ceil() as i64, grid.cell, grid.pad);
    let (ys, ye) = spatial_span(y0, y0 + h as i64, (doc.height as f64 * sy).ceil() as i64, grid.cell, grid.pad);
    let (left, top, width, height) = ((x0 - xs) as u32, (y0 - ys) as u32, (xe - xs) as u32, (ye - ys) as u32);
    if left == 0 && top == 0 && width == w && height == h { return None; }
    Some(Padding { region: Rect { x: region.x - left as f64 / sx, y: region.y - top as f64 / sy, width: width as f64 / sx, height: height as f64 / sy },
        width, height, left, top })
}

pub fn composite_plan(doc: &Document, plan: &RenderPlan, region: Rect, out_width: u32, out_height: u32) -> Raster {
    composite_plan_with(doc, plan, region, out_width, out_height, &EffectsCache::default())
}

/// `composite_plan`, finding and keeping effects images in `cache` (the engine's). The functions
/// without `_with` make each image once for their own render and keep none afterwards.
pub fn composite_plan_with(doc: &Document, plan: &RenderPlan, region: Rect, out_width: u32, out_height: u32, cache: &EffectsCache) -> Raster {
    match padding(doc, plan, region, out_width, out_height) {
        Some(p) => composite_region(doc, plan, p.region, p.width, p.height, cache).cropped(p.left, p.top, out_width, out_height),
        None => composite_region(doc, plan, region, out_width, out_height, cache),
    }
}

fn composite_region(doc: &Document, plan: &RenderPlan, region: Rect, out_width: u32, out_height: u32, cache: &EffectsCache) -> Raster {
    let mut data = vec![0u8; (out_width as usize) * (out_height as usize) * 4];
    {
        let mut target = Target { data: &mut data, w: out_width, h: out_height, region, enlargement_phases: true };
        for node in &plan.nodes {
            match node {
                PlanNode::Layer { draw } => draw_layer(doc, plan, &mut target, draw, draw.blend, true, cache),
                PlanNode::Stack { base, children, folder_coverages } => draw_stack(doc, plan, &mut target, base, children, folder_coverages, cache),
            }
        }
    }
    Raster::from_premultiplied(out_width, out_height, data)
}

pub fn composite_edit(doc: &Document, edit: Option<&PreviewEdit>, region: Rect, w: u32, h: u32) -> Raster {
    composite_edit_with(doc, edit, region, w, h, &EffectsCache::default())
}
pub fn composite_edit_with(doc: &Document, edit: Option<&PreviewEdit>, region: Rect, w: u32, h: u32, cache: &EffectsCache) -> Raster {
    composite_plan_with(doc, &render_plan(doc, edit), region, w, h, cache)
}

pub fn composite(doc: &Document, region: Rect, w: u32, h: u32) -> Raster { composite_edit(doc, None, region, w, h) }

/// Draws a single layer with Normal blend and its opacity, ignoring masks and clipping (Image Size resampling).
pub fn render_layer(target: &mut [u8], tw: u32, th: u32, region: Rect, layer: &Layer) {
    let doc = Document { id: Uuid::nil(), width: 1, height: 1, resolution: 72.0, layers: vec![layer.clone()], active_layer_id: None,
        guides: vec![], unknown: Default::default(), selection: None, selection_revision: 1 };
    let plan = RenderPlan { nodes: vec![], sources: vec![], spatial_margin: 0.0 };
    let (pw, ph) = layer.pixels.as_ref().map_or((0, 0), |p| (p.width, p.height));
    let draw = LayerDraw { id: layer.id, transform: layer.transform, corners: None, pixels_width: pw, pixels_height: ph, pixels_revision: layer.pixels_revision,
        opacity: layer.opacity.clamp(0.0, 1.0), blend: BlendMode::Normal, keeps_alpha: false, coverages: vec![], clip: None, adjustment: None, effects: None };
    // Image Size materializes layer pixels and masks using its existing resampling
    // behavior; display/export enlargement phases do not change that operation.
    let mut t = Target { data: target, w: tw, h: th, region, enlargement_phases: false };
    draw_layer(&doc, &plan, &mut t, &draw, BlendMode::Normal, false, &EffectsCache::default());
}

fn check_export_size(doc: &Document) -> Result<(), ExportError> {
    let w = doc.width as i64; let h = doc.height as i64;
    if !(1..=MAX_SIDE).contains(&w) || !(1..=MAX_SIDE).contains(&h) || (w * h) as u64 > MAX_PIXELS { return Err(ExportError::TooLarge); }
    Ok(())
}

pub fn render_full(doc: &Document) -> Result<Raster, ExportError> { render_full_with(doc, &EffectsCache::default()) }
pub fn render_full_with(doc: &Document, cache: &EffectsCache) -> Result<Raster, ExportError> {
    check_export_size(doc)?;
    Ok(composite_edit_with(doc, None, Rect { x: 0.0, y: 0.0, width: doc.width as f64, height: doc.height as f64 }, doc.width, doc.height, cache))
}

pub fn export_png(doc: &Document) -> Result<Vec<u8>, ExportError> { export_png_with(doc, &EffectsCache::default()) }
pub fn export_png_with(doc: &Document, cache: &EffectsCache) -> Result<Vec<u8>, ExportError> {
    let raster = render_full_with(doc, cache)?;
    encode_png(&raster, doc.resolution).map_err(|_| ExportError::Encode)
}

pub fn export_jpeg(doc: &Document, quality: f64, matte: [f64; 3]) -> Result<Vec<u8>, ExportError> { export_jpeg_with(doc, quality, matte, &EffectsCache::default()) }
pub fn export_jpeg_with(doc: &Document, quality: f64, matte: [f64; 3], cache: &EffectsCache) -> Result<Vec<u8>, ExportError> {
    let raster = render_full_with(doc, cache)?;
    encode_jpeg(&raster, quality, matte, doc.resolution)
}

/// Composites the document scaled to fit `max_side` (aspect preserved) and encodes it as JPEG.
/// For a quality/matte preview where a full-resolution encode would freeze the UI on a large
/// canvas: the compositor already accepts an arbitrary output size, so the preview is rendered
/// directly at that size instead of downscaling a full-resolution raster afterward.
pub fn export_jpeg_preview(doc: &Document, quality: f64, matte: [f64; 3], max_side: u32) -> Result<Vec<u8>, ExportError> {
    export_jpeg_preview_with(doc, quality, matte, max_side, &EffectsCache::default())
}
pub fn export_jpeg_preview_with(doc: &Document, quality: f64, matte: [f64; 3], max_side: u32, cache: &EffectsCache) -> Result<Vec<u8>, ExportError> {
    check_export_size(doc)?;
    let max_side = max_side.max(1) as f64;
    let longest = (doc.width.max(doc.height)) as f64;
    let scale = (max_side / longest).min(1.0);
    let out_width = ((doc.width as f64 * scale).round() as u32).max(1);
    let out_height = ((doc.height as f64 * scale).round() as u32).max(1);
    let region = Rect { x: 0.0, y: 0.0, width: doc.width as f64, height: doc.height as f64 };
    let raster = composite_edit_with(doc, None, region, out_width, out_height, cache);
    encode_jpeg(&raster, quality, matte, doc.resolution)
}
