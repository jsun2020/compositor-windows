use compositor_engine::*;
use compositor_engine::adjust::rgb_split::RgbSplitSettings;
#[test]
fn zero_split_is_identity_and_radial_center_stays_fixed() {
 let r=Raster::from_premultiplied(64,48,include_bytes!("fixtures/mac157/rgb-input.rgba").to_vec());
 let s=RgbSplitSettings{distance:0.,blur:0.,..Default::default()};assert_eq!(s.apply(&r),r);
 let s=RgbSplitSettings{distance:40.,blur:40.,radial:true,center:Some(Point{x:32.5,y:24.5}),reach:24.,..Default::default()};
 let result=s.apply(&r);assert_eq!(result.pixel(32,24),r.pixel(32,24));assert!(result.bytes().chunks_exact(4).all(|p|p[..3].iter().all(|v|*v<=p[3])));
}
#[test]
fn upstream_metal_rgb_split_reference_comparison() {
 let r=Raster::from_premultiplied(64,48,include_bytes!("fixtures/mac157/rgb-input.rgba").to_vec());
 for (name,distance,blur,angle,radial,expected) in [
 ("linear",8.,0.,0.,false,include_bytes!("fixtures/mac157/rgb-linear.rgba").as_slice()),
 ("smear",0.,24.,0.,false,include_bytes!("fixtures/mac157/rgb-smear.rgba").as_slice()),
 ("radial",12.,8.,0.,true,include_bytes!("fixtures/mac157/rgb-radial.rgba").as_slice()),
 ("angled",10.,12.,45.,false,include_bytes!("fixtures/mac157/rgb-angled.rgba").as_slice())] {
   let s=RgbSplitSettings{distance,blur,angle,radial,center:Some(Point{x:32.,y:24.}),reach:32.};let result=s.apply(&r);
   let max=result.bytes().iter().zip(expected).map(|(a,b)|a.abs_diff(*b)).max().unwrap();
   let mean=result.bytes().iter().zip(expected).map(|(a,b)|a.abs_diff(*b) as f64).sum::<f64>()/expected.len() as f64;
   let mut errors:Vec<_>=result.bytes().iter().zip(expected).map(|(a,b)|a.abs_diff(*b)).collect();errors.sort_unstable();let p99=errors[errors.len()*99/100];
   eprintln!("RGB Split {name}: max {max}, p99 {p99}, mean {mean:.4}");
   // GPU fast-sin and CPU sin produce different grain at fract() wrap boundaries.
   // The kernel unit test fixes Metal's tap jitter and checks every channel separately.
   assert!(mean<=0.5 && p99<=2,"{name}: grain-distribution mismatch max={max}, p99={p99}, mean={mean}");
 }
}
