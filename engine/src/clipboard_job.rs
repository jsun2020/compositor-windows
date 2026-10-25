//! Clipboard snapshots carry metadata and separate raw buffers to the worker.
//! Encoding, compositing and selection coverage never run on the UI thread.
use crate::*;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct ClipboardLayer { #[serde(default, rename="pixelDepth")] pub pixel_depth: PixelDepth, pub record: LayerRecord, pub pixels: Option<(u32, u32)>, pub mask: Option<(u32, u32)> }
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardInput {
    pub width: u32, pub height: u32, pub resolution: f64,
    pub layers: Vec<ClipboardLayer>, pub selection: Option<JobSelection>,
    pub target: Option<Uuid>, pub mask: bool, pub merged: bool, pub region: Rect,
}
impl Engine {
    pub fn clipboard_input(&self, id: Uuid, layer: Option<Uuid>, mask: bool, merged: bool) -> Result<(ClipboardInput, Option<Vec<i32>>), CommandError> {
        let doc = self.document(id).ok_or(CommandError::NoDocument)?;
        let region = ops::clipboard::region(doc)?;
        if !merged {
            let target = doc.layer(layer.ok_or(CommandError::NoLayer)?).ok_or(CommandError::NoLayer)?;
            if mask && target.mask.is_none() || !mask && doc.selection.is_some() && (target.is_group || target.extra.adjustment.is_some()) {
                return Err(CommandError::Refused("Select a pixel layer or a mask to copy".into()));
            }
        }
        let descendants=layer.map(|id|doc.descendants(id)).unwrap_or_default();
        let layers:Vec<_> = doc.layers.iter().filter(|l| merged || Some(l.id) == layer || (!mask&&doc.selection.is_none()&&descendants.contains(&l.id))).map(|l| ClipboardLayer {
            pixel_depth: l.pixels.as_ref().map_or(PixelDepth::Eight,Raster::depth),
            record: l.record(), pixels: l.pixels.as_ref().map(|r| (r.width, r.height)), mask: l.mask.as_ref().map(|m| (m.pixels.width, m.pixels.height)),
        }).collect();
        let ids:Vec<_>=layers.iter().map(|l|l.record.id).collect();
        let layers=layers.into_iter().map(|mut l|{if l.record.parent_id.is_some_and(|p|!ids.contains(&p)){l.record.parent_id=None;}if l.record.mask_source_id.is_some_and(|p|!ids.contains(&p)){l.record.mask_source_id=None;}l}).collect();
        let selection = doc.selection.as_ref().map(|s| JobSelection { contour_lengths: s.contours.iter().map(|c| c.len() as u32).collect(), antialiased: s.antialiased, feather: s.feather });
        let points = doc.selection.as_ref().map(|s| JobSelection::flatten(&s.contours));
        Ok((ClipboardInput { width: doc.width, height: doc.height, resolution: doc.resolution, layers, selection, target: layer, mask, merged, region }, points))
    }
}
impl ClipboardInput {
    pub fn document(&self, buffers: Vec<(Option<Raster>, Option<GrayRaster>)>, points: Option<&[i32]>) -> Result<Document, CommandError> {
        if buffers.len() != self.layers.len() { return Err(CommandError::Argument("clipboard layer count mismatch".into())); }
        let mut doc = Document::new(self.width, self.height); doc.resolution = self.resolution;
        for (l, (pixels, mask)) in self.layers.iter().zip(buffers) {
            if pixels.as_ref().is_some_and(|p|p.depth()!=l.pixel_depth) || pixels.as_ref().map(|p| (p.width, p.height)) != l.pixels || mask.as_ref().map(|p| (p.width, p.height)) != l.mask {
                return Err(CommandError::Argument("clipboard buffer size mismatch".into()));
            }
            doc.layers.push(Layer::from_record(&l.record, pixels, mask));
        }
        doc.selection = match (&self.selection, points) {
            (Some(s), Some(p)) => Some(Selection::new(s.unflatten(p)?, s.antialiased, s.feather)),
            (None, None) => None,
            _ => return Err(CommandError::Argument("clipboard selection mismatch".into())),
        };
        doc.active_layer_id=self.target;
        Ok(doc)
    }
    pub fn copy(&self, buffers: Vec<(Option<Raster>, Option<GrayRaster>)>, points: Option<&[i32]>) -> Result<ops::clipboard::CopiedPixels, CommandError> {
        let doc=self.document(buffers,points)?;
        let whole_group=self.selection.is_none()&&!self.mask&&doc.layer(self.target.unwrap_or_default()).is_some_and(|l|l.is_group||l.is_adjustment());
        ops::clipboard::copy(&doc,self.target,self.mask,self.merged||whole_group)
    }
}

impl Engine {
    /// Keep immutable GIF sources in the main engine for a whole-layer copy. Worker inputs still
    /// contain only the displayed pixels, because the system clipboard is a static image.
    pub fn capture_gif_clipboard(&mut self, id: Uuid, layer: Uuid, token: String) -> Result<bool, CommandError> {
        let original = self.document(id).ok_or(CommandError::NoDocument)?;
        original.layer(layer).ok_or(CommandError::NoLayer)?;
        let descendants = original.descendants(layer);
        let mut snapshot = original.clone();
        snapshot.layers.retain(|l| l.id == layer || descendants.contains(&l.id));
        let ids: std::collections::HashSet<_> = snapshot.layers.iter().map(|l| l.id).collect();
        for l in &mut snapshot.layers {
            if l.parent_id.is_some_and(|id| !ids.contains(&id)) { l.parent_id = None; }
            if l.mask_source_id.is_some_and(|id| !ids.contains(&id)) { l.mask_source_id = None; }
        }
        snapshot.selection = None; snapshot.active_layer_id = Some(layer);
        let has_gif = snapshot.layers.iter().any(|l| l.extra.gif.is_some());
        self.layer_clipboard = has_gif.then_some((token, snapshot));
        Ok(has_gif)
    }
    pub fn paste_gif_clipboard(&mut self, id: Uuid, token: &str) -> Result<Dirty, CommandError> {
        let source = self.layer_clipboard.as_ref().filter(|(saved, _)| saved == token)
            .map(|(_, doc)| doc.clone()).ok_or_else(|| CommandError::Refused("The animation clipboard is no longer available".into()))?;
        self.paste_copied_layers(id, source)
    }
    /// The internal whole-layer clipboard retains all editable metadata and folder contents.
    /// The system image remains PNG for other applications; its token expires on any new copy.
    pub fn paste_copied_layers(&mut self,id:Uuid,source:Document)->Result<Dirty,CommandError>{
        let target=source.active_layer_id.ok_or(CommandError::NoLayer)?;
        self.edit(id,move |doc,_|{
            if doc.layers.len()+source.layers.len()>MAX_LAYERS||doc.used_pixels()+source.used_pixels()>MAX_PIXELS{return Err(ProjectError::TooLarge.into());}
            let map:std::collections::HashMap<_,_>=source.layers.iter().map(|l|(l.id,Uuid::new_v4())).collect();
            let mut copies:Vec<_>=source.layers.iter().cloned().map(|mut l|{let old=l.id;l.id=map[&old];l.parent_id=l.parent_id.and_then(|p|map.get(&p).copied());l.mask_source_id=l.mask_source_id.and_then(|p|map.get(&p).copied());l}).collect();
            let index=copies.iter().position(|l|l.id==map[&target]).ok_or(CommandError::NoLayer)?;
            let root=copies.remove(index);let root=ops::layers::insert_above_active(doc,root)?;
            let at=doc.index_of(root).unwrap()+1;
            doc.layers.splice(at..at,copies);doc.selection=None;
            Ok(Dirty::everything())
        })
    }
}
