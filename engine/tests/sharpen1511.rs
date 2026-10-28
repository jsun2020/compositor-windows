use compositor_engine::*;
use compositor_engine::adjust::{finishing,filters::CanvasSides};
#[test]
fn sharpening_matches_core_image_and_keeps_alpha() {
 let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mac1511");
 let input=Raster::from_premultiplied(96,64,std::fs::read(root.join("lens-input.rgba")).unwrap());
 let mut failures=Vec::new();
 for radius in [1.,4.,12.] {for amount in [50.,100.,300.] {for clamp in [false,true] {
  let actual=finishing::sharpen(&input,amount,radius,CanvasSides{left:clamp,right:clamp,top:clamp,bottom:clamp});
  if let Ok(dir)=std::env::var("SHARPEN1511_DIAGNOSTICS"){std::fs::write(std::path::Path::new(&dir).join(format!("sharpen-{radius}-{amount}-{clamp}.rgba")),actual.bytes()).unwrap();}
  let expected=std::fs::read(root.join(format!("sharpen-{}-{}-{clamp}.rgba",radius as i32,amount as i32))).unwrap();
  let errors:Vec<_>=actual.bytes().iter().zip(expected).map(|(a,b)|a.abs_diff(b)).collect();
  let max=*errors.iter().max().unwrap();let mean=errors.iter().map(|v|*v as f64).sum::<f64>()/errors.len()as f64;
  eprintln!("sharpen {radius}/{amount}/{clamp}: max={max} mean={mean}");
  if max>8 || mean>1. {failures.push(format!("{radius}/{amount}/{clamp}: max={max} mean={mean}"));}
  for(a,b)in actual.bytes().chunks_exact(4).zip(input.bytes().chunks_exact(4)){assert_eq!(a[3],b[3]);}
 }}}
 assert!(failures.is_empty(),"{}",failures.join("; "));
}
