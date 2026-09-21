use compositor_engine::*;
use js_sys::{Array, Uint8Array};
use std::collections::HashMap;
use uuid::Uuid;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmEngine { engine: Engine, pending_saves: HashMap<Uuid, Package> }

fn js_err<E: std::fmt::Display>(e: E) -> JsError { JsError::new(&e.to_string()) }
fn parse_id(text: &str) -> Result<Uuid, JsError> { Uuid::parse_str(text).map_err(js_err) }

#[wasm_bindgen]
impl WasmEngine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmEngine {
        console_error_panic_hook::set_once();
        WasmEngine { engine: Engine::new(), pending_saves: HashMap::new() }
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
    pub fn close_document(&mut self, doc: &str) -> Result<(), JsError> { let id = parse_id(doc)?; self.engine.close_document(id); self.pending_saves.remove(&id); Ok(()) }
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
    pub fn layer_pixels_ptr(&self, doc: &str, layer: &str) -> Result<*const u8, JsError> {
        let d = self.engine.document(parse_id(doc)?).ok_or_else(|| JsError::new("no document"))?;
        let l = d.layer(parse_id(layer)?).ok_or_else(|| JsError::new("no layer"))?;
        Ok(l.pixels.as_ref().map_or(std::ptr::null(), |p| p.bytes().as_ptr()))
    }
    pub fn layer_pixels_len(&self, doc: &str, layer: &str) -> Result<usize, JsError> {
        let d = self.engine.document(parse_id(doc)?).ok_or_else(|| JsError::new("no document"))?;
        let l = d.layer(parse_id(layer)?).ok_or_else(|| JsError::new("no layer"))?;
        Ok(l.pixels.as_ref().map_or(0, |p| p.bytes().len()))
    }

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
    pub fn mask_pixels_ptr(&self, doc: &str, layer: &str) -> Result<*const u8, JsError> {
        let d = self.engine.document(parse_id(doc)?).ok_or_else(|| JsError::new("no document"))?;
        let l = d.layer(parse_id(layer)?).ok_or_else(|| JsError::new("no layer"))?;
        Ok(l.mask.as_ref().map_or(std::ptr::null(), |m| m.pixels.bytes().as_ptr()))
    }
    pub fn mask_pixels_len(&self, doc: &str, layer: &str) -> Result<usize, JsError> {
        let d = self.engine.document(parse_id(doc)?).ok_or_else(|| JsError::new("no document"))?;
        let l = d.layer(parse_id(layer)?).ok_or_else(|| JsError::new("no layer"))?;
        Ok(l.mask.as_ref().map_or(0, |m| m.pixels.bytes().len()))
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
}
