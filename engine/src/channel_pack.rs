//! Straight RGBA texture packing: RGB under alpha zero is data and must survive encoding.
use crate::*;
use serde::{Serialize,Deserialize};
use uuid::Uuid;
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[serde(tag="kind",content="id")]
pub enum ChannelSource {Black,White,Layer(Uuid)}
pub fn defaults(doc:&Document)->[ChannelSource;4]{
 std::array::from_fn(|c|{
  let names=[["r","red"],["g","green"],["b","blue"],["a","alpha"]][c];
  doc.layers.iter().rev().find(|l|names.contains(&l.name.trim().to_lowercase().as_str())).map(|l|ChannelSource::Layer(l.id)).unwrap_or(if c==3{ChannelSource::White}else{ChannelSource::Black})
 })
}
fn isolate(doc:&Document,id:Uuid)->Result<Document,CommandError>{
 if doc.layer(id).is_none(){return Err(CommandError::NoLayer);}
 let mut ancestors=std::collections::HashSet::new();let mut next=Some(id);
 while let Some(parent)=next{if !ancestors.insert(parent){break;}next=doc.layer(parent).and_then(|l|l.parent_id);}
 let inside=|layer:&Layer|{let mut next=layer.parent_id;let mut depth=0;while let Some(parent)=next{if parent==id{return true;}depth+=1;if depth>doc.layers.len(){break;}next=doc.layer(parent).and_then(|l|l.parent_id);}false};
 let mut result=doc.clone();for layer in &mut result.layers{if ancestors.contains(&layer.id){layer.visible=true;}else if !inside(layer){layer.visible=false;}}
 Ok(result)
}
pub fn export(doc:&Document,cache:&EffectsCache,o:&export_as::ExportOptions,preview:u32)->Result<Vec<u8>,CommandError>{
 o.validate()?;let sources=o.channel_sources.clone().unwrap_or_else(||defaults(doc));
 let factor=if preview>0{(preview as f64/o.width.max(o.height)as f64).min(1.)}else{1.};
 let(w,h)=((o.width as f64*factor).round().max(1.)as u32,(o.height as f64*factor).round().max(1.)as u32);
 let mut bytes=vec![0;w as usize*h as usize*4];
 for (c,source)in sources.iter().enumerate(){
  if let Some(previous)=sources[..c].iter().position(|s|s==source){for p in bytes.chunks_exact_mut(4){p[c]=p[previous];}continue;}
  match source {
   ChannelSource::Black=>{},ChannelSource::White=>{for p in bytes.chunks_exact_mut(4){p[c]=255;}},
   ChannelSource::Layer(id)=>{let isolated=isolate(doc,*id)?;let raster=export_as::render(&isolated,cache,o,preview)?;
    for(p,q)in bytes.chunks_exact_mut(4).zip(raster.bytes().chunks_exact(4)){p[c]=(0.2126*q[0]as f64+0.7152*q[1]as f64+0.0722*q[2]as f64).round().clamp(0.,255.)as u8;}
   }
  }
 }
 // Preview shows RGB independently of the packed alpha plane.
 if preview>0{for p in bytes.chunks_exact_mut(4){p[3]=255;}}
 let mut out=Vec::new();{
  let mut encoder=png::Encoder::new(&mut out,w,h);encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);
  encoder.set_source_srgb(png::SrgbRenderingIntent::Perceptual);
  let ppm=(doc.resolution/0.0254).round().max(1.)as u32;encoder.set_pixel_dims(Some(png::PixelDimensions{xppu:ppm,yppu:ppm,unit:png::Unit::Meter}));
  let mut writer=encoder.write_header().map_err(|_|ProjectError::Encode)?;writer.write_image_data(&bytes).map_err(|_|ProjectError::Encode)?;
 }Ok(out)
}
