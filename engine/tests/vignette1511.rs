use compositor_engine::*;
use compositor_engine::adjust::finishing::VignetteLayerSettings;
fn params() -> FilterParams { FilterParams::Vignette { settings: VignetteLayerSettings { amount: 100., ..Default::default() } } }
#[test]
fn absent_asset_vignette_fills_canvas_preview_equals_commit_and_undo_restores_absence() {
 let mut d=Document::new(96,64);let mut l=Layer::blank("Empty",d.size());let id=l.id;
 l.transform.origin=Point{x:19.,y:12.};l.transform.size=Size{width:10.,height:10.};d.layers.push(l);
 let request=PreviewRequest::Filter{layer:id,params:params()};
 let preview=preview::compute_preview(&d,&request,0).unwrap();
 assert_eq!((preview.raster.width,preview.raster.height),(96,64));
 assert!(preview.raster.bytes().chunks_exact(4).any(|p|p[3]>0));
 assert!(d.layer(id).unwrap().pixels.is_none());
 let mut e=Engine::new();let doc=e.insert_document(d);
 e.execute(doc,Command::ApplyFilter{id,params:params()}).unwrap();
 let changed=e.document(doc).unwrap().layer(id).unwrap();assert_eq!(changed.pixels.as_ref().unwrap().bytes(),preview.raster.bytes());assert_eq!(changed.transform.origin,Point{x:0.,y:0.});
 e.undo(doc).unwrap();assert!(e.document(doc).unwrap().layer(id).unwrap().pixels.is_none());assert_eq!(e.document(doc).unwrap().layer(id).unwrap().transform.origin,Point{x:19.,y:12.});
}
#[test]
fn clear_existing_asset_stays_clear_and_empty_layer_selection_limits_fill() {
 let mut d=Document::new(96,64);let l=Layer::with_pixels("Clear",Raster::new_transparent(96,64),Point{x:0.,y:0.});let id=l.id;d.layers.push(l);
 ops::adjust::apply_filter(&mut d,id,&params()).unwrap();assert!(d.layer(id).unwrap().pixels.as_ref().unwrap().bytes().iter().all(|b|*b==0));
 d.layers[0].pixels=None;d.selection=Some(Selection::new(vec![vec![[0,0],[48*256,0],[48*256,64*256],[0,64*256]]],false,0.));
 ops::adjust::apply_filter(&mut d,id,&params()).unwrap();let r=d.layer(id).unwrap().pixels.as_ref().unwrap();assert!(r.pixel(0,0)[3]>0);assert_eq!(r.pixel(95,0),[0;4]);
}
