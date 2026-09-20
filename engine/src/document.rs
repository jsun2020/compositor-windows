use crate::{BlendMode, GrayRaster, LayerRecord, LayerTransform, Manifest, Point, Raster, Size, DEFAULT_RESOLUTION};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq)]
pub struct Mask {
    pub pixels: GrayRaster,
    pub enabled: bool,
    pub placement: Option<LayerTransform>,
    pub linked: Option<bool>,
}

/// Fields owned by later phases, carried through untouched.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct LayerExtra {
    pub adjustment: Option<serde_json::Value>,
    pub shape: Option<serde_json::Value>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    pub id: Uuid,
    pub name: String,
    pub visible: bool,
    pub transform: LayerTransform,
    pub pixels: Option<Raster>,
    /// Bumped whenever `pixels` is replaced, so a renderer knows to re-upload.
    pub pixels_revision: u64,
    pub parent_id: Option<Uuid>,
    pub is_group: bool,
    pub opacity: f64,
    pub blend_mode: BlendMode,
    pub mask: Option<Mask>,
    pub mask_source_id: Option<Uuid>,
    pub extra: LayerExtra,
}

impl Layer {
    fn base(name: &str, transform: LayerTransform, pixels: Option<Raster>) -> Layer {
        Layer { id: Uuid::new_v4(), name: name.to_string(), visible: true, transform, pixels, pixels_revision: 1,
            parent_id: None, is_group: false, opacity: 1.0, blend_mode: BlendMode::Normal, mask: None,
            mask_source_id: None, extra: LayerExtra::default() }
    }
    /// A blank layer covers the canvas and owns no pixels until painted.
    pub fn blank(name: &str, canvas: Size) -> Layer {
        Layer::base(name, LayerTransform::axis_aligned(Point { x: 0.0, y: 0.0 }, canvas), None)
    }
    pub fn with_pixels(name: &str, raster: Raster, origin: Point) -> Layer {
        let size = Size { width: raster.width as f64, height: raster.height as f64 };
        Layer::base(name, LayerTransform::axis_aligned(origin, size), Some(raster))
    }
    pub fn set_pixels(&mut self, pixels: Option<Raster>) {
        self.pixels = pixels;
        self.pixels_revision += 1;
    }
    pub fn record(&self) -> LayerRecord {
        let mut r = LayerRecord::new(self.id, &self.name, self.transform,
            self.pixels.as_ref().map(|_| LayerRecord::image_filename(&self.id)));
        r.is_visible = self.visible;
        r.parent_id = self.parent_id;
        r.is_group = if self.is_group { Some(true) } else { None };
        r.opacity = if self.opacity != 1.0 { Some(self.opacity) } else { None };
        r.blend_mode = if self.blend_mode != BlendMode::Normal { Some(self.blend_mode) } else { None };
        if let Some(mask) = &self.mask {
            r.mask_file = Some(LayerRecord::mask_filename(&self.id));
            r.mask_enabled = Some(mask.enabled);
            r.mask_placement = mask.placement;
            r.mask_linked = mask.linked;
        }
        r.mask_source_id = self.mask_source_id;
        r.adjustment = self.extra.adjustment.clone();
        r.shape = self.extra.shape.clone();
        r
    }
    pub fn from_record(record: &LayerRecord, pixels: Option<Raster>, mask: Option<GrayRaster>) -> Layer {
        Layer {
            id: record.id, name: record.name.clone(), visible: record.is_visible, transform: record.transform,
            pixels, pixels_revision: 1, parent_id: record.parent_id, is_group: record.is_group(),
            opacity: record.opacity.unwrap_or(1.0), blend_mode: record.blend_mode.unwrap_or_default(),
            mask: mask.map(|pixels| Mask { pixels, enabled: record.mask_enabled.unwrap_or(true),
                placement: record.mask_placement, linked: record.mask_linked }),
            mask_source_id: record.mask_source_id,
            extra: LayerExtra { adjustment: record.adjustment.clone(), shape: record.shape.clone() },
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Document {
    pub id: Uuid,
    pub width: u32,
    pub height: u32,
    pub resolution: f64,
    pub layers: Vec<Layer>, // bottom to top
    pub active_layer_id: Option<Uuid>,
}

impl Document {
    pub fn new(width: u32, height: u32) -> Document {
        Document { id: Uuid::new_v4(), width, height, resolution: DEFAULT_RESOLUTION, layers: vec![], active_layer_id: None }
    }
    pub fn size(&self) -> Size { Size { width: self.width as f64, height: self.height as f64 } }
    pub fn manifest(&self) -> Manifest {
        let mut m = Manifest::new(self.id, self.width as i64, self.height as i64, self.active_layer_id,
            self.layers.iter().map(Layer::record).collect());
        m.resolution = Some(self.resolution);
        m
    }
    pub fn used_pixels(&self) -> u64 {
        self.layers.iter().filter_map(|l| l.pixels.as_ref()).map(|r| r.width as u64 * r.height as u64).sum()
    }
    pub fn used_mask_pixels(&self) -> u64 {
        self.layers.iter().filter_map(|l| l.mask.as_ref()).map(|m| m.pixels.width as u64 * m.pixels.height as u64).sum()
    }
    pub fn index_of(&self, id: Uuid) -> Option<usize> { self.layers.iter().position(|l| l.id == id) }
    pub fn layer(&self, id: Uuid) -> Option<&Layer> { self.layers.iter().find(|l| l.id == id) }
    pub fn layer_mut(&mut self, id: Uuid) -> Option<&mut Layer> { self.layers.iter_mut().find(|l| l.id == id) }
}
