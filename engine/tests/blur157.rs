use compositor_engine::*;
#[test]fn preserving_transparency_keeps_shape_alpha_and_saturated_edges_at_all_depths(){
 let mut bytes=vec![0;9*9*4];for y in 2..7 {for x in 2..7 {let p=&mut bytes[(y*9+x)*4..][..4];p.copy_from_slice(&[255,0,0,if(x+y)%2==0{128}else{255}]);}}
 let r=Raster::from_straight(9,9,&bytes);
 for depth in [PixelDepth::Eight,PixelDepth::Sixteen,PixelDepth::Hdr]{let r=r.to_depth(depth);
  let result=adjust::filters::gaussian_preserving_alpha(&r,3.);assert_eq!(r.depth(),result.depth());
  for y in 0..9 {for x in 0..9{let(a,b)=(r.pixel_f32(x,y),result.pixel_f32(x,y));assert_eq!(a[3],b[3]);assert!((b[0]-b[3]).abs()<0.0001);assert_eq!(b[1],0.);assert_eq!(b[2],0.);}}
 }
}
#[test]fn preservation_does_not_grow_or_change_a_uniform_layer(){
 let mut e=Engine::new();let doc=e.new_document(30,30,false).unwrap();let r=Raster::from_straight(3,3,&[255,0,0,128].repeat(9));let png=encode_png(&r,72.).unwrap();e.import_image(Some(doc),&png,"Shape",Some(Point{x:10.,y:12.})).unwrap();let id=e.document(doc).unwrap().active_layer_id.unwrap();let before=e.document(doc).unwrap().layer(id).unwrap().clone();
 e.execute(doc,Command::ApplyFilter{id,params:FilterParams::GaussianBlur{radius:5.,preserve_transparency:true}}).unwrap();let after=e.document(doc).unwrap().layer(id).unwrap();assert_eq!(before.transform,after.transform);assert_eq!(before.pixels,after.pixels);
 // With a uniform red shape this is an identity, so no artificial grow/crop history entry.
}

#[test]
fn canvas_edges_extend_opaque_pixels_without_growing_at_any_depth() {
 for depth in [PixelDepth::Eight,PixelDepth::Sixteen,PixelDepth::Hdr] {
  for params in [FilterParams::GaussianBlur{radius:3.,preserve_transparency:false},FilterParams::MotionBlur{angle:37.,distance:12.}] {
   let mut e=Engine::new();let doc=e.new_document(17,13,false).unwrap();
   let r=Raster::from_straight(17,13,&[51,153,221,255].repeat(17*13)).to_depth(depth);
   let png=if depth==PixelDepth::Eight {encode_png(&r,72.).unwrap()}else if depth==PixelDepth::Hdr {encode_exr(&r).unwrap()}else{encode_deep_png(&r,72.).unwrap()};
   e.import_image(Some(doc),&png,"Canvas",None).unwrap();
   let id=e.document(doc).unwrap().active_layer_id.unwrap();let before=e.document(doc).unwrap().layer(id).unwrap().clone();
   e.execute(doc,Command::ApplyFilter{id,params}).unwrap();let after=e.document(doc).unwrap().layer(id).unwrap();
   assert_eq!(before.transform,after.transform);let result=after.pixels.as_ref().unwrap();assert_eq!((result.width,result.height),(17,13));
   for y in 0..13 {for x in 0..17 {let p=result.pixel_f32(x,y);assert_eq!(p[3],1.);for c in 0..3{assert!((p[c]-before.pixels.as_ref().unwrap().pixel_f32(x,y)[c]).abs()<0.001);}}}
  }
 }
}
#[test]
fn only_sides_at_canvas_boundary_extend_and_rotated_layers_keep_clear_edges() {
 let mut e=Engine::new();let doc=e.new_document(20,20,false).unwrap();let r=Raster::from_straight(8,10,&[255,0,0,255].repeat(80));
 e.import_image(Some(doc),&encode_png(&r,72.).unwrap(),"Edge",Some(Point{x:4.,y:9.})).unwrap();
 let d=e.document(doc).unwrap();let l=d.layer(d.active_layer_id.unwrap()).unwrap();let p=FilterParams::GaussianBlur{radius:2.,preserve_transparency:false};
 let sides=ops::adjust::filter_canvas_sides(d,&l.transform,8,10,&p);assert!(sides.left);assert!(!sides.right&&!sides.top&&!sides.bottom);
 let mut rotated=l.transform;rotated.rotation=0.1;assert_eq!(ops::adjust::filter_canvas_sides(d,&rotated,8,10,&p),adjust::filters::CanvasSides::default());
 let id=l.id;e.execute(doc,Command::ApplyFilter{id,params:p}).unwrap();let l=e.document(doc).unwrap().layer(id).unwrap();assert_eq!(l.transform.origin.x,0.);assert!(l.transform.origin.y<4.);assert!(l.transform.size.width>8.);
 assert!(l.pixels.as_ref().unwrap().pixel(0,l.pixels.as_ref().unwrap().height/2)[3]>240);
}
