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
    fn unflatten(&self, points: &[i32]) -> Result<Vec<Contour>, CommandError> {
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
/// (their sizes; the buffers travel beside) and the rectangles it changed them within.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobOutput {
    pub transform: LayerTransform,
    pub mask_placement: Option<LayerTransform>,
    pub pixels: Option<(u32, u32)>,
    pub mask: Option<(u32, u32)>,
    pub regions: Vec<(Plane, PixelRect)>,
}

/// An effects image made by a job: its size and the transparent pixels added on every side.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct EffectsImage { pub width: u32, pub height: u32, pub inset: u32 }

impl JobInput {
    /// The document a job works on: the canvas, the selection and the one layer, active. The layer
    /// keeps its id, so a command names it as the app did. `points` is the selection's flat buffer
    /// (`JobSelection::flatten`'s shape), required exactly when `self.selection` is.
    fn document(&self, pixels: Option<Raster>, mask: Option<GrayRaster>, points: Option<&[i32]>) -> Result<Document, CommandError> {
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
    /// (with `edit`), in the engine's cache, where `draw_raster` finds it as if made here. Only while
    /// the layer is still what the job took (`stamp`), draws effects, and needs an image of this size:
    /// otherwise nothing is kept and false comes back.
    pub fn keep_effects_image(&self, id: Uuid, layer: Uuid, stamp: LayerStamp, edit: Option<&PreviewEdit>, image: Raster) -> Result<bool, CommandError> {
        let doc = self.render_document(id)?;
        let l = doc.layer(layer).ok_or(CommandError::NoLayer)?;
        if LayerStamp::of(&doc, l) != stamp { return Ok(false); }
        let (Some(draw), Some(pixels)) = (effects_draw(l, edit), l.pixels.as_ref()) else { return Ok(false) };
        if (image.width, image.height) != (pixels.width + 2 * draw.inset, pixels.height + 2 * draw.inset) { return Ok(false); }
        self.effects_cache().insert(l, &draw, image);
        Ok(true)
    }

    /// Puts an edit job's result back on `layer` as one undo step: the buffers it replaced, its
    /// transform and mask placement, recorded as changed within its regions. Refused, with the
    /// document untouched, unless the layer, the canvas and the selection are still exactly what the
    /// job took (`stamp`).
    pub fn install_job(&mut self, id: Uuid, layer: Uuid, stamp: LayerStamp, output: JobOutput, pixels: Option<Raster>, mask: Option<GrayRaster>) -> Result<Dirty, CommandError> {
        if pixels.as_ref().map(|p| (p.width, p.height)) != output.pixels || mask.as_ref().map(|m| (m.width, m.height)) != output.mask {
            return Err(CommandError::Argument("a job's buffers do not match its output".into()));
        }
        self.clear_preview(id);
        self.edit(id, |doc, _| {
            let now = LayerStamp::of(doc, doc.layer(layer).ok_or(CommandError::NoLayer)?);
            if now != stamp { return Err(CommandError::Refused(LAYER_CHANGED.into())); }
            let l = doc.layer_mut(layer).ok_or(CommandError::NoLayer)?;
            if let Some(p) = pixels { l.set_pixels(Some(p)); }
            // Most commands leave the transform exactly as the job found it; a filter that spreads
            // past the layer's edges (GaussianBlur) legitimately grows it. Writing `output.transform`
            // back unconditionally, even when a job's stamp still matches (nothing moved it), used to
            // risk a spurious ULP drift on every install through JSON's lossy float parsing without
            // serde_json's `float_roundtrip` feature; comparing first keeps an untouched transform
            // bit-identical regardless.
            if output.transform != stamp.transform { l.transform = output.transform; }
            if let Some(m) = mask {
                let target = l.mask_mut().ok_or_else(|| CommandError::Argument("the layer has no mask".into()))?;
                target.pixels = m;
                target.placement = output.mask_placement;
            }
            let regions = output.regions.iter().map(|(plane, rect)| Region { layer, plane: *plane, rect: *rect }).collect();
            Ok(Dirty::pixels(vec![layer]).within(regions))
        })
    }
}

/// Runs an edit job: `command` on the job's document, and what it left of the layer. Only the
/// buffers the command replaced come back. `points` is the selection's flat buffer, as `job_input`
/// returns it; required exactly when `input.selection` is.
pub fn run_edit_job(input: &JobInput, pixels: Option<Raster>, mask: Option<GrayRaster>, points: Option<&[i32]>, command: Command) -> Result<(JobOutput, Option<Raster>, Option<GrayRaster>), CommandError> {
    let mut engine = Engine::new();
    let id = engine.insert_document(input.document(pixels.clone(), mask.clone(), points)?);
    let dirty = engine.execute(id, command)?;
    let l = engine.document(id).and_then(|d| d.layer(input.layer.id)).ok_or(CommandError::NoLayer)?;
    let new_pixels = l.pixels.clone().filter(|p| !pixels.as_ref().is_some_and(|o| o.same_pixels(p)));
    let new_mask = l.mask.as_ref().map(|m| m.pixels.clone()).filter(|m| !mask.as_ref().is_some_and(|o| o.same_pixels(m)));
    let output = JobOutput {
        transform: l.transform,
        mask_placement: l.mask.as_ref().and_then(|m| m.placement),
        pixels: new_pixels.as_ref().map(|p| (p.width, p.height)),
        mask: new_mask.as_ref().map(|m| (m.width, m.height)),
        regions: dirty.regions.iter().filter(|r| r.layer == input.layer.id).map(|r| (r.plane, r.rect)).collect(),
    };
    Ok((output, new_pixels, new_mask))
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
