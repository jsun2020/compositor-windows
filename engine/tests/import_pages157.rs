use compositor_engine::*;
fn png(w:u32,h:u32,color:[u8;4])->Vec<u8>{encode_png(&Raster::from_straight(w,h,&color.repeat((w*h)as usize)),72.).unwrap()}
#[test]fn page_one_sets_print_canvas_and_is_topmost_active_layer(){
    let mut e=Engine::new();let id=e.import_pages(None,vec![("Page 1".into(),png(20,30,[255,0,0,128])),("Page 2".into(),png(10,20,[0,255,0,255]))],300.,None).unwrap();
    let d=e.document(id).unwrap();assert_eq!((d.width,d.height,d.resolution),(20,30,300.));assert_eq!(d.layers[0].name,"Page 2");assert_eq!(d.layers[1].name,"Page 1");assert_eq!(d.active_layer_id,Some(d.layers[1].id));assert!(e.state(id).unwrap().is_modified);assert_eq!(d.layers[0].transform.origin,Point{x:5.,y:5.});
    let decoded=open_package(&e.save_package(id).unwrap()).unwrap();assert_eq!(decoded.layers[1].pixels.as_ref().unwrap().pixel(0,0),[128,0,0,128]);
}
#[test]fn all_pages_are_one_undo_step_and_failed_page_leaves_existing_document_untouched(){
    let mut e=Engine::new();let id=e.new_document(40,40,true).unwrap();let before=e.state(id).unwrap();
    let pages=vec![("Page 1".into(),png(20,30,[1,2,3,255])),("Page 2".into(),png(10,20,[4,5,6,255]))];
    e.import_pages(Some(id),pages.clone(),300.,Some(Point{x:30.,y:25.})).unwrap();assert_eq!(e.state(id).unwrap().undo_depth,before.undo_depth+1);assert_eq!(e.document(id).unwrap().layers[2].transform.origin,Point{x:20.,y:10.});e.undo(id).unwrap();assert_eq!(e.document(id).unwrap().layers.len(),before.layers.len());
    let current=e.state(id).unwrap();let mut corrupt=pages;corrupt[1].1.truncate(40);assert!(e.import_pages(Some(id),corrupt.clone(),300.,None).is_err());assert_eq!(e.state(id).unwrap(),current);
    let count=e.document_ids().len();assert!(e.import_pages(None,corrupt,300.,None).is_err());assert_eq!(e.document_ids().len(),count);
}
#[test]fn native_sixteen_bit_png_samples_survive_batch_import(){
    let mut e=Engine::new();let r=Raster::from_rgba16(2,1,vec![12345,23456,34567,65535,12346,23457,34568,65535]);
    let id=e.import_pages(None,vec![("HEIF".into(),encode_deep_png(&r,72.).unwrap())],72.,None).unwrap();let p=e.document(id).unwrap().layers[0].pixels.as_ref().unwrap();assert_eq!(p.depth(),PixelDepth::Sixteen);assert_eq!(p.deep_pixels(),r.deep_pixels());
}
