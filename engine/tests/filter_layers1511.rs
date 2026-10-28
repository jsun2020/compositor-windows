use compositor_engine::*;
#[test]fn finishing_layers_partial_views_match_full_render(){
 for depth in [PixelDepth::Eight,PixelDepth::Sixteen]{for kind in [AdjustmentKind::Vignette,AdjustmentKind::LensEffects,AdjustmentKind::TonalContrast,AdjustmentKind::Sharpen]{
  let mut d=Document::new(64,48);let mut p=vec![255;64*48*4];for y in 0..48{for x in 0..64{p[(y*64+x)*4]=((x*7+y*3)%256)as u8;p[(y*64+x)*4+1]=if x%4<2{0}else{255};}}
  let source=Raster::from_straight(64,48,&p).to_depth(depth);d.layers.push(Layer::with_pixels("Source",source.clone(),Point{x:0.,y:0.}));
  let mut a=Layer::blank("Filter",d.size());let mut settings=adjust::filter_layer::FilterLayerSettings::default();settings.progressive_to=4.;settings.progressive_start=Some(Point{x:10.,y:5.});settings.progressive_end=Some(Point{x:40.,y:40.});settings.bloom_radius=3.;settings.rgb_split_distance=3.;
  let mut adjustment=LayerAdjustment::new(kind);adjustment.filter_layer_settings=Some(settings);a.extra.adjustment=Some(adjustment);d.layers.push(a);
  let full=compositor::render_full(&d).unwrap();assert_ne!(full.bytes(),source.bytes());assert_eq!(full.depth(),depth);
  let part=compositor::composite(&d,Rect{x:16.,y:12.,width:20.,height:18.},20,18);
  for y in 0..18{for x in 0..20{for c in 0..4{let delta=(full.pixel_f32(x+16,y+12)[c]-part.pixel_f32(x,y)[c]).abs();assert!(delta<=if depth==PixelDepth::Eight{1./255.}else{2./65535.},"{kind:?} {depth:?} at {x},{y}/{c}: {delta}");}}}
 }}
}

#[test]
fn lens_cache_reuses_optics_for_mask_edits_and_invalidates_changed_pixels() {
 let mut d=Document::new(48,32);
 let mut source=vec![0,0,0,255].repeat(48*32);source[(16*48+24)*4..][..3].fill(255);
 d.layers.push(Layer::with_pixels("Source",Raster::from_premultiplied(48,32,source.clone()),Point{x:0.,y:0.}));
 let mut l=Layer::blank("Lens",d.size());let mut a=LayerAdjustment::new(AdjustmentKind::LensEffects);
 a.filter_layer_settings=Some(adjust::filter_layer::FilterLayerSettings{lens:Some(adjust::lens_effects::LensEffectSettings{effect:adjust::lens_effects::LensEffect::Prism,..Default::default()}),..Default::default()});l.extra.adjustment=Some(a);d.layers.push(l);
 let cache=EffectsCache::new(2,48*32*4*2);
 let first=compositor::render_full_with(&d,&cache).unwrap();assert_eq!(cache.made(),1);assert!(cache.bytes()<=48*32*4*2);
 d.layers[1].set_mask(Some(Mask{pixels:GrayRaster::from_bytes(48,32,vec![0;48*32]),enabled:true,linked:None,placement:None}));
 let hidden=compositor::render_full_with(&d,&cache).unwrap();assert_eq!(hidden.bytes(),source);assert_ne!(hidden,first);assert_eq!(cache.made(),1,"mask coverage must not invalidate optical input");
 d.layers[1].set_mask(None);source[(10*48+12)*4..][..3].fill(255);d.layers[0].set_pixels(Some(Raster::from_premultiplied(48,32,source)));
 let changed=compositor::render_full_with(&d,&cache).unwrap();assert_ne!(changed,first);assert_eq!(cache.made(),2);assert!(cache.bytes()<=48*32*4*2);
}

#[test]
fn inspecting_lens_pixels_at_extreme_zoom_reuses_native_canvas_grid() {
 let mut d=Document::new(32,24);let mut bytes=vec![80,40,20,255].repeat(32*24);bytes[(12*32+16)*4..][..3].fill(255);
 d.layers.push(Layer::with_pixels("Source",Raster::from_premultiplied(32,24,bytes),Point{x:0.,y:0.}));
 let mut l=Layer::blank("Lens",d.size());let mut a=LayerAdjustment::new(AdjustmentKind::LensEffects);
 a.filter_layer_settings=Some(adjust::filter_layer::FilterLayerSettings{lens:Some(adjust::lens_effects::LensEffectSettings{effect:adjust::lens_effects::LensEffect::Prism,..Default::default()}),..Default::default()});l.extra.adjustment=Some(a);d.layers.push(l);
 let cache=EffectsCache::default();let full=compositor::render_full_with(&d,&cache).unwrap();assert_eq!(cache.made(),1);
 let zoom=compositor::composite_edit_with(&d,None,Rect{x:16.,y:12.,width:0.01,height:0.01},64,64,&cache);
 assert_eq!(cache.made(),1,"extreme zoom must not compute an expanded full-canvas optical surface");
 assert_eq!((zoom.width,zoom.height),(64,64));assert!(zoom.bytes().chunks_exact(4).all(|p|p[3]==255));
 let expected=compositor::sample(&full,16.005,12.005,false);for c in 0..3 {assert!((zoom.pixel(32,32)[c]as f32-expected[c]*255.).abs()<=1.);}
}

#[test]
fn lens_render_memory_refusal_preserves_document_and_allows_smaller_preview() {
 let mut engine=Engine::new();let id=engine.new_document(4000,2000,false).unwrap();
 engine.execute(id,Command::AddAdjustmentLayer{kind:AdjustmentKind::LensEffects,seed:0,shadows:None,highlights:None}).unwrap();
 let before=engine.document(id).unwrap().clone();
 let region=Rect{x:0.,y:0.,width:4000.,height:2000.};
 assert!(engine.composite(id,region,4000,2000).unwrap_err().to_string().contains("working memory limit"));
 assert!(engine.export_png(id).unwrap_err().to_string().contains("working memory limit"));
 assert_eq!(engine.document(id).unwrap().layers,before.layers);
 let preview=engine.composite(id,region,64,32).unwrap();assert_eq!((preview.width,preview.height),(64,32));
 // Opening/saving the project is still allowed; only the oversized render is refused.
 let adjustment=before.active_layer_id.unwrap();
 engine.execute(id,Command::SetLayerVisible{id:adjustment,visible:false}).unwrap();
 assert!(engine.composite(id,Rect{x:0.,y:0.,width:1.,height:1.},1,1).is_ok());
}
