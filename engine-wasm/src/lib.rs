use compositor_engine::*;
use js_sys::{Array, Uint8Array};
use std::collections::HashMap;
use uuid::Uuid;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmEngine {
    engine: Engine, pending_saves: HashMap<Uuid, Package>, drawn: Option<Raster>,
    /// A job's pixel, mask and selection-point buffers, kept while the app copies them out: a job's
    /// input on the main thread, a job's result in the worker (`job_buffer_ptr`, `job_points_ptr`,
    /// `release_job`).
    job: (Option<Raster>, Option<GrayRaster>, Option<Vec<i32>>),
}

/// A buffer's bytes as a raster, when its size is known.
fn raster_of(size: Option<(u32, u32)>, bytes: Option<Vec<u8>>) -> Result<Option<Raster>, JsError> {
    match (size, bytes) {
        (Some((w, h)), Some(b)) if b.len() == (w as usize) * (h as usize) * 4 => Ok(Some(Raster::from_premultiplied(w, h, b))),
        (None, None) => Ok(None),
        _ => Err(JsError::new("a job's pixels do not match its size")),
    }
}
fn gray_of(size: Option<(u32, u32)>, bytes: Option<Vec<u8>>) -> Result<Option<GrayRaster>, JsError> {
    match (size, bytes) {
        (Some((w, h)), Some(b)) if b.len() == (w as usize) * (h as usize) => Ok(Some(GrayRaster::from_bytes(w, h, b))),
        (None, None) => Ok(None),
        _ => Err(JsError::new("a job's mask does not match its size")),
    }
}
/// A job's selection points, from little-endian bytes (as `job_points_ptr` hands them out): `count`
/// is `JobSelection::point_count()`, the number of `i32`s the points make, not the byte length.
fn points_of(count: Option<usize>, bytes: Option<Vec<u8>>) -> Result<Option<Vec<i32>>, JsError> {
    match (count, bytes) {
        (Some(n), Some(b)) if b.len() == n * 4 => Ok(Some(b.chunks_exact(4).map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect())),
        (None, None) => Ok(None),
        _ => Err(JsError::new("a job's selection does not match its point count")),
    }
}

fn js_err<E: std::fmt::Display>(e: E) -> JsError { JsError::new(&e.to_string()) }
fn parse_id(text: &str) -> Result<Uuid, JsError> { Uuid::parse_str(text).map_err(js_err) }

#[wasm_bindgen]
impl WasmEngine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmEngine {
        console_error_panic_hook::set_once();
        WasmEngine { engine: Engine::new(), pending_saves: HashMap::new(), drawn: None, job: (None, None, None) }
    }

    // Jobs (Phase 4b-1, engine `jobs.rs`). On the main thread: `prepare_job` or `prepare_display_job`,
    // copy the buffers out, `release_job`; later `install_job`. In the worker: a `run_*_job`, copy the
    // result buffers out, `release_job`.

    /// `Engine::job_input` as JSON; the layer's pixels, mask and the selection's points (if any) are
    /// kept for `job_buffer_ptr` / `job_points_ptr`.
    pub fn prepare_job(&mut self, doc: &str, layer: &str) -> Result<String, JsError> {
        let (input, pixels, mask, points) = self.engine.job_input(parse_id(doc)?, parse_id(layer)?).map_err(js_err)?;
        self.job = (pixels, mask, points);
        serde_json::to_string(&input).map_err(js_err)
    }
    /// `Engine::display_job_input` (an effects job's input at `level` halvings) as JSON. An effects
    /// job never clips to a selection, so it keeps no points.
    pub fn prepare_display_job(&mut self, doc: &str, layer: &str, level: u32) -> Result<String, JsError> {
        let (input, pixels, mask) = self.engine.display_job_input(parse_id(doc)?, parse_id(layer)?, level).map_err(js_err)?;
        self.job = (Some(pixels), mask, None);
        serde_json::to_string(&input).map_err(js_err)
    }
    /// The kept job buffer: the pixels, or the mask; null when there is none. A view on it is valid
    /// until the next engine call.
    pub fn job_buffer_ptr(&self, mask: bool) -> *const u8 {
        if mask { self.job.1.as_ref().map_or(std::ptr::null(), |m| m.bytes().as_ptr()) } else { self.job.0.as_ref().map_or(std::ptr::null(), |r| r.bytes().as_ptr()) }
    }
    pub fn job_buffer_len(&self, mask: bool) -> usize {
        if mask { self.job.1.as_ref().map_or(0, |m| m.bytes().len()) } else { self.job.0.as_ref().map_or(0, |r| r.bytes().len()) }
    }
    /// The kept job's selection points, as raw little-endian `i32` bytes (`JobSelection::flatten`'s
    /// shape); null when the job has no selection. A view on it is valid until the next engine call.
    pub fn job_points_ptr(&self) -> *const u8 { self.job.2.as_ref().map_or(std::ptr::null(), |p| p.as_ptr() as *const u8) }
    pub fn job_points_len(&self) -> usize { self.job.2.as_ref().map_or(0, |p| p.len() * std::mem::size_of::<i32>()) }
    pub fn release_job(&mut self) { self.job = (None, None, None); }
    /// `Engine::install_job`: an edit job's result put back, if the layer still matches `stamp`.
    pub fn install_job(&mut self, doc: &str, layer: &str, stamp_json: &str, output_json: &str, pixels: Option<Vec<u8>>, mask: Option<Vec<u8>>) -> Result<String, JsError> {
        let stamp: LayerStamp = serde_json::from_str(stamp_json).map_err(js_err)?;
        let output: JobOutput = serde_json::from_str(output_json).map_err(js_err)?;
        let (pixels, mask) = (raster_of(output.pixels, pixels)?, gray_of(output.mask, mask)?);
        let dirty = self.engine.install_job(parse_id(doc)?, parse_id(layer)?, stamp, output, pixels, mask).map_err(js_err)?;
        serde_json::to_string(&dirty).map_err(js_err)
    }
    /// `Engine::has_effects_image`: whether the canvas's effects image for the layer is made already.
    pub fn has_effects_image(&self, doc: &str, layer: &str, edit_json: Option<String>) -> Result<bool, JsError> {
        let edit = Self::parse_edit(edit_json)?;
        self.engine.has_effects_image(parse_id(doc)?, parse_id(layer)?, edit.as_ref()).map_err(js_err)
    }
    /// `Engine::keep_effects_image`: a full-size effects image from the worker, kept in the engine's
    /// cache when the layer's pixels and mask are still what the job took (`stamp_json`) and its
    /// current `EffectsDraw` still matches what the job was asked to draw (`key`, fix round 1 issue 2).
    pub fn keep_effects_image(&self, doc: &str, layer: &str, stamp_json: &str, key: &str, edit_json: Option<String>, width: u32, height: u32, bytes: Vec<u8>) -> Result<bool, JsError> {
        let stamp: LayerStamp = serde_json::from_str(stamp_json).map_err(js_err)?;
        let edit = Self::parse_edit(edit_json)?;
        let image = raster_of(Some((width, height)), Some(bytes))?.unwrap();
        self.engine.keep_effects_image(parse_id(doc)?, parse_id(layer)?, stamp, key, edit.as_ref(), image).map_err(js_err)
    }
    /// `run_edit_job` (in the worker): the output as JSON; the buffers it replaced are kept. `points`
    /// as `job_points_ptr` hands them out, when `input.selection` is not None.
    pub fn run_edit_job(&mut self, input_json: &str, pixels: Option<Vec<u8>>, mask: Option<Vec<u8>>, points: Option<Vec<u8>>, command_json: &str) -> Result<String, JsError> {
        let input: JobInput = serde_json::from_str(input_json).map_err(js_err)?;
        let command: Command = serde_json::from_str(command_json).map_err(js_err)?;
        let (pixels, mask) = (raster_of(input.pixels, pixels)?, gray_of(input.mask, mask)?);
        let points = points_of(input.selection.as_ref().map(JobSelection::point_count), points)?;
        let (output, new_pixels, new_mask) = run_edit_job(&input, pixels, mask, points.as_deref(), command).map_err(js_err)?;
        self.job = (new_pixels, new_mask, None);
        serde_json::to_string(&output).map_err(js_err)
    }
    /// `run_histogram_job` (in the worker): four arrays of 256 bins, as JSON. `points` as
    /// `run_edit_job` takes it.
    pub fn run_histogram_job(&self, input_json: &str, pixels: Option<Vec<u8>>, mask: Option<Vec<u8>>, points: Option<Vec<u8>>) -> Result<String, JsError> {
        let input: JobInput = serde_json::from_str(input_json).map_err(js_err)?;
        let (pixels, mask) = (raster_of(input.pixels, pixels)?, gray_of(input.mask, mask)?);
        let points = points_of(input.selection.as_ref().map(JobSelection::point_count), points)?;
        serde_json::to_string(&run_histogram_job(&input, pixels, mask, points.as_deref()).map_err(js_err)?).map_err(js_err)
    }
    /// `run_effects_job` (in the worker): the image's size and inset as JSON (the image is kept), or
    /// None when the layer draws no effects.
    pub fn run_effects_job(&mut self, input_json: &str, pixels: Vec<u8>, mask: Option<Vec<u8>>, factor: f64, edit_json: Option<String>) -> Result<Option<String>, JsError> {
        let input: JobInput = serde_json::from_str(input_json).map_err(js_err)?;
        let pixels = raster_of(input.pixels, Some(pixels))?.ok_or_else(|| JsError::new("an effects job needs pixels"))?;
        let mask = gray_of(input.mask, mask)?;
        let edit = Self::parse_edit(edit_json)?;
        match run_effects_job(&input, pixels, mask, factor, edit.as_ref()).map_err(js_err)? {
            Some((image, raster)) => { self.job = (Some(raster), None, None); Ok(Some(serde_json::to_string(&image).map_err(js_err)?)) }
            None => { self.job = (None, None, None); Ok(None) }
        }
    }
    pub fn version(&self) -> String { Engine::version().to_string() }

    pub fn new_document(&mut self, width: u32, height: u32, empty_layer: bool) -> Result<String, JsError> {
        self.engine.new_document(width, height, empty_layer).map(|id| ids::upper_string(&id)).map_err(js_err)
    }
    pub fn open_package(&mut self, manifest: String, names: Vec<String>, blobs: Array, path: Option<String>) -> Result<String, JsError> {
        let images = names.into_iter().enumerate().map(|(i, name)| {
            let arr = Uint8Array::new(&blobs.get(i as u32));
            (name, arr.to_vec())
        }).collect();
        let pkg = Package { manifest_json: manifest, images };
        self.engine.open_package(&pkg, path).map(|id| ids::upper_string(&id)).map_err(js_err)
    }
    pub fn prepare_save(&mut self, doc: &str) -> Result<(), JsError> {
        let id = parse_id(doc)?;
        let pkg = self.engine.save_package(id).map_err(js_err)?;
        self.pending_saves.insert(id, pkg);
        Ok(())
    }
    pub fn save_package_manifest(&self, doc: &str) -> Result<String, JsError> {
        let id = parse_id(doc)?;
        Ok(self.pending_saves.get(&id).ok_or_else(|| JsError::new("prepare_save first"))?.manifest_json.clone())
    }
    pub fn save_package_image_names(&self, doc: &str) -> Result<Vec<String>, JsError> {
        let id = parse_id(doc)?;
        Ok(self.pending_saves.get(&id).ok_or_else(|| JsError::new("prepare_save first"))?.images.iter().map(|(n, _)| n.clone()).collect())
    }
    pub fn save_package_image(&self, doc: &str, name: &str) -> Result<Uint8Array, JsError> {
        let id = parse_id(doc)?;
        let pkg = self.pending_saves.get(&id).ok_or_else(|| JsError::new("prepare_save first"))?;
        let (_, bytes) = pkg.images.iter().find(|(n, _)| n == name).ok_or_else(|| JsError::new("no such image"))?;
        Ok(Uint8Array::from(bytes.as_slice()))
    }
    pub fn finish_save(&mut self, doc: &str) -> Result<(), JsError> { self.pending_saves.remove(&parse_id(doc)?); Ok(()) }
    pub fn mark_saved(&mut self, doc: &str, path: Option<String>) -> Result<(), JsError> { self.engine.mark_saved(parse_id(doc)?, path); Ok(()) }
    pub fn close_document(&mut self, doc: &str) -> Result<(), JsError> { let id = parse_id(doc)?; self.engine.close_document(id); self.pending_saves.remove(&id); self.drawn = None; Ok(()) }
    pub fn document_ids(&self) -> Vec<String> { self.engine.document_ids().iter().map(ids::upper_string).collect() }

    pub fn state(&self, doc: &str) -> Result<String, JsError> {
        let s = self.engine.state(parse_id(doc)?).map_err(js_err)?;
        serde_json::to_string(&s).map_err(js_err)
    }
    pub fn execute(&mut self, doc: &str, command_json: &str) -> Result<String, JsError> {
        let command: Command = serde_json::from_str(command_json).map_err(js_err)?;
        let dirty = self.engine.execute(parse_id(doc)?, command).map_err(js_err)?;
        serde_json::to_string(&dirty).map_err(js_err)
    }
    pub fn undo(&mut self, doc: &str) -> Result<String, JsError> {
        serde_json::to_string(&self.engine.undo(parse_id(doc)?).map_err(js_err)?).map_err(js_err)
    }
    pub fn redo(&mut self, doc: &str) -> Result<String, JsError> {
        serde_json::to_string(&self.engine.redo(parse_id(doc)?).map_err(js_err)?).map_err(js_err)
    }
    pub fn revert(&mut self, doc: &str) -> Result<String, JsError> {
        serde_json::to_string(&self.engine.revert(parse_id(doc)?).map_err(js_err)?).map_err(js_err)
    }
    pub fn import_image(&mut self, doc: Option<String>, bytes: &[u8], name: &str, x: Option<f64>, y: Option<f64>) -> Result<String, JsError> {
        let id = match doc { Some(d) => Some(parse_id(&d)?), None => None };
        let at = match (x, y) { (Some(x), Some(y)) => Some(Point { x, y }), _ => None };
        self.engine.import_image(id, bytes, name, at).map(|id| ids::upper_string(&id)).map_err(js_err)
    }
    pub fn export_png(&self, doc: &str) -> Result<Uint8Array, JsError> {
        Ok(Uint8Array::from(self.engine.export_png(parse_id(doc)?).map_err(js_err)?.as_slice()))
    }
    pub fn export_jpeg(&self, doc: &str, quality: f64, r: f64, g: f64, b: f64) -> Result<Uint8Array, JsError> {
        Ok(Uint8Array::from(self.engine.export_jpeg(parse_id(doc)?, quality, [r, g, b]).map_err(js_err)?.as_slice()))
    }
    pub fn export_jpeg_preview(&self, doc: &str, quality: f64, r: f64, g: f64, b: f64, max_side: u32) -> Result<Uint8Array, JsError> {
        Ok(Uint8Array::from(self.engine.export_jpeg_preview(parse_id(doc)?, quality, [r, g, b], max_side).map_err(js_err)?.as_slice()))
    }
    pub fn composite(&self, doc: &str, x: f64, y: f64, w: f64, h: f64, out_w: u32, out_h: u32) -> Result<Uint8Array, JsError> {
        let raster = self.engine.composite(parse_id(doc)?, Rect { x, y, width: w, height: h }, out_w, out_h).map_err(js_err)?;
        Ok(Uint8Array::from(raster.bytes()))
    }
    /// The layer's raster after `level` sharp halvings, the reduction the CPU compositor applies
    /// for the same zoom. `Raster::halved` memoizes into the parent raster, so the returned
    /// buffer outlives this call (the document holds the whole chain) and repeat frames at the
    /// same level recompute nothing.
    fn level_raster(&self, doc: &str, layer: &str, level: u32) -> Result<Option<Raster>, JsError> {
        self.engine.layer_raster(parse_id(doc)?, parse_id(layer)?, level).map_err(js_err)
    }
    pub fn set_preview(&mut self, doc: &str, request_json: Option<String>) -> Result<String, JsError> {
        let request = match request_json { Some(j) => Some(serde_json::from_str::<PreviewRequest>(&j).map_err(js_err)?), None => None };
        serde_json::to_string(&self.engine.set_preview(parse_id(doc)?, request).map_err(js_err)?).map_err(js_err)
    }
    /// `Engine::selection_outline`: the marching ants' outline, flat (contour count, then each
    /// contour's point count and x, y pairs), traced coarser when `step` < 1 and it is very detailed.
    pub fn selection_outline(&self, doc: &str, step: f64) -> Result<js_sys::Float64Array, JsError> {
        Ok(js_sys::Float64Array::from(self.engine.selection_outline(parse_id(doc)?, step).map_err(js_err)?.as_slice()))
    }
    pub fn selection_contains(&self, doc: &str, x: f64, y: f64) -> Result<bool, JsError> {
        self.engine.selection_contains(parse_id(doc)?, Point { x, y }).map_err(js_err)
    }
    pub fn histogram(&self, doc: &str, layer: &str) -> Result<String, JsError> {
        serde_json::to_string(&self.engine.histogram(parse_id(doc)?, parse_id(layer)?).map_err(js_err)?).map_err(js_err)
    }
    /// Auto Levels from a histogram the panel already holds (`histogram` above, computed once when
    /// it opened), so choosing Auto never recomposites. Anything but 4 x 256 finite bins is refused.
    pub fn auto_levels(&self, histogram_json: &str, mode: &str) -> Result<String, JsError> {
        let mode: LevelsAuto = serde_json::from_str(&format!("\"{mode}\"")).map_err(js_err)?;
        let histogram: Vec<Vec<f64>> = serde_json::from_str(histogram_json).map_err(js_err)?;
        if histogram.len() != 4 || histogram.iter().any(|b| b.len() != 256 || b.iter().any(|v| !v.is_finite())) {
            return Err(JsError::new("a histogram is 4 x 256 finite bins"));
        }
        serde_json::to_string(&mode.settings(&histogram)).map_err(js_err)
    }
    pub fn levels_sampling(&self, doc: &str, layer: &str, settings_json: &str, x: f64, y: f64, mode: &str) -> Result<String, JsError> {
        let settings: LevelsSettings = serde_json::from_str(settings_json).map_err(js_err)?;
        let mode: LevelsSample = serde_json::from_str(&format!("\"{mode}\"")).map_err(js_err)?;
        let out = self.engine.levels_sampling(parse_id(doc)?, parse_id(layer)?, &settings, Point { x, y }, mode).map_err(js_err)?;
        serde_json::to_string(&out).map_err(js_err)
    }
    pub fn sample_layer_color(&self, doc: &str, layer: &str, x: f64, y: f64) -> Result<Option<String>, JsError> {
        match self.engine.sample_layer_color(parse_id(doc)?, parse_id(layer)?, Point { x, y }).map_err(js_err)? {
            Some(rgb) => Ok(Some(serde_json::to_string(&rgb).map_err(js_err)?)), None => Ok(None),
        }
    }
    pub fn sample_color(&self, doc: &str, x: f64, y: f64) -> Result<Option<String>, JsError> {
        match self.engine.sample_color(parse_id(doc)?, Point { x, y }).map_err(js_err)? {
            Some(rgb) => Ok(Some(serde_json::to_string(&rgb).map_err(js_err)?)), None => Ok(None),
        }
    }
    /// `LayerAdjustment::is_identity`: whether applying this would change nothing. The panels ask
    /// rather than keep a second copy of the per-kind rule.
    pub fn adjustment_is_identity(&self, adjustment_json: &str) -> Result<bool, JsError> {
        let a: LayerAdjustment = serde_json::from_str(adjustment_json).map_err(js_err)?;
        Ok(a.is_identity())
    }
    /// `gpu_lut`: 256 RGBA rows for the kinds that map colour through a table; empty for the
    /// others and for settings that fail validation.
    pub fn adjustment_lut(&self, adjustment_json: &str) -> Result<Uint8Array, JsError> {
        let a: LayerAdjustment = serde_json::from_str(adjustment_json).map_err(js_err)?;
        Ok(Uint8Array::from(gpu_lut(&a).as_slice()))
    }
    /// `gpu_hue_response`: 361 entries of (hue shift, saturation, lightness, 0); empty unless this
    /// is a valid Hue/Saturation adjustment.
    pub fn hue_response_table(&self, adjustment_json: &str) -> Result<js_sys::Float32Array, JsError> {
        let a: LayerAdjustment = serde_json::from_str(adjustment_json).map_err(js_err)?;
        Ok(js_sys::Float32Array::from(gpu_hue_response(&a).as_slice()))
    }
    pub fn layer_pixels_ptr(&self, doc: &str, layer: &str, level: u32) -> Result<*const u8, JsError> {
        Ok(self.level_raster(doc, layer, level)?.map_or(std::ptr::null(), |r| r.bytes().as_ptr()))
    }
    pub fn layer_pixels_len(&self, doc: &str, layer: &str, level: u32) -> Result<usize, JsError> {
        Ok(self.level_raster(doc, layer, level)?.map_or(0, |r| r.bytes().len()))
    }

    /// `Engine::draw_raster`: what the plan's draw of the layer samples at `level` (its padded
    /// effects image when the plan draws its effects), which the GPU uploads as its texture. Made
    /// once per upload and kept here until `release_draw_pixels` (or the next call, which drops
    /// the last one first), so the pointer `draw_pixels_ptr` gives stays valid whatever the
    /// engine's effects cache evicts, even for an image too large for the cache to keep. Returns
    /// its byte length, 0 for none.
    pub fn prepare_draw_pixels(&mut self, doc: &str, layer: &str, level: u32, edit_json: Option<String>) -> Result<usize, JsError> {
        self.drawn = None;
        let edit = Self::parse_edit(edit_json)?;
        self.drawn = self.engine.draw_raster(parse_id(doc)?, parse_id(layer)?, level, edit.as_ref()).map_err(js_err)?;
        Ok(self.drawn.as_ref().map_or(0, |r| r.bytes().len()))
    }
    /// The bytes `prepare_draw_pixels` kept; null when it kept none.
    pub fn draw_pixels_ptr(&self) -> *const u8 { self.drawn.as_ref().map_or(std::ptr::null(), |r| r.bytes().as_ptr()) }
    /// Drops the raster `prepare_draw_pixels` kept, once the upload has copied its bytes: a padded
    /// image near the limit is about 0.8 GB, of no use once the GPU has it.
    pub fn release_draw_pixels(&mut self) { self.drawn = None; }

    fn parse_edit(json: Option<String>) -> Result<Option<PreviewEdit>, JsError> {
        match json { Some(j) => Ok(Some(serde_json::from_str(&j).map_err(js_err)?)), None => Ok(None) }
    }
    fn parse_ids(json: &str) -> Result<Vec<Uuid>, JsError> {
        let v: Vec<String> = serde_json::from_str(json).map_err(js_err)?;
        v.iter().map(|s| Uuid::parse_str(s).map_err(js_err)).collect()
    }
    pub fn render_plan(&self, doc: &str, edit_json: Option<String>) -> Result<String, JsError> {
        let edit = Self::parse_edit(edit_json)?;
        serde_json::to_string(&self.engine.render_plan(parse_id(doc)?, edit.as_ref()).map_err(js_err)?).map_err(js_err)
    }
    pub fn composite_edit(&self, doc: &str, edit_json: Option<String>, x: f64, y: f64, w: f64, h: f64, out_w: u32, out_h: u32) -> Result<Uint8Array, JsError> {
        let edit = Self::parse_edit(edit_json)?;
        let raster = self.engine.composite_edit(parse_id(doc)?, edit.as_ref(), Rect { x, y, width: w, height: h }, out_w, out_h).map_err(js_err)?;
        Ok(Uint8Array::from(raster.bytes()))
    }
    /// `Engine::pixels_delta`: `[x, y, width, height]` of what changed in the layer's pixels since
    /// revision `from` (width 0 for nothing), or an empty array when the whole raster must be uploaded.
    /// Revisions stay below 2^53, so they travel as numbers.
    pub fn pixels_delta(&self, doc: &str, layer: &str, from: f64) -> Result<Vec<f64>, JsError> {
        let rect = self.engine.pixels_delta(parse_id(doc)?, parse_id(layer)?, from as u64).map_err(js_err)?;
        Ok(rect.map_or_else(Vec::new, |r| vec![r.x as f64, r.y as f64, r.width as f64, r.height as f64]))
    }
    /// `Engine::mask_delta`, in the mask's own grid, as `pixels_delta`.
    pub fn mask_delta(&self, doc: &str, layer: &str, from: f64) -> Result<Vec<f64>, JsError> {
        let rect = self.engine.mask_delta(parse_id(doc)?, parse_id(layer)?, from as u64).map_err(js_err)?;
        Ok(rect.map_or_else(Vec::new, |r| vec![r.x as f64, r.y as f64, r.width as f64, r.height as f64]))
    }
    /// The mask as the canvas shows it (`Engine::mask_pixels`): its buffer is the document's, or an
    /// open mask preview's, which lives until the next engine call either way.
    pub fn mask_pixels_ptr(&self, doc: &str, layer: &str) -> Result<*const u8, JsError> {
        Ok(self.engine.mask_pixels(parse_id(doc)?, parse_id(layer)?).map_err(js_err)?.map_or(std::ptr::null(), |m| m.bytes().as_ptr()))
    }
    pub fn mask_pixels_len(&self, doc: &str, layer: &str) -> Result<usize, JsError> {
        Ok(self.engine.mask_pixels(parse_id(doc)?, parse_id(layer)?).map_err(js_err)?.map_or(0, |m| m.bytes().len()))
    }
    /// `Engine::layer_region`: the bytes of a rectangle of the layer's pixels at `level`, as shown.
    pub fn layer_region(&self, doc: &str, layer: &str, level: u32, x: u32, y: u32, width: u32, height: u32) -> Result<Uint8Array, JsError> {
        let bytes = self.engine.layer_region(parse_id(doc)?, parse_id(layer)?, level, PixelRect { x, y, width, height }).map_err(js_err)?;
        Ok(Uint8Array::from(bytes.as_slice()))
    }
    pub fn clip_dependents(&self, doc: &str, ids_json: &str) -> Result<String, JsError> {
        let ids = self.engine.clip_dependents(parse_id(doc)?, &Self::parse_ids(ids_json)?).map_err(js_err)?;
        serde_json::to_string(&ids.iter().map(ids::upper_string).collect::<Vec<_>>()).map_err(js_err)
    }
    pub fn merge_action(&self, doc: &str, ids_json: &str) -> Result<Option<String>, JsError> {
        Ok(self.engine.merge_action(parse_id(doc)?, &Self::parse_ids(ids_json)?).map_err(js_err)?.map(|s| s.to_string()))
    }
    pub fn group_box(&self, doc: &str, ids_json: &str) -> Result<Option<String>, JsError> {
        match self.engine.group_box(parse_id(doc)?, &Self::parse_ids(ids_json)?).map_err(js_err)? { Some(t) => Ok(Some(serde_json::to_string(&t).map_err(js_err)?)), None => Ok(None) }
    }
    pub fn can_toggle_clipping(&self, doc: &str, id: &str) -> Result<bool, JsError> { self.engine.can_toggle_clipping(parse_id(doc)?, parse_id(id)?).map_err(js_err) }
    pub fn can_place(&self, doc: &str, id: &str, parent: Option<String>) -> Result<bool, JsError> {
        let p = match parent { Some(p) => Some(parse_id(&p)?), None => None };
        self.engine.can_place(parse_id(doc)?, parse_id(id)?, p).map_err(js_err)
    }

    /// `spatial_blur`: a blur adjustment's kernel in output pixels and its halving level, at
    /// `out_per_doc` output pixels per document pixel. The GPU asks rather than keeping a copy of
    /// the reach rule or the settings' defaults.
    pub fn spatial_blur(&self, adjustment_json: &str, out_per_doc: f64) -> Result<String, JsError> {
        let a: LayerAdjustment = serde_json::from_str(adjustment_json).map_err(js_err)?;
        serde_json::to_string(&compositor_engine::spatial_blur(&a, out_per_doc)).map_err(js_err)
    }
    /// `spatial_grid` for the document's plan (with the pending edit): the halving lattice and the
    /// pad a render at `out_per_doc` uses. The GPU's frame asks rather than re-deriving them.
    pub fn spatial_grid(&self, doc: &str, edit_json: Option<String>, out_per_doc: f64) -> Result<String, JsError> {
        let edit = Self::parse_edit(edit_json)?;
        let plan = self.engine.render_plan(parse_id(doc)?, edit.as_ref()).map_err(js_err)?;
        serde_json::to_string(&compositor_engine::spatial_grid(&plan, out_per_doc)).map_err(js_err)
    }
    /// `spatial_span`: one axis of a render's working span, in output pixels from the canvas's
    /// leading edge, as `[start, end]`. Whole numbers travel as f64 (no BigInt across the bridge).
    pub fn spatial_span(&self, near: f64, far: f64, canvas: f64, cell: u32, pad: u32) -> Vec<f64> {
        let (start, end) = compositor_engine::spatial_span(near as i64, far as i64, canvas as i64, cell, pad);
        vec![start as f64, end as f64]
    }
}
