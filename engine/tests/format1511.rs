use compositor_engine::*;
use compositor_engine::adjust::{filter_layer::FilterLayerSettings,lens_effects::LensEffectSettings,finishing::SharpenLayerSettings};
#[test]
fn new_adjustments_save_roundtrip_and_refuse_older_headers() {
 for(kind,version)in[(AdjustmentKind::Vignette,18),(AdjustmentKind::LensEffects,18),(AdjustmentKind::TonalContrast,18),(AdjustmentKind::Sharpen,19)] {
  let mut e=Engine::new();let id=e.new_document(30,20,false).unwrap();e.execute(id,Command::AddAdjustmentLayer{kind,seed:0,shadows:None,highlights:None}).unwrap();
  let d=e.document(id).unwrap();let mut m=d.manifest();assert_eq!(m.version,version);m.version=version-1;assert!(m.validate().is_err());
  let expected=d.layers[0].extra.adjustment.clone();let saved=e.save_package(id).unwrap();let reopened=e.open_package(&saved,None).unwrap();assert_eq!(e.document(reopened).unwrap().layers[0].extra.adjustment,expected);
 }
}
#[test]
fn explicit_fringe_promotes_to_20_while_legacy_lens_stays_18() {
 let old:LensEffectSettings=serde_json::from_str(r#"{"effect":"Prism"}"#).unwrap();
 let mut a=LayerAdjustment::new(AdjustmentKind::LensEffects);a.filter_layer_settings=Some(FilterLayerSettings{lens:Some(old),..Default::default()});assert_eq!(a.minimum_version(),18);
 a.filter_layer_settings.as_mut().unwrap().lens.as_mut().unwrap().prism_fringe=Some(0.);assert_eq!(a.minimum_version(),20);
 let mut d=Document::new(30,20);let mut l=Layer::blank("Lens",d.size());l.extra.adjustment=Some(a);d.layers.push(l);
 let mut m=d.manifest();assert_eq!(m.version,20);assert!(m.validate().is_ok());m.version=19;assert!(m.validate().is_err());
}
#[test]
fn invalid_nested_settings_cannot_hide_behind_another_active_adjustment() {
 let mut f=FilterLayerSettings::default();f.sharpen=Some(SharpenLayerSettings{amount:301.,radius:1.});assert!(!f.is_valid());
 let mut lens=LensEffectSettings::default();lens.prism_blur=f64::NAN;f.sharpen=None;f.lens=Some(lens);assert!(!f.is_valid());
}

#[test]
fn original_swift_codable_settings_survive_rust_save_and_return() {
 let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mac1511");
 for (i,kind) in [AdjustmentKind::Vignette,AdjustmentKind::TonalContrast,AdjustmentKind::Sharpen,AdjustmentKind::LensEffects].into_iter().enumerate() {
  let name=format!("model-filter-{i}.json");let text=std::fs::read_to_string(root.join(&name)).unwrap();
  let settings:FilterLayerSettings=serde_json::from_str(&text).unwrap();assert!(settings.is_valid());
  let mut d=Document::new(96,64);let mut l=Layer::blank("Mac settings",d.size());let mut a=LayerAdjustment::new(kind);a.filter_layer_settings=Some(settings.clone());l.extra.adjustment=Some(a);d.layers.push(l);
  let saved=save_package(&d).unwrap();let reopened=open_package(&saved).unwrap();
  let actual=reopened.layers[0].extra.adjustment.as_ref().unwrap().filter_layer_settings.as_ref().unwrap();assert_eq!(actual,&settings);
  if let Ok(dir)=std::env::var("MAC1511_CODED_OUTPUT") {std::fs::create_dir_all(&dir).unwrap();std::fs::write(std::path::Path::new(&dir).join(name),serde_json::to_vec_pretty(actual).unwrap()).unwrap();}
 }
}
