use crate::{AdjustmentKind, BlendMode, GrayRaster, LayerAdjustment, LayerEffects, LayerRecord, LayerTransform, Manifest, Point, Raster, Size, DEFAULT_RESOLUTION};
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
    pub adjustment: Option<LayerAdjustment>,
    pub shape: Option<serde_json::Value>,
    pub effects: Option<LayerEffects>,
    pub text: Option<serde_json::Value>,
    pub unknown: serde_json::Map<String, serde_json::Value>,
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
    /// Bumped whenever `mask` is replaced or mutated, so a renderer knows to re-upload.
    pub mask_revision: u64,
}

impl Layer {
    fn base(name: &str, transform: LayerTransform, pixels: Option<Raster>) -> Layer {
        Layer { id: Uuid::new_v4(), name: name.to_string(), visible: true, transform, pixels, pixels_revision: 1,
            parent_id: None, is_group: false, opacity: 1.0, blend_mode: BlendMode::Normal, mask: None,
            mask_source_id: None, extra: LayerExtra::default(), mask_revision: 1 }
    }
    /// A blank layer covers the canvas and owns no pixels until painted.
    pub fn blank(name: &str, canvas: Size) -> Layer {
        Layer::base(name, LayerTransform::axis_aligned(Point { x: 0.0, y: 0.0 }, canvas), None)
    }
    pub fn with_pixels(name: &str, raster: Raster, origin: Point) -> Layer {
        let size = Size { width: raster.width as f64, height: raster.height as f64 };
        Layer::base(name, LayerTransform::axis_aligned(origin, size), Some(raster))
    }
    /// Replaces the layer's pixels. A shape record describes the pixels it drew, so rewriting
    /// them drops it: macOS treats `liveShape` as nil once the asset is no longer the image the
    /// shape produced (ShapeTool.swift), and would otherwise redraw the shape over the new
    /// pixels on the next handle drag. Live text is dropped the same way: the Mac stops writing
    /// `text` once the layer's image is replaced (TypeTool.swift:52-55), and would otherwise
    /// reopen the layer as live text and re-render over the new pixels on its next text edit.
    pub fn set_pixels(&mut self, pixels: Option<Raster>) {
        self.pixels = pixels;
        self.pixels_revision += 1;
        self.extra.shape = None;
        self.extra.text = None;
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
        r.effects = self.extra.effects.clone();
        r.text = self.extra.text.clone();
        r.unknown = self.extra.unknown.clone();
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
            extra: LayerExtra { adjustment: record.adjustment.clone(), shape: record.shape.clone(),
                effects: record.effects.clone(), text: record.text.clone(), unknown: record.unknown.clone() },
            mask_revision: 1,
        }
    }
    /// What this layer alone contains that this build does not draw as the Mac does (a Motion Blur
    /// adjustment, drawn approximately; unknown keys, here or inside its effects), as phrases for
    /// the notice (`Document::undrawn` collects these across every layer, plus its own
    /// document-level check). Not sorted or de-duplicated here -- callers that need that pool the
    /// phrases through a set, as `Document::undrawn` does and as `merge` does when refusing to
    /// bake one (I1).
    pub fn undrawn_features(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(a) = &self.extra.adjustment {
            // Drawn as an even streak where Core Image tapers it (Filters.swift:170-208): named,
            // and so refused by merge, until the motion probe measures the gap (ruling G-I2).
            if a.kind == AdjustmentKind::MotionBlur { out.push("Motion Blur adjustment layers (drawn approximately)".to_string()); }
        }
        // Layer effects are drawn (Phase 3.5c); only keys a later Mac wrote inside them are not.
        let unknown_effects = self.extra.effects.as_ref().is_some_and(LayerEffects::has_unknown);
        if !self.extra.unknown.is_empty() || unknown_effects { out.push("settings from a newer version of Compositor".to_string()); }
        out
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
    pub guides: Vec<crate::Guide>,
    pub unknown: serde_json::Map<String, serde_json::Value>,
}

impl Document {
    pub fn new(width: u32, height: u32) -> Document {
        Document { id: Uuid::new_v4(), width, height, resolution: DEFAULT_RESOLUTION, layers: vec![], active_layer_id: None,
            guides: vec![], unknown: Default::default() }
    }
    pub fn size(&self) -> Size { Size { width: self.width as f64, height: self.height as f64 } }
    pub fn manifest(&self) -> Manifest {
        let mut m = Manifest::new(self.id, self.width as i64, self.height as i64, self.active_layer_id,
            self.layers.iter().map(Layer::record).collect());
        m.resolution = Some(self.resolution);
        m.guides = if self.guides.is_empty() { None } else { Some(self.guides.clone()) };
        m.unknown = self.unknown.clone();
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
    /// What this project contains that this build does not draw yet: Motion Blur adjustment
    /// layers (drawn approximately) and keys from a newer version. Sorted and de-duplicated so
    /// the notice is stable. Everything listed is preserved on save.
    pub fn undrawn(&self) -> Vec<String> {
        let mut out = std::collections::BTreeSet::new();
        if !self.unknown.is_empty() { out.insert("settings from a newer version of Compositor".to_string()); }
        for layer in &self.layers { out.extend(layer.undrawn_features()); }
        out.into_iter().collect()
    }
}

impl Mask {
    pub fn is_linked(&self) -> bool { self.linked != Some(false) }
    pub fn is_uniform(&self) -> bool { self.pixels.width == 1 && self.pixels.height == 1 }
    /// What the mask shows beyond its pixels: white or black, whichever most of its edge is.
    pub fn background(&self) -> u8 {
        let (w, h) = (self.pixels.width as usize, self.pixels.height as usize);
        let d = self.pixels.bytes();
        let (mut total, mut count) = (0u64, 0u64);
        for y in 0..h { for x in 0..w {
            if y == 0 || y == h - 1 || x == 0 || x == w - 1 { total += d[y * w + x] as u64; count += 1; }
        }}
        if count == 0 || total * 2 >= count * 255 { 255 } else { 0 }
    }
    /// Where the mask sits once its layer moves from `old` to `new` (the macOS placement rule).
    pub fn follow(&self, old: &LayerTransform, new: &LayerTransform) -> Option<LayerTransform> {
        if self.is_uniform() { return None; }
        let moved = if self.is_linked() { self.placement.map(|p| p.following(old, new)) } else { Some(self.placement.unwrap_or(*old)) };
        moved.filter(|m| !m.same_placement(new))
    }
}

impl Layer {
    pub fn set_mask(&mut self, mask: Option<Mask>) { self.mask = mask; self.mask_revision += 1; }
    pub fn mask_mut(&mut self) -> Option<&mut Mask> { self.mask_revision += 1; self.mask.as_mut() }
    pub fn has_pixels(&self) -> bool { self.pixels.is_some() }
    pub fn is_adjustment(&self) -> bool { self.extra.adjustment.is_some() }
}

impl Document {
    pub fn descendants(&self, id: Uuid) -> Vec<Uuid> {
        let mut result = Vec::new();
        let mut pending = vec![id];
        while let Some(parent) = pending.pop() {
            for l in &self.layers {
                if l.parent_id == Some(parent) && !result.contains(&l.id) { result.push(l.id); pending.push(l.id); }
            }
        }
        // Array order, so callers can rely on bottom-to-top.
        self.layers.iter().filter(|l| result.contains(&l.id)).map(|l| l.id).collect()
    }
    pub fn siblings(&self, parent: Option<Uuid>) -> Vec<Uuid> {
        self.layers.iter().filter(|l| l.parent_id == parent).map(|l| l.id).collect()
    }
    /// Layers whose every ancestor is visible (groups included).
    pub fn visible_ids(&self) -> std::collections::HashSet<Uuid> {
        let by_id: std::collections::HashMap<Uuid, &Layer> = self.layers.iter().map(|l| (l.id, l)).collect();
        self.layers.iter().filter(|layer| {
            let mut node = Some(*layer); let mut steps = 0;
            while let Some(n) = node { if !n.visible || steps > crate::MAX_NESTING { return false; } steps += 1; node = n.parent_id.and_then(|p| by_id.get(&p).copied()); }
            true
        }).map(|l| l.id).collect()
    }
    /// Visible pixel layers bottom to top in hierarchy order, groups excluded: what the
    /// compositor draws. Hierarchy order, not array order, is the z-order on both platforms -
    /// macOS composites `renderLayers`, the same depth-first walk filtered the same way
    /// (LayerGroups.swift), and the layers panel shows that walk reversed. The array is free to
    /// hold a folder's children anywhere (grouping appends them, as macOS does); only the walk
    /// decides what draws over what.
    pub fn render_ids(&self) -> Vec<Uuid> {
        let visible = self.visible_ids();
        crate::ops::hierarchy::hierarchy_order(self).into_iter()
            .filter(|id| visible.contains(id) && self.layer(*id).map_or(false, |l| !l.is_group))
            .collect()
    }
    /// Document equality for undo purposes: everything but the active layer, which is selection
    /// state rather than content. macOS keeps `activeLayerID` beside the document in a history
    /// snapshot and returns early from `end` unless the document itself changed
    /// (DocumentHistory.swift), so selecting a layer records nothing and preserves redo.
    ///
    /// Destructured without `..` on purpose: a field added to `Document` later is then a compile
    /// error here rather than a field silently left out of the undo comparison, which would make
    /// edits to it quietly un-undoable.
    pub fn same_content(&self, other: &Document) -> bool {
        let Document { id, width, height, resolution, layers, active_layer_id: _, guides, unknown } = self;
        *id == other.id && *width == other.width && *height == other.height
            && *resolution == other.resolution && *layers == other.layers
            && *guides == other.guides && *unknown == other.unknown
    }
}
