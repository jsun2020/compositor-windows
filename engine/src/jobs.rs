//! Jobs (Phase 4b-1): one layer taken out of a document and worked on by a second engine, the app's
//! job worker, off the UI thread. A job reads nothing but its input: the canvas's size, the selection,
//! and the layer itself (its record as the manifest writes it, with its pixel and mask buffers sent
//! beside the JSON). Three kinds: an edit (a destructive `Command` on that layer), whose result is put
//! back as one undo step only if the layer is still exactly what the job took (`LayerStamp`), as the
//! Mac installs a detached result (SelectionEdits.swift:108-115); a histogram; and an effects image.
use crate::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Said when a job's result finds its layer changed since the job took it.
pub const LAYER_CHANGED: &str = "The layer changed while the edit was being made, so it was not applied.";

/// What the document and layer were when a job took it: the edit goes back only onto exactly this.
/// Beside the layer's own revisions and transform, the canvas's size (a Canvas Size or Crop with an
/// anchor that leaves this layer's transform and pixels untouched, e.g. top-left at (0, 0), still
/// changes what the job's rectangles and any later paint mean) and the selection's revision (a
/// selection-limited job clipped to a selection that has since changed must not land).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayerStamp {
    pub pixels_revision: u64,
    pub mask_revision: u64,
    pub transform: LayerTransform,
    pub width: u32,
    pub height: u32,
    pub selection_revision: u64,
}

impl LayerStamp {
    pub fn of(doc: &Document, layer: &Layer) -> LayerStamp {
        LayerStamp {
            pixels_revision: layer.pixels_revision, mask_revision: layer.mask_revision, transform: layer.transform,
            width: doc.width, height: doc.height, selection_revision: doc.selection_revision,
        }
    }
}

/// The selection a job clips to: the antialiasing, the feather and each contour's point count, as
/// JSON (small); the points themselves travel as a flat buffer beside the pixels and mask (twice per
/// point: x then y, in the selection's subpixel units), never JSON. A wand's outline can hold millions
/// of points - tens of megabytes of JSON copied on the UI thread for every job - so a job's selection
/// crosses the wasm boundary the way its pixels do, as bytes a typed array reads without re-parsing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobSelection { pub contour_lengths: Vec<u32>, pub antialiased: bool, pub feather: f64 }

impl JobSelection {
    /// How many `i32`s the contours' points make (`flatten`'s length): two per point.
    pub fn point_count(&self) -> usize { self.contour_lengths.iter().map(|&n| n as usize).sum::<usize>() * 2 }
    /// The contours' points as a flat buffer: one contour after another, each point's x then y.
    pub fn flatten(contours: &[Contour]) -> Vec<i32> { contours.iter().flatten().flat_map(|p| [p[0], p[1]]).collect() }
    /// The contours a flat buffer of `flatten`'s shape holds, split back up by `contour_lengths`.
    pub fn unflatten(&self, points: &[i32]) -> Result<Vec<Contour>, CommandError> {
        if points.len() != self.point_count() { return Err(CommandError::Argument("a job's selection does not match its point count".into())); }
        let mut out = Vec::with_capacity(self.contour_lengths.len());
        let mut at = 0;
        for &n in &self.contour_lengths {
            let n = n as usize;
            out.push(points[at..at + n * 2].chunks_exact(2).map(|c| [c[0], c[1]]).collect());
            at += n * 2;
        }
        Ok(out)
    }
}

/// A job's input, beside its buffers: the canvas, the selection, the layer's record and the sizes of
/// its pixel and mask buffers (None where it has none), and the stamp an edit must find again.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobInput {
    pub width: u32,
    pub height: u32,
    pub selection: Option<JobSelection>,
    pub layer: LayerRecord,
    pub pixels: Option<(u32, u32)>,
    pub mask: Option<(u32, u32)>,
    pub stamp: LayerStamp,
}

/// What an edit left of its layer: the transform, the mask's placement, which buffers it replaced
/// (their sizes; the buffers travel beside), the rectangles it changed them within, and the new
/// pixels already halved to the level the canvas draws them at (`display`: the level and the size;
/// the halved buffer travels beside, the fourth job buffer, F1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobOutput {
    pub transform: LayerTransform,
    pub mask_placement: Option<LayerTransform>,
    pub pixels: Option<(u32, u32)>,
    pub mask: Option<(u32, u32)>,
    pub regions: Vec<(Plane, PixelRect)>,
    #[serde(default)]
    pub display: Option<DisplayHalving>,
    /// A checked changed pixel avoids comparing two entire large buffers on the UI thread.
    #[serde(default)] pub witness: Option<ChangeWitness>,
}
#[derive(Clone,Copy,Debug,PartialEq,Eq,Serialize,Deserialize)]
pub struct ChangeWitness {pub x:u32,pub y:u32,pub before:[u8;4],pub after:[u8;4]}

/// The new pixels after `level` halvings (`width` x `height`): what the canvas uploads at the zoom the
/// edit was asked at, made in the worker so the UI thread never halves a large result (F1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisplayHalving { pub level: u32, pub width: u32, pub height: u32 }

/// An effects image made by a job: its size and the transparent pixels added on every side.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct EffectsImage { pub width: u32, pub height: u32, pub inset: u32 }

impl JobInput {
    /// The document a job works on: the canvas, the selection and the one layer, active. The layer
    /// keeps its id, so a command names it as the app did. `points` is the selection's flat buffer
    /// (`JobSelection::flatten`'s shape), required exactly when `self.selection` is.
    pub(crate) fn document(&self, pixels: Option<Raster>, mask: Option<GrayRaster>, points: Option<&[i32]>) -> Result<Document, CommandError> {
        if pixels.as_ref().map(|p| (p.width, p.height)) != self.pixels || mask.as_ref().map(|m| (m.width, m.height)) != self.mask {
            return Err(CommandError::Argument("a job's buffers do not match its input".into()));
        }
        let mut doc = Document::new(self.width, self.height);
        let mut layer = Layer::from_record(&self.layer, pixels, mask);
        // A layer inside a folder keeps its id and pixels; the folder itself is not sent.
        layer.parent_id = None;
        layer.mask_source_id = None;
        doc.active_layer_id = Some(layer.id);
        doc.layers = vec![layer];
        doc.selection = match (&self.selection, points) {
            (Some(s), Some(pts)) => Some(Selection::new(s.unflatten(pts)?, s.antialiased, s.feather)),
            (None, None) => None,
            _ => return Err(CommandError::Argument("a job's selection does not match its input".into())),
        };
        Ok(doc)
    }
}

impl Engine {
    /// A job's input for `layer`, and its own pixel, mask and selection-point buffers (shared: the
    /// wasm bridge copies them out once). The stored document, never an open preview: a job edits
    /// what is committed.
    pub fn job_input(&self, id: Uuid, layer: Uuid) -> Result<(JobInput, Option<Raster>, Option<GrayRaster>, Option<Vec<i32>>), CommandError> {
        let doc = &self.session(id)?.document;
        let l = doc.layer(layer).ok_or(CommandError::NoLayer)?;
        let (selection, points) = match &doc.selection {
            Some(s) => (
                Some(JobSelection { contour_lengths: s.contours.iter().map(|c| c.len() as u32).collect(), antialiased: s.antialiased, feather: s.feather }),
                Some(JobSelection::flatten(&s.contours)),
            ),
            None => (None, None),
        };
        let input = JobInput {
            width: doc.width, height: doc.height,
            selection,
            layer: l.record(),
            pixels: l.pixels.as_ref().map(|p| (p.width, p.height)),
            mask: l.mask.as_ref().map(|m| (m.pixels.width, m.pixels.height)),
            stamp: LayerStamp::of(doc, l),
        };
        Ok((input, l.pixels.clone(), l.mask.as_ref().map(|m| m.pixels.clone()), points))
    }

    /// An effects job's input for `layer` as the canvas shows it (through any open preview), its
    /// pixels after `level` halvings (the Mac previews effects from a reduced copy); the header names
    /// that reduced size. An effects job never clips to a selection.
    pub fn display_job_input(&self, id: Uuid, layer: Uuid, level: u32) -> Result<(JobInput, Raster, Option<GrayRaster>), CommandError> {
        let doc = self.render_document(id)?;
        let l = doc.layer(layer).ok_or(CommandError::NoLayer)?;
        let pixels = self.layer_raster(id, layer, level)?.ok_or_else(|| CommandError::Argument("the layer has no pixels".into()))?;
        let input = JobInput {
            width: doc.width, height: doc.height, selection: None, layer: l.record(),
            pixels: Some((pixels.width, pixels.height)), mask: l.mask.as_ref().map(|m| (m.pixels.width, m.pixels.height)),
            stamp: LayerStamp::of(&doc, l),
        };
        Ok((input, pixels, l.mask.as_ref().map(|m| m.pixels.clone())))
    }

    /// Whether the canvas's effects image for `layer` (with the pending `edit`) is already made: the
    /// renderer then draws it at once (`draw_raster` finds it); otherwise it asks the job worker.
    pub fn has_effects_image(&self, id: Uuid, layer: Uuid, edit: Option<&PreviewEdit>) -> Result<bool, CommandError> {
        let doc = self.render_document(id)?;
        let l = doc.layer(layer).ok_or(CommandError::NoLayer)?;
        Ok(effects_draw(l, edit).is_some_and(|draw| self.effects_cache().contains(l, &draw)))
    }
    /// Keeps an effects image the job worker made at full size for `layer` as the canvas showed it
    /// (with `edit`), in the engine's cache, where `draw_raster` finds it as if made here. Fix round
    /// 1, issue 2: the whole `LayerStamp` (transform, canvas size, selection revision) used to be
    /// compared, but none of those change the effects image, so a move or a new selection made while
    /// the job ran refused a perfectly good result. Only `pixels_revision` and `mask_revision` (what
    /// the job's buffers actually came from) are compared now, together with `key`, the job's own
    /// `EffectsDraw.key` at ask time (not `effects-images.ts`'s own compound `bytesKey`, which the
    /// engine has never heard of): comparing it against what `effects_draw(l, edit)` gives *now*
    /// catches an effects (or mask-placement) change the two revisions alone would miss, even one
    /// that happens to leave the padding (`inset`) the same.
    /// Otherwise nothing is kept and false comes back.
    pub fn keep_effects_image(&self, id: Uuid, layer: Uuid, stamp: LayerStamp, key: &str, edit: Option<&PreviewEdit>, image: Raster) -> Result<bool, CommandError> {
        let doc = self.render_document(id)?;
        let l = doc.layer(layer).ok_or(CommandError::NoLayer)?;
        if l.pixels_revision != stamp.pixels_revision || l.mask_revision != stamp.mask_revision { return Ok(false); }
        let (Some(draw), Some(pixels)) = (effects_draw(l, edit), l.pixels.as_ref()) else { return Ok(false) };
        if draw.key != key { return Ok(false); }
        if (image.width, image.height) != (pixels.width + 2 * draw.inset, pixels.height + 2 * draw.inset) { return Ok(false); }
        self.effects_cache().insert(l, &draw, image);
        Ok(true)
    }

    /// Puts an edit job's result back on `layer` as one undo step: the buffers it replaced, its
    /// transform and mask placement, recorded as changed within its regions. Refused, with the
    /// document untouched, unless the layer, the canvas and the selection are still exactly what the
    /// job took (`stamp`), and unless the project still fits its budget with the result in it.
    ///
    /// The stamp covers the mask's placement through `mask_revision`: every change of a placement
    /// goes through `Layer::mask_mut`, which moves the revision, so a mask moved while the job ran
    /// (a nudge, a Transform of the mask alone, an undo of either) refuses the result as a pixel edit
    /// would.
    ///
    /// `display` is the new pixels already halved to the canvas's level (`output.display`), which the
    /// new pixels adopt (`Raster::adopt`): the next frame uploads it without halving the layer on the
    /// UI thread (F1).
    pub fn install_job(&mut self, id: Uuid, layer: Uuid, stamp: LayerStamp, output: JobOutput, pixels: Option<Raster>, mask: Option<GrayRaster>, display: Option<Raster>) -> Result<Dirty, CommandError> {
        if pixels.as_ref().map(|p| (p.width, p.height)) != output.pixels || mask.as_ref().map(|m| (m.width, m.height)) != output.mask
            || display.as_ref().map(|d| (d.width, d.height)) != output.display.map(|d| (d.width, d.height)) {
            return Err(CommandError::Argument("a job's buffers do not match its output".into()));
        }
        if let (Some(p), Some(d), Some(spec)) = (&pixels, &display, output.display) {
            if !p.adopt(spec.level, d.clone()) { return Err(CommandError::Argument("a job's display halving does not match its pixels".into())); }
        }
        let confirmed=output.witness.is_some_and(|w|{
            let old=self.document(id).and_then(|d|d.layer(layer)).and_then(|l|l.pixels.as_ref());
            match (old,pixels.as_ref()){
                (Some(a),Some(b)) if (a.width,a.height)==(b.width,b.height)&&w.x<a.width&&w.y<a.height=>output.regions.iter().any(|(plane,r)|*plane==Plane::Pixels&&w.x>=r.x&&w.y>=r.y&&w.x<r.x.saturating_add(r.width)&&w.y<r.y.saturating_add(r.height))&&w.before!=w.after&&a.pixel(w.x,w.y)==w.before&&b.pixel(w.x,w.y)==w.after,
                _=>false,
            }
        });
        self.clear_preview(id);
        self.edit_with_change(id,confirmed, |doc, _| {
            let now = LayerStamp::of(doc, doc.layer(layer).ok_or(CommandError::NoLayer)?);
            if now != stamp { return Err(CommandError::Refused(LAYER_CHANGED.into())); }
            let l = doc.layer_mut(layer).ok_or(CommandError::NoLayer)?;
            if let Some(p) = pixels { l.set_pixels(Some(p)); }
            // Most commands leave the transform exactly as the job found it; a filter that spreads
            // past the layer's edges (GaussianBlur) legitimately grows it. Keeping the layer's own
            // transform when the job reports the same one is defence in depth: serde_json's
            // `float_roundtrip` already brings the output's transform back exactly.
            if output.transform != stamp.transform { l.transform = output.transform; }
            if let Some(m) = mask {
                let target = l.mask_mut().ok_or_else(|| CommandError::Argument("the layer has no mask".into()))?;
                target.pixels = m;
                target.placement = output.mask_placement;
            }
            // The job's document held this one layer, so its own budget checks (`image_grid`, the
            // covering mask's growth in `paint_layer`, a spreading filter's grown grid) counted none of
            // the others: the project's budget is enforced here, with the result in place, as every
            // edit made on the UI thread enforces it. A project over it would not open again
            // (package.rs, OverBudget). `edit` works on a copy, so the refusal leaves the document as
            // it was and records nothing.
            if doc.used_pixels() > MAX_PIXELS || doc.used_mask_pixels() > MAX_PIXELS {
                return Err(CommandError::Project(ProjectError::TooLarge));
            }
            let regions = output.regions.iter().map(|(plane, rect)| Region { layer, plane: *plane, rect: *rect }).collect();
            Ok(Dirty::pixels(vec![layer]).within(regions))
        })
    }
}

/// Runs an edit job: `command` on the job's document, and what it left of the layer. Only the
/// buffers the command replaced come back. `points` is the selection's flat buffer, as `job_input`
/// returns it; required exactly when `input.selection` is.
///
/// `out_per_doc` is the scale the canvas draws the document at (device pixels per document pixel; 0
/// for none): new pixels that changed as a whole are also returned halved to the level the canvas
/// uploads them at (`compositor::prefilter_level`, the renderers' rule, on the result's own grid, so
/// a blank layer a fill grew to the canvas counts too), which `install_job` hands them to adopt (F1).
/// None when that level is 0, for Nearest sampling (never prefiltered), and when the edit reports
/// changed rectangles (the UI thread then seeds the halvings from the old pixels).
pub fn run_edit_job(input: &JobInput, pixels: Option<Raster>, mask: Option<GrayRaster>, points: Option<&[i32]>, command: Command, out_per_doc: f64) -> Result<(JobOutput, Option<Raster>, Option<GrayRaster>, Option<Raster>), CommandError> {
    run_document_edit(input.document(pixels.clone(),mask.clone(),points)?,input.layer.id,pixels,mask,command,out_per_doc)
}
pub fn run_document_edit(document: Document,layer:Uuid,pixels:Option<Raster>,mask:Option<GrayRaster>,command:Command,out_per_doc:f64)->Result<(JobOutput,Option<Raster>,Option<GrayRaster>,Option<Raster>),CommandError>{
    let mut engine = Engine::new();
    let id = engine.insert_document(document);
    let dirty = engine.execute(id, command)?;
    edit_output(engine.document(id).ok_or(CommandError::NoDocument)?,layer,pixels,mask,dirty,out_per_doc)
}

/// Common output/witness/display preparation, also for checked GPU results.
pub(crate) fn edit_output(document:&Document,layer:Uuid,pixels:Option<Raster>,mask:Option<GrayRaster>,dirty:Dirty,out_per_doc:f64)->Result<(JobOutput,Option<Raster>,Option<GrayRaster>,Option<Raster>),CommandError>{
    let l = document.layer(layer).ok_or(CommandError::NoLayer)?;
    let new_pixels = l.pixels.clone().filter(|p| !pixels.as_ref().is_some_and(|o| o.same_pixels(p)));
    let new_mask = l.mask.as_ref().map(|m| m.pixels.clone()).filter(|m| !mask.as_ref().is_some_and(|o| o.same_pixels(m)));
    let regions: Vec<(Plane, PixelRect)> = dirty.regions.iter().filter(|r| r.layer == layer).map(|r| (r.plane, r.rect)).collect();
    // Small partial edits keep the established incremental cache path. At more
    // than 4 MP, copying that full cached ladder can itself block the UI, so the
    // worker supplies the requested display level even for a local rectangle.
    let display = new_pixels.as_ref().filter(|p| out_per_doc > 0.0 && l.transform.sampling != Sampling::Nearest &&
        (p.width as u64*p.height as u64>4_000_000 || !regions.iter().any(|(plane,_)|*plane==Plane::Pixels)))
        .map(|p| (compositor::prefilter_level(p.width, p.height, p.width as f64 / (l.transform.size.width * out_per_doc).max(1e-9)), p))
        .filter(|(level, _)| *level > 0)
        .map(|(level, p)| (level, p.reduced(level)));
    let witness=match (&pixels,&new_pixels){
        (Some(before),Some(after)) if (before.width,before.height)==(after.width,after.height)=>{
            regions.iter().filter(|(p,_)|*p==Plane::Pixels).find_map(|(_,r)|{
                for y in r.y..(r.y+r.height).min(before.height){for x in r.x..(r.x+r.width).min(before.width){let(a,b)=(before.pixel(x,y),after.pixel(x,y));if a!=b{return Some(ChangeWitness{x,y,before:a,after:b});}}}None
            })
        },_=>None,
    };
    let output = JobOutput {
        transform: l.transform,
        mask_placement: l.mask.as_ref().and_then(|m| m.placement),
        pixels: new_pixels.as_ref().map(|p| (p.width, p.height)),
        mask: new_mask.as_ref().map(|m| (m.width, m.height)),
        regions,
        display: display.as_ref().map(|(level, d)| DisplayHalving { level: *level, width: d.width, height: d.height }),
        witness,
    };
    Ok((output, new_pixels, new_mask, display.map(|(_, d)| d)))
}

/// Headers travel as small JSON; every tile's RGBA bytes travel separately.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WarpTileRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

pub(crate) fn check_warp_canvas(input: &JobInput, mask_target: bool) -> Result<(), CommandError> {
    if mask_target {
        return Err(CommandError::Refused(ops::warp::MASK_REFUSAL.into()));
    }
    if input.width == 0
        || input.height == 0
        || input.width as i64 > MAX_SIDE
        || input.height as i64 > MAX_SIDE
        || input.width as u64 * input.height as u64 > MAX_PIXELS
    {
        return Err(CommandError::Argument("invalid warp canvas".into()));
    }
    Ok(())
}

/// Prepare the unstyled document-space source off the UI thread. The caller
/// retains the original JobInput/stamp and buffers for final replacement.
pub fn run_warp_source_job(
    input: &JobInput,
    pixels: Option<Raster>,
    mask: Option<GrayRaster>,
    points: Option<&[i32]>,
    mask_target: bool,
) -> Result<Raster, CommandError> {
    check_warp_canvas(input, mask_target)?;
    ops::warp::source_plane(&input.document(pixels, mask, points)?, input.layer.id)
}

/// A completed GPU stroke follows the same selection/grid replacement and
/// output protocol as the reference. No CPU warp kernel runs here. Nothing
/// is installed on the main engine until its original LayerStamp is checked.
pub fn run_warp_result_job(
    input: &JobInput,
    pixels: Option<Raster>,
    mask: Option<GrayRaster>,
    points: Option<&[i32]>,
    mask_target: bool,
    spec: &WarpSpec,
    tiles: Vec<(WarpTileRect, Vec<u8>)>,
    out_per_doc: f64,
) -> Result<
    (
        JobOutput,
        Option<Raster>,
        Option<GrayRaster>,
        Option<Raster>,
    ),
    CommandError,
> {
    check_warp_canvas(input, mask_target)?;
    let dabs = ops::warp::footprint(spec)?;
    check_warp_tiles(input.width, input.height, &tiles)?;
    let mut document = input.document(pixels.clone(), mask.clone(), points)?;
    // Still validate the layer for no-op strokes.
    ops::warp::target(&document, input.layer.id, false)?;
    let dirty = if dabs.is_empty() || tiles.is_empty() {
        Dirty::pixels(vec![])
    } else {
        let mut bytes = ops::warp::source_plane(&document, input.layer.id)?.into_bytes();
        overlay_warp_tiles(&mut bytes, input.width, tiles);
        let result = Raster::from_premultiplied(input.width, input.height, bytes);
        ops::warp::writeback(&mut document, &SelectionClips::default(), input.layer.id, spec, &dabs, &result, false)?
    };
    edit_output(&document, input.layer.id, pixels, mask, dirty, out_per_doc)
}

pub(crate) fn check_warp_tiles(width: u32, height: u32, tiles: &[(WarpTileRect, Vec<u8>)]) -> Result<(), CommandError> {
    // Bound JSON headers and total readback independently of canvas size.
    if tiles.len() > 4096 {
        return Err(CommandError::Argument("too many warp tiles".into()));
    }
    let mut count = 0u64;
    for (i, (rect, bytes)) in tiles.iter().enumerate() {
        let area = rect.width as u64 * rect.height as u64;
        count = count
            .checked_add(area)
            .ok_or_else(|| CommandError::Argument("invalid warp tiles".into()))?;
        if rect.width == 0
            || rect.height == 0
            || rect.x as u64 + rect.width as u64 > width as u64
            || rect.y as u64 + rect.height as u64 > height as u64
            || count > 16_777_216
            || area * 4 != bytes.len() as u64
            || bytes
                .chunks_exact(4)
                .any(|p| p[..3].iter().any(|v| *v > p[3]))
        {
            return Err(CommandError::Argument("invalid warp tiles".into()));
        }
        if tiles[..i].iter().any(|(other, _)| {
            rect.x < other.x + other.width
                && other.x < rect.x + rect.width
                && rect.y < other.y + other.height
                && other.y < rect.y + rect.height
        }) {
            return Err(CommandError::Argument("overlapping warp tiles".into()));
        }
    }
    Ok(())
}

pub(crate) fn overlay_warp_tiles(bytes: &mut [u8], width: u32, tiles: Vec<(WarpTileRect, Vec<u8>)>) {
    for (rect, tile) in tiles {
        for y in 0..rect.height as usize {
            let start = ((rect.y as usize + y) * width as usize + rect.x as usize) * 4;
            let row = rect.width as usize * 4;
            bytes[start..start + row].copy_from_slice(&tile[y * row..(y + 1) * row]);
        }
    }
}

/// Runs a histogram job: the layer's histogram weighted by the selection (`Engine::histogram`).
/// `points` as `run_edit_job` takes it.
pub fn run_histogram_job(input: &JobInput, pixels: Option<Raster>, mask: Option<GrayRaster>, points: Option<&[i32]>) -> Result<Vec<Vec<f64>>, CommandError> {
    let mut engine = Engine::new();
    let id = engine.insert_document(input.document(pixels, mask, points)?);
    engine.histogram(id, input.layer.id)
}

/// Runs an effects job: the layer drawn with its effects (`effects_image`), from `pixels` that may be
/// the layer's own reduced `factor` times (its halvings), the effects scaled with them, as the Mac's
/// canvas preview does (EffectsPreviewCache.swift:114-133). None when the layer draws no effects. An
/// effects job never clips to a selection (`display_job_input`), so it sends no points.
pub fn run_effects_job(input: &JobInput, pixels: Raster, mask: Option<GrayRaster>, factor: f64, edit: Option<&PreviewEdit>) -> Result<Option<(EffectsImage, Raster)>, CommandError> {
    let mut reduced = input.clone();
    reduced.pixels = Some((pixels.width, pixels.height));
    let doc = reduced.document(Some(pixels), mask, None)?;
    let mut layer = doc.layers[0].clone();
    if factor != 1.0 { layer.extra.effects = layer.extra.effects.map(|e| e.scaled(factor)); }
    let Some(draw) = effects_draw(&layer, edit) else { return Ok(None) };
    let image = effects_image(&layer, layer.pixels.as_ref().unwrap(), &draw);
    Ok(Some((EffectsImage { width: image.width, height: image.height, inset: draw.inset }, image)))
}
