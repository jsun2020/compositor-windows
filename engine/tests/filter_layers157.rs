use compositor_engine::*;
#[test]fn bloom_matches_independent_core_image_references(){
 let r=Raster::from_premultiplied(64,48,include_bytes!("fixtures/mac157/bloom-input.rgba").to_vec());
 for(radius,amount)in [(2.,25.),(4.,50.),(8.,100.)]{
  let result=adjust::bloom::bloom(&r,amount,radius);let expected=std::fs::read(format!("{}/tests/fixtures/mac157/bloom-{}-{}.rgba",env!("CARGO_MANIFEST_DIR"),radius as i32,amount as i32)).unwrap();
  let max=result.bytes().iter().zip(&expected).map(|(a,b)|a.abs_diff(*b)).max().unwrap();let mean=result.bytes().iter().zip(&expected).map(|(a,b)|a.abs_diff(*b)as f64).sum::<f64>()/expected.len()as f64;
  eprintln!("Bloom {radius}/{amount}: max {max}, mean {mean}");assert!(max<=4&&mean<=1.,"Bloom {radius}/{amount}: max {max}, mean {mean}");
 }
}
#[test]fn new_adjustment_kinds_are_version_gated_and_roundtrip_with_undo(){
 for kind in [AdjustmentKind::ProgressiveBlur,AdjustmentKind::BloomGlow,AdjustmentKind::RgbSplit]{
  let mut e=Engine::new();let doc=e.new_document(40,30,false).unwrap();e.execute(doc,Command::AddAdjustmentLayer{kind,seed:0,shadows:None,highlights:None}).unwrap();
  let d=e.document(doc).unwrap();let mut m=d.manifest();assert_eq!(m.version,16);m.version=15;assert!(m.validate().is_err());
  let saved=e.save_package(doc).unwrap();let reopened=e.open_package(&saved,None).unwrap();assert_eq!(e.document(doc).unwrap().layers[0].extra.adjustment,e.document(reopened).unwrap().layers[0].extra.adjustment);
  e.undo(doc).unwrap();assert!(e.document(doc).unwrap().layers.is_empty());e.redo(doc).unwrap();assert_eq!(e.document(doc).unwrap().manifest().version,16);
 }
}
#[test]fn new_filter_layers_render_and_partial_views_keep_document_coordinates(){
 for depth in [PixelDepth::Eight,PixelDepth::Sixteen]{for kind in [AdjustmentKind::ProgressiveBlur,AdjustmentKind::BloomGlow,AdjustmentKind::RgbSplit]{
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
fn reduced_filter_layers_keep_subpixel_radii() {
    // One canvas pixel is 0.1 output pixels at this zoom. Clamping it back
    // to one output pixel visibly spills the bright stripe into its neighbors.
    let mut bytes = [0u8, 0, 0, 255].repeat(9 * 9);
    for y in 0..9 { bytes[(y * 9 + 4) * 4..(y * 9 + 4) * 4 + 3].fill(255); }
    let raster = Raster::from_premultiplied(9, 9, bytes);
    let mut layer = LayerAdjustment::new(AdjustmentKind::BloomGlow);
    let mut settings = adjust::filter_layer::FilterLayerSettings::default();
    settings.bloom_radius = 1.;
    settings.bloom_amount = 100.;
    layer.filter_layer_settings = Some(settings);
    let result = layer.apply_spatial_filter(&raster, Rect{x:0., y:0., width:90., height:90.});
    assert_eq!(result.bytes(), raster.bytes());
    let normal = layer.apply_spatial_filter(&raster, Rect{x:0., y:0., width:9., height:9.});
    assert!(normal.pixel(3,4)[0] > 0);
}
