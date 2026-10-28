use compositor_engine::*;
use compositor_engine::adjust::finishing::*;
fn fixture(name:&str)->Vec<u8>{std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mac1511").join(name)).unwrap()}
#[test]
fn vignette_matches_pinned_native_kernel_and_preserves_alpha() {
 let input=Raster::from_premultiplied(96,64,fixture("lens-input.rgba"));
 for amount in [0.,35.,100.] {for roundness in [-100.,0.,100.] {
  let s=VignetteLayerSettings{amount,roundness,color:AdjustmentColor{red:0.2,green:0.1,blue:0.3},..Default::default()};
  let result=vignette(&input,&s,Rect{x:0.,y:0.,width:96.,height:64.},false);
  assert_eq!(result.bytes(),fixture(&format!("vignette-{}-{}.rgba",amount as i32,roundness as i32)));
  for(a,b)in result.bytes().chunks_exact(4).zip(input.bytes().chunks_exact(4)){assert_eq!(a[3],b[3]);}
 }}
}
#[test]
fn tonal_contrast_has_bounded_cross_platform_gaussian_error() {
 let input=Raster::from_premultiplied(96,64,fixture("lens-input.rgba"));
 let mut failures=Vec::new();
 for radius in [1.,16.,100.] {for amount in [50.,100.] {
  let result=tonal(&input,&TonalLayerSettings{amount,radius,..Default::default()});
  let expected=fixture(&format!("tonal-{}-{}.rgba",radius as i32,amount as i32));
  let errors:Vec<_>=result.bytes().iter().zip(&expected).map(|(a,b)|a.abs_diff(*b)).collect();
  let max=*errors.iter().max().unwrap();let mean=errors.iter().map(|v|*v as f64).sum::<f64>()/errors.len()as f64;eprintln!("tonal {radius}/{amount}: max={max} mean={mean}");
  // The portable Gaussian and Core Image are different implementations.
  // At radius 100 the blurred alpha is only a few byte levels: one base-byte
  // difference is amplified by unpremultiplication (observed max 6/255).
  // This NEW cross-platform bound is not a byte-exact claim or a performance
  // budget. The native tonal stage is checked byte-exact separately.
  if max>8 || mean>0.5 {failures.push(format!("{radius}/{amount}: {max}/{mean}"));}
 }}assert!(failures.is_empty(),"{}",failures.join("; "));
}

#[test]
fn tonal_blur_stage_has_bounded_core_image_error() {
 let input=Raster::from_premultiplied(96,64,fixture("lens-input.rgba"));
 let mut failures=Vec::new();
 for radius in [1.,16.,100.] {
  let actual=adjust::filters::gaussian_blur(&input,radius);let expected=fixture(&format!("tonal-blurred-{}.rgba",radius as i32));
  let max=actual.bytes().iter().zip(&expected).map(|(a,b)|a.abs_diff(*b)).max().unwrap();eprintln!("tonal blur {radius}: max {max}");
  if let Ok(dir)=std::env::var("TONAL1511_DIAGNOSTICS"){std::fs::write(std::path::Path::new(&dir).join(format!("tonal-blur-{radius}.rgba")),actual.bytes()).unwrap();}
  if max>if radius==1. {4}else{1} {failures.push(format!("radius {radius}: {max}"));}
 }assert!(failures.is_empty(),"{}",failures.join("; "));
}
