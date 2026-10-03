use compositor_engine::*;
use compositor_engine::ops::clipboard;
use uuid::Uuid;
fn p(x: f64, y: f64) -> Point { Point { x, y } }
fn document() -> (Engine, Uuid, Uuid) {
    let mut doc = Document::new(12, 8);
    let raster = Raster::from_straight(4, 4, &[255, 0, 0, 255].repeat(16));
    let layer = Layer::with_pixels("Red", raster, p(2.0, 1.0));
    let id = layer.id; doc.layers.push(layer); doc.active_layer_id = Some(id);
    let mut e = Engine::new(); let handle = e.insert_document(doc); (e, handle, id)
}
fn same_user_content(a: &Document, b: &Document) -> bool {
    a.manifest() == b.manifest() && a.selection == b.selection && a.layers.iter().zip(&b.layers).all(|(a,b)| a.pixels == b.pixels && a.mask == b.mask)
}
fn select(e: &mut Engine, doc: Uuid, x: f64, y: f64, w: f64, h: f64) {
    e.execute(doc, Command::SelectShape { kind: SelectionShape::Rectangle, points: vec![p(x,y), p(x+w,y), p(x+w,y+h), p(x,y+h)], mode: SelectionMode::Replace, antialiased: false }).unwrap();
}
#[test]
fn copy_is_raw_and_read_only_while_merged_uses_appearance() {
    let (mut e, doc, id) = document();
    e.execute(doc, Command::SetLayerOpacity { id, opacity: 0.5 }).unwrap();
    select(&mut e, doc, 2.0, 1.0, 2.0, 3.0);
    let before = e.document(doc).unwrap().clone(); let depth = e.state(doc).unwrap().undo_depth;
    let raw = clipboard::copy(&before, Some(id), false, false).unwrap();
    let merged = clipboard::copy(&before, None, false, true).unwrap();
    assert_eq!((raw.raster.width, raw.raster.height, raw.origin), (2,3,p(2.0,1.0)));
    assert_eq!(raw.raster.pixel(0,0), [255,0,0,255]);
    assert_eq!(merged.raster.pixel(0,0), [128,0,0,128]);
    assert_eq!(e.document(doc).unwrap(), &before); assert_eq!(e.state(doc).unwrap().undo_depth, depth);
}
#[test]
fn no_selection_copies_canvas_including_transparent_margin() {
    let (e, doc, id) = document();
    let copy = clipboard::copy(e.document(doc).unwrap(), Some(id), false, false).unwrap();
    assert_eq!((copy.raster.width, copy.raster.height, copy.origin), (12,8,p(0.0,0.0)));
    assert_eq!(copy.raster.pixel(0,0), [0;4]); assert_eq!(copy.raster.pixel(2,1), [255,0,0,255]);
}
#[test]
fn selection_alpha_survives_png_round_trip() {
    let (mut e, doc, id) = document(); select(&mut e, doc, 3.0,2.0,1.0,1.0);
    e.execute(doc, Command::FeatherSelection { amount: 2 }).unwrap();
    let copy = clipboard::copy(e.document(doc).unwrap(), Some(id), false, false).unwrap();
    let decoded = decode_image(&encode_png(&copy.raster,72.0).unwrap()).unwrap().raster;
    assert_eq!(decoded.bytes(), copy.raster.bytes());
    assert!(decoded.bytes().chunks_exact(4).any(|p| p[3] > 0 && p[3] < 255));
}
#[test]
fn paste_inserts_above_active_and_deselects_in_one_undo() {
    let (mut e, doc, id) = document(); e.execute(doc,Command::AddBlankLayer).unwrap();
    e.execute(doc,Command::SetActiveLayer { id: Some(id) }).unwrap(); select(&mut e,doc,2.0,1.0,2.0,2.0);
    let before = e.document(doc).unwrap().clone(); let depth = e.state(doc).unwrap().undo_depth;
    let copied = clipboard::copy(&before,Some(id),false,false).unwrap();
    e.paste_raster(doc,copied.raster,Some(copied.origin),false).unwrap();
    let after = e.document(doc).unwrap();
    assert_eq!(after.layers[1].transform.origin, p(2.0,1.0)); assert_eq!(after.active_layer_id,Some(after.layers[1].id));
    assert!(after.selection.is_none()); assert_eq!(e.state(doc).unwrap().undo_depth,depth+1);
    e.undo(doc).unwrap(); assert!(same_user_content(e.document(doc).unwrap(), &before));
}
#[test]
fn external_paste_is_centered_and_layer_via_copy_keeps_selection() {
    let (mut e, doc, id) = document();
    e.paste_raster(doc,Raster::from_straight(3,3,&[0,255,0,255].repeat(9)),None,false).unwrap();
    assert_eq!(e.document(doc).unwrap().layers.last().unwrap().transform.origin,p(4.0,2.0));
    e.execute(doc,Command::SetActiveLayer { id: Some(id) }).unwrap(); select(&mut e,doc,2.0,1.0,2.0,2.0);
    let selection = e.document(doc).unwrap().selection.clone();
    e.execute(doc,Command::LayerViaCopy { id, mask:false }).unwrap();
    assert_eq!(e.document(doc).unwrap().selection,selection); assert_eq!(e.document(doc).unwrap().layers[1].pixels.as_ref().unwrap().width,2);
}
#[test]
fn cut_without_selection_clears_only_canvas_and_is_one_undo() {
    let (mut e, doc, id) = document(); let before = e.document(doc).unwrap().clone();
    e.execute(doc,Command::CutPixels { id, mask:false }).unwrap();
    assert!(e.document(doc).unwrap().layer(id).unwrap().pixels.as_ref().unwrap().bytes().iter().all(|&v| v == 0));
    assert!(e.document(doc).unwrap().selection.is_none()); assert_eq!(e.state(doc).unwrap().undo_depth,1);
    e.undo(doc).unwrap(); assert!(same_user_content(e.document(doc).unwrap(), &before));
}
#[test]
fn floating_cancel_and_identity_are_exact_even_with_feather_and_redo() {
    let (mut e, doc, id) = document(); select(&mut e,doc,3.0,2.0,2.0,2.0);
    e.execute(doc,Command::FeatherSelection { amount:2 }).unwrap();
    e.execute(doc,Command::SetLayerOpacity { id,opacity:0.3 }).unwrap(); e.undo(doc).unwrap();
    let before = e.document(doc).unwrap().clone(); let depth=e.state(doc).unwrap().undo_depth;
    let float = e.begin_floating(doc,id,false).unwrap(); let t=e.document(doc).unwrap().layer(float).unwrap().transform;
    e.cancel_floating(doc).unwrap(); assert!(e.document(doc).unwrap().same_content(&before)); assert!(e.state(doc).unwrap().can_redo);
    e.begin_floating(doc,id,false).unwrap(); e.commit_floating(doc,t,None).unwrap();
    assert!(e.document(doc).unwrap().same_content(&before)); assert_eq!(e.state(doc).unwrap().undo_depth,depth); assert!(e.state(doc).unwrap().can_redo);
}
#[test]
fn floating_move_merges_back_and_moves_outline_in_one_undo() {
    let (mut e,doc,id)=document(); select(&mut e,doc,2.0,1.0,2.0,2.0);
    let before=e.document(doc).unwrap().clone(); let depth=e.state(doc).unwrap().undo_depth;
    let float=e.begin_floating(doc,id,false).unwrap(); let mut t=e.document(doc).unwrap().layer(float).unwrap().transform;
    t.origin.x+=5.0; e.commit_floating(doc,t,None).unwrap();
    let after=e.document(doc).unwrap(); assert_eq!(after.layers.len(),1); assert_eq!(after.active_layer_id,Some(id));
    let canvas=e.composite(doc,Rect{x:0.0,y:0.0,width:12.0,height:8.0},12,8).unwrap();
    assert_eq!(canvas.pixel(2,1),[0;4]); assert_eq!(canvas.pixel(7,1),[255,0,0,255]); assert_eq!(canvas.pixel(5,4),[255,0,0,255]);
    assert_eq!(after.selection.as_ref().unwrap().bounds().unwrap().x,7.0); assert_eq!(e.state(doc).unwrap().undo_depth,depth+1);
    e.undo(doc).unwrap(); assert!(same_user_content(e.document(doc).unwrap(), &before));
}
#[test]
fn floating_duplicate_leaves_source_pixels_in_place() {
    let (mut e,doc,id)=document(); select(&mut e,doc,2.0,1.0,2.0,2.0);
    let float=e.begin_floating(doc,id,true).unwrap(); let mut t=e.document(doc).unwrap().layer(float).unwrap().transform;
    t.origin.x+=5.0; e.commit_floating(doc,t,None).unwrap();
    let canvas=e.composite(doc,Rect{x:0.0,y:0.0,width:12.0,height:8.0},12,8).unwrap();
    assert_eq!(canvas.pixel(2,1),[255,0,0,255]); assert_eq!(canvas.pixel(7,1),[255,0,0,255]);
}
#[test]
fn whole_layer_snapshot_preserves_editable_metadata_and_mask_after_source_changes(){
    let (mut e,doc,id)=document();e.execute(doc,Command::AddMask{id,revealing:false}).unwrap();
    let mut original=e.document(doc).unwrap().clone();original.layer_mut(id).unwrap().extra.text=Some(serde_json::json!({"content":"Editable", "fontName":"Helvetica"}));
    let new=e.insert_document(original);let (input,points)=e.clipboard_input(new,Some(id),false,false).unwrap();assert!(points.is_none());
    let source=e.document(new).unwrap().layer(id).unwrap().clone();let copied=input.document(vec![(source.pixels.clone(),source.mask.as_ref().map(|m|m.pixels.clone()))],None).unwrap();
    let target=e.new_document(50,40,false).unwrap();e.paste_copied_layers(target,copied).unwrap();let pasted=&e.document(target).unwrap().layers[0];
    assert_ne!(pasted.id,id);assert_eq!(pasted.extra.text,source.extra.text);assert!(pasted.mask.is_some());assert_eq!(pasted.pixels,source.pixels);assert_eq!(e.state(target).unwrap().undo_depth,1);e.undo(target).unwrap();assert!(e.document(target).unwrap().layers.is_empty());
}
