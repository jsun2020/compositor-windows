use compositor_engine::*;
use compositor_engine::channel_pack::{self,ChannelSource as Source};
use compositor_engine::export_as::{ExportOptions,ExportFormat};
fn options()->ExportOptions{ExportOptions{width:2,height:1,format:ExportFormat::ChannelPack,quality:1.,background:None,gif_loop:None,channel_sources:None}}
fn raw_png(bytes:&[u8])->(Vec<u8>,png::Info<'static>){let mut reader=png::Decoder::new(std::io::Cursor::new(bytes.to_vec())).read_info().unwrap();let mut out=vec![0;reader.output_buffer_size()];let frame=reader.next_frame(&mut out).unwrap();out.truncate(frame.buffer_size());(out,reader.info().clone())}
#[test]fn packed_rgb_survives_zero_alpha_and_preview_is_opaque(){
 let mut doc=Document::new(2,1);doc.resolution=300.;let mut red=Layer::with_pixels(" Red ",Raster::from_straight(2,1,&[80,80,80,255,200,200,200,128]),Point{x:0.,y:0.});red.visible=false;doc.layers.push(red);
 let before=doc.manifest().to_json_pretty().unwrap();let o=options();let cache=EffectsCache::default();let mut o= o;o.channel_sources=Some([Source::Layer(doc.layers[0].id),Source::White,Source::Black,Source::Black]);
 let(bytes,info)=raw_png(&channel_pack::export(&doc,&cache,&o,0).unwrap());assert_eq!(bytes,[80,255,0,0,100,255,0,0]);assert_eq!(info.pixel_dims.unwrap().xppu,11811);assert!(info.srgb.is_some());
 let(preview,_)=raw_png(&channel_pack::export(&doc,&cache,&o,640).unwrap());assert_eq!(preview,[80,255,0,255,100,255,0,255]);assert_eq!(doc.manifest().to_json_pretty().unwrap(),before);
 assert_eq!(channel_pack::defaults(&doc),[Source::Layer(doc.layers[0].id),Source::Black,Source::Black,Source::White]);
}
#[test]fn isolated_hidden_layer_keeps_its_group_opacity_and_mask_and_rejects_missing_source(){
 let mut doc=Document::new(2,1);let mut group=Layer::blank("Folder",doc.size());group.is_group=true;group.visible=false;group.opacity=0.5;let parent=group.id;
 let mut layer=Layer::with_pixels("Map",Raster::from_straight(2,1,&[255;8]),Point{x:0.,y:0.});layer.parent_id=Some(parent);layer.visible=false;
 let id=layer.id;doc.layers=vec![layer,group];
 let mut o=options();o.channel_sources=Some([Source::Layer(id),Source::Layer(id),Source::Black,Source::White]);
 let(bytes,_)=raw_png(&channel_pack::export(&doc,&EffectsCache::default(),&o,0).unwrap());assert_eq!(bytes,[128,128,0,255,128,128,0,255]);
 o.channel_sources=Some([Source::Layer(uuid::Uuid::new_v4()),Source::Black,Source::Black,Source::White]);assert!(channel_pack::export(&doc,&EffectsCache::default(),&o,0).is_err());
}
