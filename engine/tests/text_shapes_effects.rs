use compositor_engine::*;
use compositor_engine::ops::text::TextStyle;
fn p(x:f64,y:f64)->Point{Point{x,y}}
fn style(content:&str)->serde_json::Value{serde_json::json!({"content":content,"fontName":"Arial","fontSize":24,"red":1,"green":0,"blue":0,"alignment":"Left","tracking":0,"leading":0})}
fn raster(w:u32,h:u32)->Raster{Raster::from_straight(w,h,&[255,0,0,255].repeat(w as usize*h as usize))}
#[test]
fn text_run_validation_uses_utf16_and_refuses_overlap_and_overflow(){
    let mut v=style("A😀中文B");v["colorRuns"]=serde_json::json!([{"location":1,"length":2,"red":0,"green":1,"blue":0}]);v["fontRuns"]=serde_json::json!([{"location":3,"length":2,"fontName":"Microsoft YaHei"}]);assert!(serde_json::from_value::<TextStyle>(v.clone()).unwrap().is_valid());
    v["colorRuns"][0]["length"]=serde_json::json!(6);assert!(!serde_json::from_value::<TextStyle>(v.clone()).unwrap().is_valid());v["colorRuns"]=serde_json::json!([]);assert!(!serde_json::from_value::<TextStyle>(v).unwrap().is_valid());
}
#[test]
fn text_creation_edit_and_noop_are_atomic_and_keep_transformed_upper_left_anchor(){
    let mut e=Engine::new();let doc=e.new_document(200,100,false).unwrap();let v=style("A😀中文B");e.install_text(doc,None,v.clone(),raster(40,30),p(10.0,10.0),None).unwrap();let id=e.document(doc).unwrap().active_layer_id.unwrap();assert_eq!(e.state(doc).unwrap().undo_depth,1);
    let mut t=e.document(doc).unwrap().layer(id).unwrap().transform;t.size=Size{width:80.0,height:45.0};t.rotation=30.0;t.flip_x=true;e.execute(doc,Command::SetLayerTransform{id,transform:t}).unwrap();e.execute(doc,Command::AddMask{id,revealing:true}).unwrap();
    let anchor=t.unit_to_document().apply(p(0.0,0.0));let before=e.document(doc).unwrap().clone();let depth=e.state(doc).unwrap().undo_depth;
    e.install_text(doc,Some(id),v.clone(),raster(40,30),p(0.0,0.0),None).unwrap();assert_eq!(e.state(doc).unwrap().undo_depth,depth);
    let mut changed=v;changed["content"]=serde_json::json!("More text");changed["futureTextOption"]=serde_json::json!(42);e.install_text(doc,Some(id),changed.clone(),raster(60,40),p(0.0,0.0),None).unwrap();let after=e.document(doc).unwrap().layer(id).unwrap().clone();
    assert_eq!(after.transform.size,Size{width:120.0,height:60.0});let a=after.transform.unit_to_document().apply(p(0.0,0.0));assert!((a.x-anchor.x).abs()<1e-8&&(a.y-anchor.y).abs()<1e-8);assert_eq!(after.extra.text,Some(changed));assert_eq!(after.mask.as_ref().unwrap().placement,Some(t));assert_eq!(e.state(doc).unwrap().undo_depth,depth+1);
    let pkg=e.save_package(doc).unwrap();let reopened=e.open_package(&pkg,None).unwrap();assert_eq!(e.document(reopened).unwrap().layer(id).unwrap().extra.text,after.extra.text);
    e.undo(doc).unwrap();assert_eq!(e.document(doc).unwrap().manifest(),before.manifest());assert_eq!(e.document(doc).unwrap().layer(id).unwrap().pixels,before.layer(id).unwrap().pixels);
}
#[test]
fn text_preview_is_not_saved_and_cancel_restores_exact_metadata_pixels_and_history(){
    let mut e=Engine::new();let doc=e.new_document(200,100,false).unwrap();e.install_text(doc,None,style("One"),raster(40,30),p(10.0,10.0),None).unwrap();let id=e.document(doc).unwrap().active_layer_id.unwrap();let before=e.document(doc).unwrap().clone();let stamp=LayerStamp::of(&before,before.layer(id).unwrap());let depth=e.state(doc).unwrap().undo_depth;
    e.keep_text_preview(doc,id,stamp,style("Two longer"),raster(80,30)).unwrap();assert_eq!(e.state(doc).unwrap().layers[0].pixels_width,80);assert_eq!(e.document(doc).unwrap(),&before);assert_eq!(e.state(doc).unwrap().undo_depth,depth);let pkg=e.save_package(doc).unwrap();let reopened=e.open_package(&pkg,None).unwrap();assert_eq!(e.document(reopened).unwrap().layer(id).unwrap().extra.text,before.layer(id).unwrap().extra.text);
    e.set_preview(doc,None).unwrap();assert!(e.document(doc).unwrap().same_content(&before));assert_eq!(e.state(doc).unwrap().layers[0].pixels_width,40);
}
#[test]
fn live_shape_redraws_fixed_radius_on_resize_and_painting_rasterizes_it(){
    let mut e=Engine::new();let doc=e.new_document(200,100,false).unwrap();e.execute(doc,Command::AddShape{shape:ShapeSpec::Rectangle{rect:Rect{x:10.0,y:10.0,width:20.0,height:20.0},corner_radius:6.0},color:[1.0,0.0,0.0]}).unwrap();let id=e.document(doc).unwrap().active_layer_id.unwrap();e.execute(doc,Command::AddMask{id,revealing:true}).unwrap();let before=e.document(doc).unwrap().clone();let mut t=before.layer(id).unwrap().transform;t.size=Size{width:60.0,height:40.0};e.execute(doc,Command::SetLayerTransform{id,transform:t}).unwrap();let l=e.document(doc).unwrap().layer(id).unwrap();assert_eq!(l.pixels.as_ref().unwrap().width,60);assert_eq!(l.extra.shape,before.layer(id).unwrap().extra.shape);assert_eq!(l.mask.as_ref().unwrap().placement,Some(t));assert_eq!(l.pixels.as_ref().unwrap().pixel(6,0)[3],255);
    e.undo(doc).unwrap();assert_eq!(e.document(doc).unwrap().manifest(),before.manifest());
    e.execute(doc,Command::InvertPixels{id,mask:false}).unwrap();assert!(e.document(doc).unwrap().layer(id).unwrap().extra.shape.is_none());e.execute(doc,Command::SetLayerTransform{id,transform:t}).unwrap();assert_eq!(e.document(doc).unwrap().layer(id).unwrap().pixels.as_ref().unwrap().width,20);
}
fn effects()->LayerEffects{serde_json::from_value(serde_json::json!({"stroke":{"size":4,"red":0,"green":1,"blue":0,"opacity":1,"inside":false,"futureStroke":9},"colorOverlay":{"red":0,"green":0,"blue":1,"opacity":0.2},"futureEffect":7})).unwrap()}
#[test]
fn effects_preview_and_commit_preserve_raw_pixels_unknown_effects_and_editable_text(){
    let mut e=Engine::new();let doc=e.new_document(100,80,false).unwrap();e.install_text(doc,None,style("Text"),raster(20,20),p(20.0,20.0),None).unwrap();let id=e.document(doc).unwrap().active_layer_id.unwrap();let before=e.document(doc).unwrap().clone();let preview=PreviewEdit::Effects{id,effects:Some(effects())};let rendered=e.composite_edit(doc,Some(&preview),Rect{x:0.0,y:0.0,width:100.0,height:80.0},100,80).unwrap();assert!(rendered.pixel(18,25)[3]>0);assert!(e.document(doc).unwrap().same_content(&before));
    e.execute(doc,Command::SetLayerEffects{id,effects:Some(effects())}).unwrap();let l=e.document(doc).unwrap().layer(id).unwrap();assert_eq!(l.pixels,before.layer(id).unwrap().pixels);assert_eq!(l.extra.text,before.layer(id).unwrap().extra.text);assert_eq!(l.extra.effects.as_ref().unwrap().unknown["futureEffect"],7);assert_eq!(l.extra.effects.as_ref().unwrap().stroke.as_ref().unwrap().unknown["futureStroke"],9);assert_eq!(e.state(doc).unwrap().undo_depth,2);e.undo(doc).unwrap();assert_eq!(e.document(doc).unwrap().manifest(),before.manifest());
}
#[test]
fn invalid_effects_refuse_without_editing_pixels_or_history(){let mut e=Engine::new();let doc=e.new_document(100,80,false).unwrap();e.install_text(doc,None,style("Text"),raster(20,20),p(20.0,20.0),None).unwrap();let id=e.document(doc).unwrap().active_layer_id.unwrap();let before=e.document(doc).unwrap().clone();let mut fx=effects();fx.stroke.as_mut().unwrap().size=501.0;assert!(e.execute(doc,Command::SetLayerEffects{id,effects:Some(fx)}).is_err());assert!(e.document(doc).unwrap().same_content(&before));assert_eq!(e.state(doc).unwrap().undo_depth,1);}
