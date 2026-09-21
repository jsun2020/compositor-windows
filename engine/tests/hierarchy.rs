use compositor_engine::ops::{appearance, hierarchy, layers};
use compositor_engine::*;

fn doc() -> Document { let mut d = Document::new(100, 100); d.active_layer_id = None; d }
fn names(d: &Document) -> Vec<String> { d.layers.iter().map(|l| l.name.clone()).collect() }
fn parent(d: &Document, id: uuid::Uuid) -> Option<uuid::Uuid> { d.layer(id).unwrap().parent_id }
fn red(w: u32, h: u32, a: &[u8]) -> Raster { let mut v = Vec::new(); for &x in a { v.extend_from_slice(&[x, 0, 0, x]); } Raster::from_premultiplied(w, h, v) }

#[test]
fn opacity_and_blend_mode() {
    let mut d = doc();
    let id = layers::add_blank_layer(&mut d).unwrap();
    appearance::set_opacity(&mut d, id, 0.25).unwrap();
    appearance::set_blend_mode(&mut d, id, BlendMode::Multiply).unwrap();
    assert_eq!((d.layer(id).unwrap().opacity, d.layer(id).unwrap().blend_mode), (0.25, BlendMode::Multiply));
    assert!(appearance::set_opacity(&mut d, id, f64::NAN).is_err());
    appearance::set_opacity(&mut d, id, 7.0).unwrap();
    assert_eq!(d.layer(id).unwrap().opacity, 1.0);
    let g = hierarchy::add_group(&mut d).unwrap();
    assert!(appearance::set_opacity(&mut d, g, 0.5).is_err());
    appearance::set_opacity_many(&mut d, &[id, g], 0.5).unwrap();
    assert_eq!((d.layer(id).unwrap().opacity, d.layer(g).unwrap().opacity), (0.5, 1.0));
}

#[test]
fn nested_groups_place_and_move_out() {
    let mut d = doc();
    let outer = hierarchy::add_group(&mut d).unwrap();
    let inner = hierarchy::add_group(&mut d).unwrap();
    let child = layers::add_blank_layer(&mut d).unwrap();
    assert_eq!(parent(&d, inner), Some(outer));
    assert_eq!(parent(&d, child), Some(inner));
    assert!(!hierarchy::can_place(&d, outer, Some(inner)));
    assert!(!hierarchy::can_place(&d, inner, Some(inner)));
    assert!(hierarchy::place_layer(&mut d, outer, Some(inner), None, false).is_err());
    // Move the child out of the inner folder, above it in the outer folder.
    hierarchy::place_layer(&mut d, child, Some(outer), Some(inner), false).unwrap();
    assert_eq!(parent(&d, child), Some(outer));
    assert_eq!(d.active_layer_id, Some(child));
    assert_eq!(names(&d), ["Folder 1", "Folder 2", "Layer 1"]);
    hierarchy::place_layer(&mut d, child, None, None, true).unwrap();
    assert_eq!((parent(&d, child), d.layers[0].id), (None, child));
}

#[test]
fn group_layers_wraps_selection_at_the_common_parent_in_order() {
    let mut d = doc();
    let folder = hierarchy::add_group(&mut d).unwrap();
    let child = layers::add_blank_layer(&mut d).unwrap();
    d.active_layer_id = None;
    let sibling = layers::add_blank_layer(&mut d).unwrap();
    let wrapper = hierarchy::group_layers(&mut d, &[folder, child, sibling]).unwrap();
    assert_eq!(parent(&d, folder), Some(wrapper));
    assert_eq!(parent(&d, sibling), Some(wrapper));
    assert_eq!(parent(&d, child), Some(folder), "a selected folder keeps its contents");
    assert_eq!(parent(&d, wrapper), None);
    assert_eq!(d.render_ids(), vec![child, sibling]);
    assert_eq!(d.active_layer_id, Some(wrapper));
    // Single layer then single folder: wrapped, not a child folder.
    let mut d = doc();
    let layer = layers::add_blank_layer(&mut d).unwrap();
    let inner = hierarchy::group_layers(&mut d, &[layer]).unwrap();
    let outer = hierarchy::group_layers(&mut d, &[inner]).unwrap();
    assert_eq!((parent(&d, layer), parent(&d, inner), parent(&d, outer)), (Some(inner), Some(outer), None));
    // Different folders: the common parent is the root; empty selection makes an empty folder.
    let mut d = doc();
    let first = hierarchy::group_layers(&mut d, &[]).unwrap();
    let a = layers::add_blank_layer(&mut d).unwrap();
    d.active_layer_id = None;
    let second = hierarchy::group_layers(&mut d, &[]).unwrap();
    let b = layers::add_blank_layer(&mut d).unwrap();
    let g = hierarchy::group_layers(&mut d, &[a, b]).unwrap();
    assert_eq!((parent(&d, g), parent(&d, a), parent(&d, b)), (None, Some(g), Some(g)));
    assert_eq!((parent(&d, first), parent(&d, second)), (None, None));
    assert_eq!(names(&d)[d.index_of(g).unwrap()], "Folder 3");
}

#[test]
fn move_by_swaps_siblings_and_duplicate_copies_above() {
    let mut d = doc();
    let ids: Vec<_> = (0..3).map(|_| layers::add_blank_layer(&mut d).unwrap()).collect();
    hierarchy::move_layer_by(&mut d, ids[0], 1).unwrap();
    assert_eq!(names(&d), ["Layer 2", "Layer 1", "Layer 3"]);
    assert!(hierarchy::move_layer_by(&mut d, ids[2], 1).is_err(), "top layer cannot move up");
    let copy = hierarchy::duplicate_layer(&mut d, ids[0]).unwrap();
    assert_eq!(names(&d), ["Layer 2", "Layer 1", "Layer 1 copy", "Layer 3"]);
    assert_eq!(d.active_layer_id, Some(copy));
    let g = hierarchy::add_group(&mut d).unwrap();
    assert!(hierarchy::duplicate_layer(&mut d, g).is_err(), "folders are not duplicated");
    let top = ids[2];
    let dropped = hierarchy::duplicate_layer_to(&mut d, ids[1], None, Some(top), false).unwrap();
    assert_eq!(names(&d)[3], "Folder 1", "the folder was inserted above the active copy");
    assert_eq!(d.layers.last().unwrap().name, "Layer 2 copy");
    assert_eq!(d.index_of(dropped).unwrap(), d.index_of(top).unwrap() + 1);
}

#[test]
fn clipping_links_stacks_and_detachment() {
    let mut d = doc();
    let ids: Vec<_> = (0..3).map(|i| { let l = Layer::with_pixels(&format!("L{i}"), red(2, 2, &[255; 4]), Point { x: 0.0, y: 0.0 }); let id = l.id; d.layers.push(l); id }).collect();
    assert!(!hierarchy::can_toggle_clipping(&d, ids[0]) && hierarchy::can_toggle_clipping(&d, ids[1]));
    hierarchy::toggle_clipping(&mut d, ids[0]).unwrap_err();
    hierarchy::toggle_clipping(&mut d, ids[1]).unwrap();
    hierarchy::toggle_clipping(&mut d, ids[2]).unwrap();
    assert_eq!(d.layer(ids[1]).unwrap().mask_source_id, Some(ids[0]));
    assert_eq!(d.layer(ids[2]).unwrap().mask_source_id, Some(ids[0]), "shares the base");
    hierarchy::toggle_clipping(&mut d, ids[2]).unwrap();
    assert_eq!(d.layer(ids[2]).unwrap().mask_source_id, None);
    assert_eq!(d.layer(ids[1]).unwrap().mask_source_id, Some(ids[0]));
    hierarchy::toggle_clipping(&mut d, ids[2]).unwrap();
    hierarchy::release_clipping(&mut d, ids[1]).unwrap();
    assert!(d.layer(ids[1]).unwrap().mask_source_id.is_none() && d.layer(ids[2]).unwrap().mask_source_id.is_none(), "releasing the lowest child releases the ones above");
    hierarchy::toggle_clipping(&mut d, ids[1]).unwrap();
    hierarchy::toggle_clipping(&mut d, ids[2]).unwrap();
    hierarchy::place_layer(&mut d, ids[2], None, None, true).unwrap();
    assert_eq!(d.layers[0].id, ids[2]);
    assert_eq!(d.layers[0].mask_source_id, None, "dragged out of the stack, it stops clipping");
    assert_eq!(d.layers.last().unwrap().mask_source_id, Some(ids[0]));
    assert!(!hierarchy::can_link_mask(&d, ids[1], ids[0]), "cycle");
    assert!(!hierarchy::can_link_mask(&d, ids[0], ids[0]));
}

#[test]
fn deleting_sources_bakes_or_unlinks() {
    let mut d = Document::new(2, 2);
    let mut target = Layer::with_pixels("T", red(2, 2, &[255; 4]), Point { x: 0.0, y: 0.0 }); target.transform.sampling = Sampling::Nearest;
    let mut source = Layer::with_pixels("S", red(2, 2, &[255, 0, 128, 255]), Point { x: 0.0, y: 0.0 }); source.transform.sampling = Sampling::Nearest;
    source.visible = false;
    target.mask_source_id = Some(source.id);
    let (tid, sid) = (target.id, source.id);
    d.layers = vec![target, source];
    assert_eq!(hierarchy::clip_dependents(&d, &[sid]), vec![tid]);
    assert!(hierarchy::clip_dependents(&d, &[tid]).is_empty());
    let before: Vec<u8> = composite(&d, Rect { x: 0.0, y: 0.0, width: 2.0, height: 2.0 }, 2, 2).bytes().chunks_exact(4).map(|p| p[3]).collect();
    let mut baked = d.clone();
    hierarchy::delete_layers(&mut baked, &[sid], true).unwrap();
    let after: Vec<u8> = composite(&baked, Rect { x: 0.0, y: 0.0, width: 2.0, height: 2.0 }, 2, 2).bytes().chunks_exact(4).map(|p| p[3]).collect();
    assert_eq!(before, after);
    assert_eq!(baked.layers.len(), 1);
    assert_eq!(baked.layers[0].mask_source_id, None);
    assert_eq!(baked.layers[0].pixels_revision, 2);
    let mut unlinked = d.clone();
    hierarchy::delete_layers(&mut unlinked, &[sid], false).unwrap();
    assert_eq!(unlinked.layers[0].pixels_revision, 1);
    assert_eq!(unlinked.layers[0].mask_source_id, None);
}

#[test]
fn deleting_a_folder_removes_its_contents_and_picks_a_neighbour() {
    let mut d = doc();
    let keep = layers::add_blank_layer(&mut d).unwrap();
    d.active_layer_id = None;
    let folder = hierarchy::add_group(&mut d).unwrap();
    let child = layers::add_blank_layer(&mut d).unwrap();
    d.active_layer_id = None;
    let top = layers::add_blank_layer(&mut d).unwrap();
    hierarchy::delete_layers(&mut d, &[folder, top], false).unwrap();
    assert_eq!(d.layers.iter().map(|l| l.id).collect::<Vec<_>>(), vec![keep]);
    assert_eq!(d.active_layer_id, Some(keep));
    let _ = child;
}
