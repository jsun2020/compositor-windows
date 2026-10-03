use crate::*;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

pub fn validate(doc: &Document) -> Result<(), CommandError> { doc.manifest().validate().map_err(CommandError::Project) }

pub fn next_folder_name(doc: &Document) -> String {
    let mut n = 1;
    while doc.layers.iter().any(|l| l.name == format!("Folder {n}")) { n += 1; }
    format!("Folder {n}")
}

fn ancestors(doc: &Document, id: Uuid) -> Vec<Option<Uuid>> {
    let mut result = Vec::new();
    let mut parent = doc.layer(id).and_then(|l| l.parent_id);
    let mut steps = 0;
    while let Some(p) = parent { if steps > MAX_NESTING { break; } result.push(Some(p)); parent = doc.layer(p).and_then(|l| l.parent_id); steps += 1; }
    result.push(None);
    result
}

/// Depth-first, bottom-first: root siblings in array order, each folder's contents right after it.
pub fn hierarchy_order(doc: &Document) -> Vec<Uuid> {
    fn visit(doc: &Document, parent: Option<Uuid>, depth: usize, out: &mut Vec<Uuid>) {
        if depth > MAX_NESTING { return; }
        for id in doc.siblings(parent) {
            out.push(id);
            if doc.layer(id).map_or(false, |l| l.is_group) { visit(doc, Some(id), depth + 1, out); }
        }
    }
    let mut out = Vec::new();
    visit(doc, None, 0, &mut out);
    out
}

pub fn add_group(doc: &mut Document) -> Result<Uuid, CommandError> {
    if doc.layers.len() >= MAX_LAYERS { return Err(CommandError::Argument("too many layers".into())); }
    let mut group = Layer::blank(&next_folder_name(doc), doc.size());
    group.is_group = true;
    let active = doc.active_layer_id.and_then(|id| doc.layer(id).cloned());
    group.parent_id = match &active { Some(a) if a.is_group => Some(a.id), Some(a) => a.parent_id, None => None };
    let insertion = doc.active_layer_id.and_then(|id| doc.index_of(id)).map(|i| i + 1).unwrap_or(doc.layers.len());
    let id = group.id;
    let mut layers = doc.layers.clone();
    layers.insert(insertion, group);
    let previous = std::mem::replace(&mut doc.layers, layers);
    if let Err(e) = validate(doc) { doc.layers = previous; return Err(e); }
    doc.active_layer_id = Some(id);
    Ok(id)
}

/// Wraps `ids` (a selected folder carries its subtree) in a new folder at their common parent.
pub fn group_layers(doc: &mut Document, ids: &[Uuid]) -> Result<Uuid, CommandError> {
    if doc.layers.len() >= MAX_LAYERS { return Err(CommandError::Argument("too many layers".into())); }
    let selected: HashSet<Uuid> = ids.iter().copied().filter(|id| doc.layer(*id).is_some()).collect();
    let root_ids: HashSet<Uuid> = selected.iter().copied().filter(|id| !ancestors(doc, *id).iter().any(|a| a.map_or(false, |a| selected.contains(&a)))).collect();
    let ordered: Vec<Uuid> = hierarchy_order(doc).into_iter().filter(|id| root_ids.contains(id)).collect();
    let parent: Option<Uuid> = ordered.first().and_then(|first| {
        ancestors(doc, *first).into_iter().find(|candidate| ordered.iter().all(|id| ancestors(doc, *id).contains(candidate)))
    }).flatten();
    let mut group = Layer::blank(&next_folder_name(doc), doc.size());
    group.is_group = true;
    group.parent_id = parent;
    let branches: Vec<Uuid> = ordered.iter().map(|id| {
        let mut branch = *id; let mut steps = 0;
        while let Some(next) = doc.layer(branch).and_then(|l| l.parent_id) {
            if Some(next) == parent || steps > MAX_NESTING { break; }
            branch = next; steps += 1;
        }
        branch
    }).collect();
    let highest = doc.layers.iter().rposition(|l| branches.contains(&l.id));
    let insertion = highest.map(|h| doc.layers[..=h].iter().filter(|l| !root_ids.contains(&l.id)).count()).unwrap_or(doc.layers.len());
    let mut layers: Vec<Layer> = doc.layers.iter().filter(|l| !root_ids.contains(&l.id)).cloned().collect();
    let gid = group.id;
    layers.insert(insertion.min(layers.len()), group);
    for id in &ordered {
        if let Some(mut child) = doc.layer(*id).cloned() { child.parent_id = Some(gid); layers.push(child); }
    }
    let previous = std::mem::replace(&mut doc.layers, layers);
    if let Err(e) = validate(doc) { doc.layers = previous; return Err(e); }
    doc.active_layer_id = Some(gid);
    Ok(gid)
}

/// Ungroup Layers (`ungroupLayers`, LayerGroups.swift:214-239 at v1.4.5): the folder's direct children
/// take its place among its own siblings, in the order they had inside it, and the folder goes, its own
/// opacity, blend mode, mask and effects with it, as Photoshop's Ungroup does. A clipped layer no longer
/// next to its base stops clipping (`releaseDetachedClipping`). The first child becomes the active
/// layer (`selectLayers(childIDs, primary: children.first?.id)`); the children, bottom first, are
/// returned for the app to select.
pub fn ungroup_layers(doc: &mut Document, id: Uuid) -> Result<Vec<Uuid>, CommandError> {
    let group = doc.layer(id).ok_or(CommandError::NoLayer)?;
    if !group.is_group { return Err(CommandError::Argument("only a folder can be ungrouped".into())); }
    let parent = group.parent_id;
    let child_ids: HashSet<Uuid> = doc.layers.iter().filter(|l| l.parent_id == Some(id)).map(|l| l.id).collect();
    let children: Vec<Layer> = doc.layers.iter().filter(|l| child_ids.contains(&l.id)).cloned()
        .map(|mut l| { l.parent_id = parent; l }).collect();
    // Spliced in at the folder's own spot, so they land exactly where it sat among its siblings.
    let mut layers: Vec<Layer> = Vec::with_capacity(doc.layers.len());
    for layer in &doc.layers {
        if layer.id == id { layers.extend(children.iter().cloned()); }
        else if !child_ids.contains(&layer.id) { layers.push(layer.clone()); }
    }
    release_detached_clipping(&mut layers);
    let ordered: Vec<Uuid> = children.iter().map(|l| l.id).collect();
    // The folder may be the active layer; it is gone, so the active layer moves before the check.
    let previous = (std::mem::replace(&mut doc.layers, layers), doc.active_layer_id);
    doc.active_layer_id = ordered.first().copied();
    if let Err(e) = validate(doc) { (doc.layers, doc.active_layer_id) = previous; return Err(e); }
    Ok(ordered)
}

pub fn can_place(doc: &Document, id: Uuid, parent: Option<Uuid>) -> bool {
    if doc.layer(id).is_none() { return false; }
    let Some(parent) = parent else { return true; };
    parent != id && !doc.descendants(id).contains(&parent) && doc.layer(parent).map_or(false, |l| l.is_group)
}

/// A layer dropped between a base and a layer clipped to it joins the clipping group.
fn adopt_clipping(layers: &mut [Layer], id: Uuid) {
    let Some(layer) = layers.iter().find(|l| l.id == id).cloned() else { return; };
    if layer.is_group { return; }
    let siblings: Vec<Layer> = layers.iter().filter(|l| l.parent_id == layer.parent_id).cloned().collect();
    let Some(index) = siblings.iter().position(|l| l.id == id) else { return; };
    if index == 0 || index + 1 >= siblings.len() { return; }
    let Some(source) = siblings[index + 1].mask_source_id else { return; };
    if source == id { return; }
    let below = &siblings[index - 1];
    if below.id == source || below.mask_source_id == Some(source) {
        if let Some(l) = layers.iter_mut().find(|l| l.id == id) { l.mask_source_id = Some(source); }
    }
}

/// A clipped layer that no longer sits in the contiguous stack above its base stops clipping.
fn release_detached_clipping(layers: &mut [Layer]) {
    let mut release = HashSet::new();
    let parents: HashSet<Option<Uuid>> = layers.iter().map(|l| l.parent_id).collect();
    for parent in parents {
        let mut base: Option<Uuid> = None;
        for layer in layers.iter().filter(|l| l.parent_id == parent) {
            if let Some(source) = layer.mask_source_id {
                if Some(source) != base { release.insert(layer.id); base = Some(layer.id); }
            } else { base = if layer.is_group { None } else { Some(layer.id) }; }
        }
    }
    for l in layers.iter_mut() { if release.contains(&l.id) { l.mask_source_id = None; } }
}

pub fn place_layer(doc: &mut Document, id: Uuid, parent: Option<Uuid>, above: Option<Uuid>, at_bottom: bool) -> Result<(), CommandError> {
    if !can_place(doc, id, parent) || above == Some(id) { return Err(CommandError::Argument("cannot place the layer there".into())); }
    let mut layers = doc.layers.clone();
    let index = layers.iter().position(|l| l.id == id).ok_or(CommandError::NoLayer)?;
    let mut layer = layers.remove(index);
    layer.parent_id = parent;
    let mut insertion = if at_bottom {
        match parent {
            Some(p) => layers.iter().position(|l| l.id == p).map(|i| i + 1).unwrap_or(0),
            None => 0,
        }
    } else {
        layers.len()
    };
    if let Some(target) = above {
        let t = layers.iter().position(|l| l.id == target && l.parent_id == parent).ok_or(CommandError::Argument("drop target is not in that folder".into()))?;
        insertion = t + 1;
    }
    layers.insert(insertion, layer);
    adopt_clipping(&mut layers, id);
    release_detached_clipping(&mut layers);
    let previous = std::mem::replace(&mut doc.layers, layers);
    if let Err(e) = validate(doc) { doc.layers = previous; return Err(e); }
    doc.active_layer_id = Some(id);
    Ok(())
}

/// Swaps the layer with its sibling `offset` steps up (+) or down (-) the stack.
pub fn move_layer_by(doc: &mut Document, id: Uuid, offset: i32) -> Result<(), CommandError> {
    let layer = doc.layer(id).ok_or(CommandError::NoLayer)?.clone();
    let siblings = doc.siblings(layer.parent_id);
    let index = siblings.iter().position(|s| *s == id).ok_or(CommandError::NoLayer)? as i32;
    let other = index + offset;
    if other < 0 || other >= siblings.len() as i32 { return Err(CommandError::Argument("no sibling in that direction".into())); }
    let a = doc.index_of(id).unwrap(); let b = doc.index_of(siblings[other as usize]).unwrap();
    doc.layers.swap(a, b);
    adopt_clipping(&mut doc.layers, id);
    release_detached_clipping(&mut doc.layers);
    Ok(())
}

pub fn duplicate_layer(doc: &mut Document, id: Uuid) -> Result<Uuid, CommandError> {
    let index = doc.index_of(id).ok_or(CommandError::NoLayer)?;
    if doc.layers[index].is_group { return Err(CommandError::Argument("folders are not duplicated this way".into())); }
    if doc.layers.len() >= MAX_LAYERS { return Err(CommandError::Argument("too many layers".into())); }
    let mut copy = doc.layers[index].clone();
    copy.id = Uuid::new_v4();
    copy.name = format!("{} copy", copy.name);
    let cid = copy.id;
    doc.layers.insert(index + 1, copy);
    doc.active_layer_id = Some(cid);
    Ok(cid)
}

pub fn duplicate_layer_to(doc: &mut Document, id: Uuid, parent: Option<Uuid>, above: Option<Uuid>, at_bottom: bool) -> Result<Uuid, CommandError> {
    if !can_place(doc, id, parent) { return Err(CommandError::Argument("cannot place the copy there".into())); }
    let copy = duplicate_layer(doc, id)?;
    place_layer(doc, copy, parent, above, at_bottom)?;
    Ok(copy)
}

/// Layers outside `ids` (and their descendants) that clip to something being removed.
pub fn clip_dependents(doc: &Document, ids: &[Uuid]) -> Vec<Uuid> {
    let mut removed: HashSet<Uuid> = HashSet::new();
    for id in ids { removed.insert(*id); removed.extend(doc.descendants(*id)); }
    doc.layers.iter().filter(|l| !removed.contains(&l.id) && l.mask_source_id.map_or(false, |s| removed.contains(&s))).map(|l| l.id).collect()
}

/// The target's pixels multiplied by its clipping source's coverage, in the target's own grid.
pub fn bake_clip(doc: &Document, target: Uuid) -> Option<Raster> {
    let layer = doc.layer(target)?;
    let source = layer.mask_source_id?;
    let raster = layer.pixels.as_ref()?;
    let mut plan = render_plan(doc, None);
    ensure_source(doc, &mut plan, source);
    let to_doc = layer.transform.pixel_to_document(raster.width, raster.height);
    // Baking writes the target's own pixel grid, so the output scale is that grid's density.
    let out_per_doc = raster.width as f64 / layer.transform.size.width.max(1e-9);
    // A delete is a one-off: its own cache makes each source's effects image once for the bake.
    let cache = EffectsCache::default();
    let sources = crate::compositor::clip_source_rasters(doc, &plan, source, out_per_doc, &cache);
    let mut data = raster.bytes().to_vec();
    for y in 0..raster.height { for x in 0..raster.width {
        let p = to_doc.apply(Point { x: x as f64 + 0.5, y: y as f64 + 0.5 });
        let k = source_coverage_at(doc, &plan, source, p, &sources).clamp(0.0, 1.0);
        let i = ((y * raster.width + x) * 4) as usize;
        for c in 0..4 { data[i + c] = (data[i + c] as f32 * k).round() as u8; }
    }}
    Some(Raster::from_premultiplied(raster.width, raster.height, data))
}

/// Deletes `ids` with their contents as one change. `bake` keeps dependants' masked look in their pixels; otherwise the links are removed.
pub fn delete_layers(doc: &mut Document, ids: &[Uuid], bake: bool) -> Result<(), CommandError> {
    let mut baked: HashMap<Uuid, Raster> = HashMap::new();
    if bake { for t in clip_dependents(doc, ids) { if let Some(r) = bake_clip(doc, t) { baked.insert(t, r); } } }
    for id in ids {
        if doc.layer(*id).is_none() { continue; }
        let index = doc.index_of(*id).unwrap();
        let mut removed: HashSet<Uuid> = HashSet::from([*id]);
        removed.extend(doc.descendants(*id));
        doc.layers.retain(|l| !removed.contains(&l.id));
        for l in &mut doc.layers {
            if l.mask_source_id.map_or(false, |s| removed.contains(&s)) {
                l.mask_source_id = None;
                if let Some(r) = baked.remove(&l.id) { l.set_pixels(Some(r)); }
            }
        }
        if doc.active_layer_id.map_or(false, |a| removed.contains(&a)) {
            doc.active_layer_id = if doc.layers.is_empty() { None } else if index > 0 { Some(doc.layers[(index - 1).min(doc.layers.len() - 1)].id) } else { Some(doc.layers[0].id) };
        }
    }
    Ok(())
}

pub fn can_link_mask(doc: &Document, source: Uuid, target: Uuid) -> bool {
    if source == target { return false; }
    let (Some(s), Some(t)) = (doc.layer(source), doc.layer(target)) else { return false; };
    if s.is_group || t.is_group || s.extra.adjustment.is_some() { return false; }
    let mut probe = doc.clone();
    probe.layer_mut(target).unwrap().mask_source_id = Some(source);
    validate(&probe).is_ok()
}

pub fn link_mask(doc: &mut Document, source: Uuid, target: Uuid) -> Result<(), CommandError> {
    if !can_link_mask(doc, source, target) { return Err(CommandError::Argument("cannot clip to that layer".into())); }
    doc.layer_mut(target).unwrap().mask_source_id = Some(source);
    Ok(())
}

/// Releasing a layer releases the clipped siblings above it that share its base.
pub fn release_clipping(doc: &mut Document, target: Uuid) -> Result<(), CommandError> {
    let layer = doc.layer(target).ok_or(CommandError::NoLayer)?.clone();
    let source = layer.mask_source_id.ok_or(CommandError::Argument("the layer is not clipped".into()))?;
    let siblings = doc.siblings(layer.parent_id);
    let index = siblings.iter().position(|s| *s == target).unwrap();
    let mut releases = Vec::new();
    for id in &siblings[index..] {
        let l = doc.layer(*id).unwrap();
        if *id == target || l.mask_source_id == Some(source) { releases.push(*id); } else { break; }
    }
    for id in releases { doc.layer_mut(id).unwrap().mask_source_id = None; }
    Ok(())
}

pub fn can_toggle_clipping(doc: &Document, id: Uuid) -> bool {
    let Some(layer) = doc.layer(id) else { return false; };
    if layer.is_group { return false; }
    if layer.mask_source_id.is_some() { return true; }
    let siblings = doc.siblings(layer.parent_id);
    let Some(index) = siblings.iter().position(|s| *s == id) else { return false; };
    if index == 0 { return false; }
    let below = doc.layer(siblings[index - 1]).unwrap();
    if below.is_group || below.is_adjustment() { return false; }
    can_link_mask(doc, below.mask_source_id.unwrap_or(below.id), id)
}

/// Clips to the next lower sibling (sharing its base when it is clipped), or releases.
pub fn toggle_clipping(doc: &mut Document, id: Uuid) -> Result<(), CommandError> {
    let layer = doc.layer(id).ok_or(CommandError::NoLayer)?.clone();
    if layer.mask_source_id.is_some() { return release_clipping(doc, id); }
    if !can_toggle_clipping(doc, id) { return Err(CommandError::Argument("nothing below to clip to".into())); }
    let siblings = doc.siblings(layer.parent_id);
    let index = siblings.iter().position(|s| *s == id).unwrap();
    let below = doc.layer(siblings[index - 1]).unwrap().clone();
    link_mask(doc, below.mask_source_id.unwrap_or(below.id), id)
}
