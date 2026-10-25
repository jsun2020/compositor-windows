use compositor_engine::*;
fn image(name:&str,x:f64,y:f64,w:u32,h:u32)->Layer {
    Layer::with_pixels(name,Raster::from_rgba16(w,h,[12345,23456,45678,65535].repeat((w*h)as usize)),Point{x,y})
}
#[test]
fn new_project_fits_rotated_bounds_and_preserves_folder_masks_and_pixels(){
    let mut doc=Document::new(1000,800);let mut group=Layer::blank("Folder",doc.size());group.is_group=true;let group_id=group.id;
    let mut layer=image("Deep",213.,321.,40,20);layer.parent_id=Some(group_id);layer.transform.rotation=90.;
    layer.mask=Some(Mask{pixels:GrayRaster::from_bytes(2,2,vec![255,128,64,0]),enabled:true,linked:Some(false),placement:Some(LayerTransform::axis_aligned(Point{x:215.,y:325.},Size{width:10.,height:10.}))});
    let bounds=layer.transform.bounds();let original=layer.clone();doc.layers=vec![group,layer];doc.active_layer_id=Some(group_id);
    let mut engine=Engine::new();let source=engine.insert_document(doc.clone());let copied=engine.copy_layers_to_project(source,&[group_id],None,None).unwrap();
    let result=engine.document(copied.document).unwrap();assert_eq!((result.width,result.height),((bounds.max_x().ceil()-bounds.x.floor())as u32,(bounds.max_y().ceil()-bounds.y.floor())as u32));
    assert_eq!(result.layers.len(),2);assert_eq!(result.layers[1].parent_id,Some(result.layers[0].id));assert_eq!(copied.selected,vec![result.layers[0].id]);
    let output=&result.layers[1];assert_eq!(output.pixels,original.pixels);assert_eq!(output.pixels.as_ref().unwrap().depth(),PixelDepth::Sixteen);
    let delta=Point{x:output.transform.origin.x-original.transform.origin.x,y:output.transform.origin.y-original.transform.origin.y};
    assert_eq!(output.mask.as_ref().unwrap().placement.unwrap().origin,Point{x:215.+delta.x,y:325.+delta.y});assert_eq!(engine.document(source).unwrap().layers,doc.layers);
    assert_eq!(engine.state(copied.document).unwrap().undo_depth,1);engine.undo(copied.document).unwrap();assert!(engine.document(copied.document).unwrap().layers.is_empty());
}
#[test]
fn copy_to_existing_project_keeps_relative_positions_and_internal_clipping(){
    let mut doc=Document::new(100,100);let a=image("A",10.,20.,10,10);let mut b=image("B",30.,40.,10,10);b.mask_source_id=Some(a.id);let ids=[a.id,b.id];doc.layers=vec![a,b];doc.active_layer_id=Some(ids[0]);
    let mut engine=Engine::new();let source=engine.insert_document(doc);let target=engine.new_document(200,200,false).unwrap();
    let result=engine.copy_layers_to_project(source,&ids,Some(target),Some(Point{x:80.,y:90.})).unwrap();let layers=&engine.document(target).unwrap().layers;
    assert_eq!(layers[0].transform.origin,Point{x:65.,y:75.});assert_eq!(layers[1].transform.origin,Point{x:85.,y:95.});assert_eq!(layers[1].mask_source_id,Some(layers[0].id));
    assert_eq!(result.selected.len(),2);engine.undo(target).unwrap();assert!(engine.document(target).unwrap().layers.is_empty());
}
#[test]
fn omitted_clipping_source_is_baked_without_dropping_sixteen_bit_pixels(){
    let mut doc=Document::new(2,2);let mut mask=image("Clipping source",0.,0.,2,2);mask.pixels=Some(Raster::from_straight(2,2,&[255,255,255,128].repeat(4)));
    let mut layer=image("Clipped",0.,0.,2,2);layer.mask_source_id=Some(mask.id);let id=layer.id;let before=layer.pixels.clone().unwrap();doc.layers=vec![mask,layer];doc.active_layer_id=Some(id);
    let mut engine=Engine::new();let source=engine.insert_document(doc);let copied=engine.copy_layers_to_project(source,&[id],None,None).unwrap();let layer=&engine.document(copied.document).unwrap().layers[0];
    assert!(layer.mask_source_id.is_none());let after=layer.pixels.as_ref().unwrap();assert_eq!(after.depth(),PixelDepth::Sixteen);
    for c in 0..4{assert!((after.pixel_f32(0,0)[c]-before.pixel_f32(0,0)[c]*128./255.).abs()<1./65535.);}
}
#[test]
fn animation_copy_shares_source_keeps_frames_and_refusals_leave_no_empty_tab(){
    let mut engine=Engine::new();let source=engine.import_image(None,include_bytes!("fixtures/mac157/animation.gif"),"Animation",None).unwrap();let id=engine.document(source).unwrap().active_layer_id.unwrap();
    let copied=engine.copy_layers_to_project(source,&[id],None,None).unwrap();let a=engine.document(source).unwrap().layers[0].extra.gif.as_ref().unwrap();let b=engine.document(copied.document).unwrap().layers[0].extra.gif.as_ref().unwrap();
    assert!(std::sync::Arc::ptr_eq(&a.source,&b.source));assert_eq!(a.references(),b.references());
    let saved=engine.save_package(copied.document).unwrap();engine.open_package(&saved,None).unwrap();let count=engine.document_ids().len();
    assert!(engine.copy_layers_to_project(source,&[uuid::Uuid::new_v4()],None,None).is_err());assert_eq!(engine.document_ids().len(),count);
    assert!(engine.copy_layers_to_project(source,&[id],Some(source),None).is_err());assert_eq!(engine.document_ids().len(),count);
}
