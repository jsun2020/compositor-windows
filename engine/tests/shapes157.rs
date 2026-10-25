use compositor_engine::*;
use compositor_engine::ops::shape::{shape_raster,shape_record,redraw_image};
use serde_json::{Value,json};
#[test]
fn upstream_core_graphics_shape_geometry_and_coverage() {
 let cases:Vec<Value>=serde_json::from_str(include_str!("fixtures/mac157/shapes.json")).unwrap();
 for c in cases {
  let spec:ShapeSpec=serde_json::from_value(c["spec"].clone()).unwrap();let b=spec.bounds();
  assert_eq!([b.x,b.y,b.width,b.height],c["bounds"].as_array().unwrap().iter().map(|v|v.as_f64().unwrap()).collect::<Vec<_>>().as_slice(),"{}",c["file"]);
  let r=shape_raster(&spec,[1.;3]);let bytes=std::fs::read(format!("{}/tests/fixtures/mac157/{}",env!("CARGO_MANIFEST_DIR"),c["file"].as_str().unwrap())).unwrap();
  assert_eq!(r.bytes().len(),bytes.len());
  let max=r.bytes().iter().zip(&bytes).map(|(a,b)|a.abs_diff(*b)).max().unwrap();let mean=r.bytes().iter().zip(&bytes).map(|(a,b)|a.abs_diff(*b)as f64).sum::<f64>()/bytes.len()as f64;
  eprintln!("{} max={max} mean={mean:.3}",c["file"]);
  // Core Graphics uses a distinct thin-stroke rasterizer: its 1 px round dashes
  // have 127-valued cap pixels while exact area coverage gives 226. Keep these
  // edge-only differences explicit; the independent area test below checks geometry.
  let thin=c["spec"]["width"].as_f64()==Some(1.0);
  let round=c["spec"]["lineCap"]=="Round";
  let limit=if thin&&round {110}else if thin {40}else{12};
  assert!(max<=limit,"{}: edge difference {max}",c["file"]);
  for(a,b)in r.bytes().iter().zip(&bytes) {
    if *a==255 {assert!(*b>=250,"{}: solid interior changed",c["file"]);}
    if *a==0 {assert!(*b<=if thin{40}else{10},"{}: geometry spills beyond outline",c["file"]);}
  }
  // Editable metadata redraws exactly into its original pixel box.
  let record=shape_record(&spec,[1.;3]);
  if b.width.fract()==0. && b.height.fract()==0. {assert_eq!(redraw_image(&record,Size{width:b.width,height:b.height},1.).unwrap(),r);}
 }
}
#[test]
fn shape_format_gates_and_roundtrip_keep_new_settings() {
 for(spec,version)in [(json!({"kind":"Rectangle","rect":{"x":0,"y":0,"width":80,"height":60},"cornerRadius":12,"smoothing":true}),14),
 (json!({"kind":"Line","start":[10,10],"end":[50,30],"width":5,"lineStyle":"Dotted","lineCap":"Square"}),15)] {
  let mut e=Engine::new();let id=e.new_document(100,100,false).unwrap();e.execute(id,Command::AddShape{shape:serde_json::from_value(spec).unwrap(),color:[1.,0.,0.]}).unwrap();
  let d=e.document(id).unwrap();let mut m=d.manifest();assert_eq!(m.version,version);m.version=version-1;assert!(m.validate().is_err());
  let saved=e.save_package(id).unwrap();let reopened=e.open_package(&saved,None).unwrap();assert_eq!(e.document(reopened).unwrap().layers[0].extra.shape,e.document(id).unwrap().layers[0].extra.shape);
  e.undo(id).unwrap();assert!(e.document(id).unwrap().layers.is_empty());e.redo(id).unwrap();assert_eq!(e.document(id).unwrap().manifest().version,version);
 }
}

#[test]
fn dashed_lines_follow_independent_area_coverage() {
 use compositor_engine::ops::shape::{ShapeLineCap as Cap,ShapeLineStyle as Style};
 for cap in [Cap::Square,Cap::Round] {for style in [Style::Solid,Style::Dotted,Style::Dashed,Style::LongDash] {for width in [1.,3.] {
  let(a,b)=(Point{x:2.,y:2.},Point{x:20.,y:11.});let spec=ShapeSpec::Line{start:a,end:b,width,style,cap};let bounds=spec.bounds();let r=shape_raster(&spec,[1.;3]);
  let length=(b.x-a.x).hypot(b.y-a.y);let(ux,uy)=((b.x-a.x)/length,(b.y-a.y)/length);let total=length+width;
  let(dash,gap)=match style{Style::Solid=>(total,0.),Style::Dotted=>(width,width),Style::Dashed=>(3.*width,2.*width),Style::LongDash=>(7.*width,2.*width)};
  let inside=|x:f64,y:f64|{let dx=x-a.x;let dy=y-a.y;let along=dx*ux+dy*uy+width/2.;let across=-dx*uy+dy*ux;let mut d=0.;
   while d<total {let next=(d+dash).min(total);
    let yes=if cap==Cap::Square {along>=d&&along<=next&&across.abs()<=width/2.}else{
     let from=d+width/2.;let to=next-width/2.;let nearest=if to-from>0.001{along.clamp(from,to)}else{(d+next)/2.};(along-nearest).hypot(across)<=width/2.
    };if yes{return true;}d=next+gap;
   }false};
  for y in 0..r.height {for x in 0..r.width {let mut covered=0u32;
   for sy in 0..64 {for sx in 0..64 {if inside(bounds.x+x as f64+(sx as f64+0.5)/64.,bounds.y+y as f64+(sy as f64+0.5)/64.){covered+=1;}}}
   let expected=(covered as f64/4096.*255.).round()as u8;assert!(r.pixel(x,y)[3].abs_diff(expected)<=4,"{spec:?} pixel {x},{y}: {} vs {expected}",r.pixel(x,y)[3]);
  }}
 }}}
}
