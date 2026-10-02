//! Ungroup Layers (Phase 4.5), ported from Compositor 1.4.5's GroupTests
//! (CompositorTests/GroupTests.swift:110-189 at v1.4.5): ungroupLayersRestoresChildrenAtTheFoldersSpotAndUndoes,
//! ungroupPreservesClippingBetweenTwoOfAFoldersOwnChildren and ungroupingReleasesClippingThatNoLongerMakesSense,
//! with their steps and expectations.
use compositor_engine::*;
use uuid::Uuid;

fn run(e: &mut Engine, id: Uuid, c: Command) { let what = format!("{c:?}"); e.execute(id, c).unwrap_or_else(|err| panic!("{what}: {err}")); }
fn active(e: &Engine, id: Uuid) -> Uuid { e.state(id).unwrap().active_layer_id.unwrap() }
fn blank(e: &mut Engine, id: Uuid) -> Uuid { run(e, id, Command::AddBlankLayer); active(e, id) }
/// A layer with pixels (a clip needs a base with pixels; the Mac's blank layers have them, this port's
/// do not), placed above the active layer as `AddBlankLayer` places one.
fn painted(e: &mut Engine, id: Uuid) -> Uuid {
    let png = encode_png(&Raster::from_premultiplied(4, 4, [255, 0, 0, 255].repeat(16)), 72.0).unwrap();
    e.import_image(Some(id), &png, "Painted", Some(Point { x: 50.0, y: 50.0 })).unwrap();
    active(e, id)
}
fn ids(e: &Engine, id: Uuid) -> Vec<Uuid> { e.state(id).unwrap().layers.iter().map(|l| l.id).collect() }
fn parent(e: &Engine, id: Uuid, layer: Uuid) -> Option<Uuid> { e.state(id).unwrap().layers.iter().find(|l| l.id == layer).unwrap().parent_id }
fn clip_source(e: &Engine, id: Uuid, layer: Uuid) -> Option<Uuid> { e.document(id).unwrap().layer(layer).unwrap().mask_source_id }

#[test]
fn ungroup_restores_the_children_at_the_folders_spot_and_undoes() {
    let mut e = Engine::new();
    let id = e.new_document(100, 100, false).unwrap();
    let below = blank(&mut e, id);
    assert!(e.execute(id, Command::UngroupLayers { id: below }).is_err(), "a plain layer has nothing to unwrap");
    run(&mut e, id, Command::AddGroup);
    let group = active(&e, id);
    let child_a = blank(&mut e, id);
    let child_b = blank(&mut e, id);
    run(&mut e, id, Command::SetActiveLayer { id: None });
    let above = blank(&mut e, id);
    let parents: Vec<Option<Uuid>> = e.state(id).unwrap().layers.iter().map(|l| l.parent_id).collect();
    assert_eq!(parents, vec![None, None, Some(group), Some(group), None]);

    run(&mut e, id, Command::SetActiveLayer { id: Some(group) });
    let before = e.state(id).unwrap().undo_depth;
    run(&mut e, id, Command::UngroupLayers { id: group });
    assert_eq!(Command::UngroupLayers { id: group }.action_name(), "Ungroup Layers");
    let order = ids(&e, id);
    assert!(!order.contains(&group), "the folder itself goes");
    assert_eq!((parent(&e, id, child_a), parent(&e, id, child_b)), (None, None));
    // Spliced in where the folder sat: below stays below both children, above stays above both.
    assert_eq!(order, vec![below, child_a, child_b, above]);
    assert_eq!(active(&e, id), child_a, "the first child is the active layer (selectLayers(childIDs, primary: children.first))");
    assert_eq!(e.state(id).unwrap().undo_depth, before + 1);

    e.undo(id).unwrap();
    assert_eq!(parent(&e, id, child_a), Some(group));
    assert!(e.state(id).unwrap().layers.iter().find(|l| l.id == group).unwrap().is_group);
    e.redo(id).unwrap();
    assert_eq!(e.state(id).unwrap().layers.len(), 4);
}

#[test]
fn ungroup_keeps_a_clip_between_two_of_the_folders_own_children() {
    let mut e = Engine::new();
    let id = e.new_document(100, 100, false).unwrap();
    let base = painted(&mut e, id);
    let clipped = painted(&mut e, id);
    run(&mut e, id, Command::LinkMask { source: base, target: clipped });
    run(&mut e, id, Command::GroupLayers { ids: vec![base, clipped] });
    let group = active(&e, id);
    run(&mut e, id, Command::UngroupLayers { id: group });
    // Spliced in together at the folder's old spot, so the pair stays adjacent.
    assert_eq!(clip_source(&e, id, clipped), Some(base));
}

#[test]
fn ungroup_releases_a_clip_that_no_longer_makes_sense() {
    let mut e = Engine::new();
    let id = e.new_document(100, 100, false).unwrap();
    let outside_base = painted(&mut e, id);
    let between = painted(&mut e, id);
    let child_source = painted(&mut e, id);
    // A clip can be set up across a folder boundary -- `LinkMask` does not forbid it -- though the two
    // layers are not really adjacent once the folder is in between.
    run(&mut e, id, Command::LinkMask { source: outside_base, target: child_source });
    run(&mut e, id, Command::GroupLayers { ids: vec![child_source] });
    let group = active(&e, id);
    assert_eq!(ids(&e, id), vec![outside_base, between, group, child_source]);
    run(&mut e, id, Command::UngroupLayers { id: group });
    // Ungrouped, child_source lands right after `between`, no longer next to its base, so the clip goes.
    assert_eq!(ids(&e, id), vec![outside_base, between, child_source]);
    assert_eq!(clip_source(&e, id, child_source), None);
}

#[test]
fn ungroup_drops_the_folders_own_look_and_keeps_nested_folders_whole() {
    // LayerGroups.swift:217-219: the folder's opacity, mask and effects go with it (a folder's blend mode
    // is always Normal, pass-through); a folder inside it is one of its children and keeps its contents.
    let mut e = Engine::new();
    let id = e.new_document(100, 100, false).unwrap();
    run(&mut e, id, Command::AddGroup);
    let outer = active(&e, id);
    run(&mut e, id, Command::AddGroup);
    let inner = active(&e, id);
    let deep = blank(&mut e, id);
    run(&mut e, id, Command::SetLayerOpacity { id: outer, opacity: 0.25 });
    run(&mut e, id, Command::UngroupLayers { id: outer });
    assert!(e.document(id).unwrap().layer(outer).is_none());
    assert_eq!((parent(&e, id, inner), parent(&e, id, deep)), (None, Some(inner)));
    let inner_layer = e.document(id).unwrap().layer(inner).unwrap().clone();
    assert_eq!(inner_layer.opacity, 1.0, "the children keep their own look, not the folder's 0.25");
}
