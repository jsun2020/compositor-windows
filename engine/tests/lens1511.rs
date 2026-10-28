use compositor_engine::{adjust::lens_effects::*,Raster};
use std::path::Path;
fn fixture(name:&str)->Vec<u8>{std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mac1511").join(name)).unwrap()}
#[test]
fn new_prism_controls_match_pinned_mac_reference_and_record_other_optics() {
 let input=Raster::from_premultiplied(96,64,fixture("lens-input.rgba"));
 let mut cases=vec![("Prism".to_string(),LensEffectSettings{effect:LensEffect::Prism,..Default::default()})];
 for(name,both,blur,fringe,offset)in[("Prism-Crisp",false,0.,0.,22.),("Prism-Both",true,0.,30.,22.),("Prism-Blur",true,80.,60.,22.),("Prism-ZeroOffset",false,0.,100.,0.)] {
 cases.push((name.into(),LensEffectSettings{effect:LensEffect::Prism,prism_both_ways:both,prism_blur:blur,prism_fringe:Some(fringe),prism_offset:offset,prism_angle:30.,..Default::default()}));
 }
 for threshold in [96.,98.,100.] {for(name,effect)in[("Anamorphic Flare",LensEffect::AnamorphicFlare),("Starburst",LensEffect::Starburst)] {cases.push((format!("{name}-{}",threshold as i32),LensEffectSettings{effect,threshold,..Default::default()}));}}
 let mut failures=Vec::new();
 for(name,s)in cases {
  let actual=apply_checked(&input,&s).unwrap();let expected=fixture(&format!("lens-{name}.rgba"));
  let errors:Vec<_>=actual.bytes().chunks_exact(4).zip(expected.chunks_exact(4)).flat_map(|(a,b)|(0..3).map(move|c|a[c].abs_diff(b[c]))).collect();
  let max=*errors.iter().max().unwrap();let mean=errors.iter().map(|v|*v as f64).sum::<f64>()/errors.len()as f64;
  eprintln!("{name}: max={max} mean={mean:.4}");
  for(a,b)in actual.bytes().chunks_exact(4).zip(input.bytes().chunks_exact(4)){assert_eq!(a[3],b[3]);}
  if let Ok(dir)=std::env::var("LENS1511_DIAGNOSTICS") {std::fs::write(Path::new(&dir).join(format!("{name}.rgba")),actual.bytes()).unwrap();}
  if s.effect==LensEffect::Prism && !(max<=8 && mean<=1.) {failures.push(format!("{name}: max={max} mean={mean}"));}
 }
 assert!(failures.is_empty(),"{}",failures.join("; "));
}
#[test]
fn new_defaults_and_legacy_missing_fringe_are_distinct_but_render_equivalent() {
 let d=LensEffectSettings::default();assert_eq!((d.prism_amount,d.prism_copies,d.prism_offset),(80.,5.,6.));
 let old:LensEffectSettings=serde_json::from_str(r#"{"effect":"Prism","prismOffset":0}"#).unwrap();
 assert_eq!(old.prism_fringe,None);assert!(!old.is_identity());
 let input=Raster::from_premultiplied(96,64,fixture("lens-input.rgba"));
 assert_eq!(apply(&input,&old),apply(&input,&LensEffectSettings{prism_fringe:Some(30.),..old}));
}
