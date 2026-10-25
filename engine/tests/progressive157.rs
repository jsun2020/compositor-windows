use compositor_engine::*;
use compositor_engine::adjust::{progressive::ProgressiveBlurSettings,filters::CanvasSides};
fn stripes(depth:PixelDepth)->Raster {
 let mut p=vec![255;200*200*4];for y in 0..200 {for x in 0..200 {for c in 0..3{p[(y*200+x)*4+c]=if x%4<2{0}else{255};}}}
 Raster::from_straight(200,200,&p).to_depth(depth)
}
fn contrast(r:&Raster,x:u32,y:u32)->u8 {let a:Vec<_>=(x..x+8).map(|x|r.pixel(x,y)[0]).collect();a.iter().max().unwrap()-a.iter().min().unwrap()}
#[test]fn linear_radial_inverted_and_two_nonzero_ends_follow_the_handles(){
 let input=stripes(PixelDepth::Eight);let mut s=ProgressiveBlurSettings{to:6.,start:Some(Point{x:100.,y:20.}),end:Some(Point{x:100.,y:180.}),..Default::default()};
 let sides=CanvasSides{left:true,right:true,top:true,bottom:true};let r=s.apply(&input,sides);
 assert!(contrast(&r,96,10)>230);assert!(contrast(&r,96,100)<contrast(&r,96,30));assert!(contrast(&r,96,190)<40);
 s.from=1.5;let r=s.apply(&input,sides);assert!(contrast(&r,96,10)<230&&contrast(&r,96,10)>contrast(&r,96,190));
 s.from=0.;s.radial=true;s.start=Some(Point{x:100.,y:100.});s.end=Some(Point{x:160.,y:100.});let r=s.apply(&input,sides);
 assert!(contrast(&r,96,100)>230);assert!(contrast(&r,4,4)<40);
 s.inverted=true;let r=s.apply(&input,sides);assert!(contrast(&r,96,100)<40);assert!(contrast(&r,4,4)>230);
}
#[test]fn high_precision_and_hdr_highlights_survive_progressive_blur(){
 for depth in [PixelDepth::Sixteen,PixelDepth::Hdr]{
  let r=Raster::from_rgba16(12,10,[12345,24691,45678,65535].repeat(120)).to_depth(depth);
  let r=if depth==PixelDepth::Hdr {r.hdr_exposure(2.,0.,1.)}else{r};
  let s=ProgressiveBlurSettings{from:1.,to:3.,..Default::default()};let output=s.apply(&r,CanvasSides{left:true,right:true,top:true,bottom:true});
  assert_eq!(output.depth(),depth);for y in 0..10{for x in 0..12{for c in 0..4{assert!((r.pixel_f32(x,y)[c]-output.pixel_f32(x,y)[c]).abs()<0.004,"{depth:?} {x},{y}/{c}: {:?} -> {:?}",r.pixel_f32(x,y),output.pixel_f32(x,y));}}}
 }
}
#[test]fn document_handles_map_into_grown_and_reduced_raster_coordinates(){
 let p=FilterParams::ProgressiveBlur{settings:ProgressiveBlurSettings{start:Some(Point{x:30.,y:40.}),end:Some(Point{x:90.,y:60.}),..Default::default()}};
 let t=LayerTransform{origin:Point{x:10.,y:20.},size:Size{width:100.,height:80.},rotation:0.,flip_x:false,flip_y:false,sampling:Sampling::High};
 let FilterParams::ProgressiveBlur{settings}=p.scaled(0.5).mapped(&t,50,40)else{unreachable!()};
 assert_eq!(settings.start,Some(Point{x:10.,y:10.}));assert_eq!(settings.end,Some(Point{x:40.,y:20.}));assert_eq!(settings.to,10.);
}
