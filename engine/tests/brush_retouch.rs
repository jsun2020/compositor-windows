use compositor_engine::*;
use uuid::Uuid;
fn p(x:f64,y:f64)->Point{Point{x,y}}
fn setup()->(Engine,Uuid,Uuid){let mut e=Engine::new();let doc=e.new_document(80,64,true).unwrap();let id=e.document(doc).unwrap().active_layer_id.unwrap();(e,doc,id)}
fn stroke(points:Vec<Point>)->BrushSpec{BrushSpec{diameter:12.0,hardness:1.0,opacity:0.5,points,color:[1.0,0.0,0.0,1.0],erasing:false,operation:BrushOperation::Paint}}
fn paint(e:&mut Engine,doc:Uuid,id:Uuid,mask:bool,s:BrushSpec){e.execute(doc,Command::BrushStroke{id,mask,brush:s}).unwrap();}
fn image(e:&Engine,doc:Uuid)->Raster{e.composite(doc,Rect{x:0.0,y:0.0,width:80.0,height:64.0},80,64).unwrap()}
fn select(e:&mut Engine,doc:Uuid,x:f64,y:f64,w:f64,h:f64){e.execute(doc,Command::SelectShape{kind:SelectionShape::Rectangle,points:vec![p(x,y),p(x+w,y),p(x+w,y+h),p(x,y+h)],mode:SelectionMode::Replace,antialiased:false}).unwrap();}
#[test]
fn one_stroke_caps_opacity_and_allocates_only_its_footprint(){
    let(mut e,doc,id)=setup();paint(&mut e,doc,id,false,stroke(vec![p(20.0,20.0),p(50.0,20.0),p(20.0,20.0)]));
    let r=image(&e,doc);assert_eq!(r.pixel(25,20),[128,0,0,128]);assert_eq!(e.state(doc).unwrap().undo_depth,1);
    let l=e.document(doc).unwrap().layer(id).unwrap();assert!(l.pixels.as_ref().unwrap().width<80);assert!(l.pixels.as_ref().unwrap().height<64);
    e.undo(doc).unwrap();assert!(e.document(doc).unwrap().layer(id).unwrap().pixels.is_none());
}
#[test]
fn eraser_selection_and_soft_tip_are_bounded(){
    let(mut e,doc,id)=setup();let mut s=stroke(vec![p(32.0,32.0)]);s.diameter=30.0;s.opacity=1.0;s.hardness=0.0;paint(&mut e,doc,id,false,s.clone());
    let before=image(&e,doc);assert!(before.pixel(32,32)[3]>before.pixel(42,32)[3]);assert_eq!(before.pixel(48,32),[0;4]);
    select(&mut e,doc,30.0,30.0,4.0,4.0);s.erasing=true;s.hardness=1.0;paint(&mut e,doc,id,false,s);
    let erased=image(&e,doc);assert_eq!(erased.pixel(32,32),[0;4]);assert_eq!(erased.pixel(35,32),before.pixel(35,32));
    e.undo(doc).unwrap();assert_eq!(image(&e,doc).bytes(),before.bytes());
}
#[test]
fn mask_paint_grows_without_losing_pixels_or_effects(){
    let(mut e,doc,id)=setup();paint(&mut e,doc,id,false,stroke(vec![p(20.0,20.0)]));e.execute(doc,Command::AddMask{id,revealing:true}).unwrap();
    let pixels=e.document(doc).unwrap().layer(id).unwrap().pixels.clone();
    let mut s=stroke(vec![p(50.0,40.0)]);s.color=[0.0,0.0,0.0,1.0];s.opacity=1.0;paint(&mut e,doc,id,true,s);
    let l=e.document(doc).unwrap().layer(id).unwrap();assert_eq!(l.pixels,pixels);assert!(l.mask.as_ref().unwrap().placement.is_some());assert!(l.mask.as_ref().unwrap().pixels.bytes().contains(&0));
    e.undo(doc).unwrap();assert_eq!(e.document(doc).unwrap().layer(id).unwrap().pixels,pixels);
}
#[test]
fn clone_samples_source_snapshot_and_all_visible_layers(){
    let(mut e,doc,id)=setup();let mut s=stroke(vec![p(20.0,20.0)]);s.opacity=1.0;paint(&mut e,doc,id,false,s.clone());
    s.points=vec![p(40.0,20.0)];s.operation=BrushOperation::Clone{offset:p(-20.0,0.0),all_layers:false};paint(&mut e,doc,id,false,s.clone());assert_eq!(image(&e,doc).pixel(40,20),[255,0,0,255]);
    e.execute(doc,Command::AddBlankLayer).unwrap();let target=e.document(doc).unwrap().active_layer_id.unwrap();
    // An empty target is legal when all visible layers provide the source.
    s.points=vec![p(60.0,20.0)];s.operation=BrushOperation::Clone{offset:p(-40.0,0.0),all_layers:true};paint(&mut e,doc,target,false,s);assert_eq!(image(&e,doc).pixel(60,20),[255,0,0,255]);
}
#[test]
fn blur_softens_only_inside_the_brush_and_is_one_undo(){
    let(mut e,doc,id)=setup();let mut s=stroke(vec![p(30.0,30.0)]);s.opacity=1.0;paint(&mut e,doc,id,false,s.clone());let before=image(&e,doc);let depth=e.state(doc).unwrap().undo_depth;
    s.diameter=30.0;s.operation=BrushOperation::Blur{radius:5.0};paint(&mut e,doc,id,false,s);let after=image(&e,doc);assert!(after.pixel(36,30)[3]>before.pixel(36,30)[3]);assert_eq!(after.pixel(50,30),before.pixel(50,30));assert_eq!(e.state(doc).unwrap().undo_depth,depth+1);
    e.undo(doc).unwrap();assert_eq!(image(&e,doc).bytes(),before.bytes());
}
fn damaged()->(Raster,GrayRaster){let mut bytes=[60,120,180,255].repeat(32*32);let mut mask=vec![0;32*32];for y in 14..18{for x in 14..18{let i=y*32+x;bytes[i*4..i*4+4].copy_from_slice(&[255,0,0,255]);mask[i]=255;}}(Raster::from_premultiplied(32,32,bytes),GrayRaster::from_bytes(32,32,mask))}
#[test]
fn original_healing_kernels_are_deterministic_and_preserve_uncovered_pixels(){
    let(r,m)=damaged();for mode in [HealingMode::ContentAware,HealingMode::CreateTexture,HealingMode::ProximityMatch]{let a=compositor_engine::retouch::heal(&r,&m,1.0,mode,7).unwrap();let b=compositor_engine::retouch::heal(&r,&m,1.0,mode,7).unwrap();assert_eq!(a.bytes(),b.bytes());assert_ne!(a.pixel(15,15),r.pixel(15,15));assert_eq!(a.pixel(1,1),r.pixel(1,1));assert_eq!(compositor_engine::retouch::heal(&r,&m,0.0,mode,7).unwrap().bytes(),r.bytes());}}
#[test]
fn content_fill_uses_surrounding_pixels_and_refuses_when_none_exist(){let(r,m)=damaged();let a=compositor_engine::retouch::fill(&r,&m).unwrap();assert_eq!(a.pixel(15,15),[60,120,180,255]);assert_eq!(a.pixel(1,1),r.pixel(1,1));assert!(compositor_engine::retouch::fill(&r,&GrayRaster::from_bytes(32,32,vec![255;32*32])).is_err());}
#[test]
fn content_fill_grows_only_over_selected_edges_and_is_one_undo(){
    let mut d=Document::new(80,64);let l=Layer::with_pixels("Source",Raster::from_straight(32,32,&[60,120,180,255].repeat(32*32)),p(10.0,10.0));let id=l.id;d.layers.push(l);d.active_layer_id=Some(id);let mut e=Engine::new();let doc=e.insert_document(d);
    select(&mut e,doc,40.0,20.0,5.0,8.0);let before=e.document(doc).unwrap().clone();let depth=e.state(doc).unwrap().undo_depth;e.execute(doc,Command::ContentAwareFill{id}).unwrap();
    let l=e.document(doc).unwrap().layer(id).unwrap();assert_eq!(l.pixels.as_ref().unwrap().width,35);assert_eq!(l.pixels.as_ref().unwrap().height,32);assert_eq!(image(&e,doc).pixel(44,24),[60,120,180,255]);assert_eq!(e.state(doc).unwrap().undo_depth,depth+1);e.undo(doc).unwrap();assert_eq!(e.document(doc).unwrap().manifest(),before.manifest());assert_eq!(e.document(doc).unwrap().layer(id).unwrap().pixels,before.layer(id).unwrap().pixels);
}
