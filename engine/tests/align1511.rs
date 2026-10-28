use compositor_engine::*;
use compositor_engine::ops::align::*;
fn layer(x:f64,y:f64,w:u32,h:u32)->Layer {Layer::with_pixels("L",Raster::new_transparent(w,h),Point{x,y})}
#[test]
fn alignment_uses_canvas_selection_or_multi_item_bounds_and_one_undo() {
 let mut d=Document::new(101,80);let a=layer(10.,9.,20,10);let b=layer(70.,30.,10,20);let ids=vec![a.id,b.id];d.layers.extend([a,b]);
 align(&mut d,&ids[..1],LayerAlignment::HorizontalCenter).unwrap();assert_eq!(d.layers[0].transform.origin.x,41.);
 align(&mut d,&ids,LayerAlignment::Right).unwrap();assert_eq!(d.layers[0].transform.origin.x,60.);assert_eq!(d.layers[1].transform.origin.x,70.);
 d.selection=Some(Selection::new(vec![vec![[5*256,7*256],[35*256,7*256],[35*256,27*256],[5*256,27*256]]],false,8.));
 let before=d.clone();let mut e=Engine::new();let doc=e.insert_document(d);
 e.execute(doc,Command::AlignLayers{ids,alignment:LayerAlignment::Top}).unwrap();
 assert!(e.document(doc).unwrap().layers.iter().all(|l|l.transform.origin.y==7.));
 e.undo(doc).unwrap();assert!(e.document(doc).unwrap().same_content(&before));
}
#[test]
fn selected_folder_and_descendant_move_once_hidden_layers_stay_put_and_mask_link_is_respected() {
 let mut d=Document::new(100,100);let mut g=Layer::blank("Folder",d.size());g.is_group=true;let gid=g.id;
 let mut a=layer(10.,10.,10,10);a.parent_id=Some(gid);let aid=a.id;
 a.set_mask(Some(Mask{pixels:GrayRaster::from_bytes(10,10,vec![255;100]),enabled:true,placement:None,linked:Some(false)}));
 let mut b=layer(30.,20.,10,10);b.parent_id=Some(gid);let mut hidden=layer(1.,1.,10,10);hidden.parent_id=Some(gid);hidden.visible=false;
 d.layers.extend([g,a,b,hidden]);
 align(&mut d,&[gid,aid],LayerAlignment::Right).unwrap();
 assert_eq!(d.layer(aid).unwrap().transform.origin.x,70.);assert_eq!(d.layers[2].transform.origin.x,90.);assert_eq!(d.layers[3].transform.origin.x,1.);
 assert_eq!(d.layer(aid).unwrap().mask.as_ref().unwrap().placement.unwrap().origin.x,10.);
}
#[test]
fn distribute_preserves_outermost_and_equalizes_centers_or_gaps() {
 for (distribution,expected) in [(LayerDistribution::HorizontalCenters,43.),(LayerDistribution::HorizontalSpacing,40.)] {
  let mut d=Document::new(100,100);d.layers=vec![layer(0.,0.,10,10),layer(20.,0.,20,10),layer(90.,0.,20,10)];let ids:Vec<_>=d.layers.iter().map(|l|l.id).collect();
  distribute(&mut d,&ids,distribution).unwrap();assert_eq!(d.layers[0].transform.origin.x,0.);assert_eq!(d.layers[2].transform.origin.x,90.);assert_eq!(d.layers[1].transform.origin.x,expected);
 }
}
