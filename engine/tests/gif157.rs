use compositor_engine::{*, animation::*};
use std::borrow::Cow;

fn fixture() -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = gif::Encoder::new(&mut bytes, 3, 2, &[0,0,0, 255,0,0, 0,255,0, 0,0,255]).unwrap();
        for (left, top, width, height, pixels, dispose, delay) in [
            (0,0,3,2,vec![1,1,1,1,1,1],gif::DisposalMethod::Keep,10),
            (1,0,1,1,vec![2],gif::DisposalMethod::Previous,1),
            (0,1,1,1,vec![3],gif::DisposalMethod::Background,20),
            (2,0,1,2,vec![0,2],gif::DisposalMethod::Keep,0),
        ] {
            encoder.write_frame(&gif::Frame { left, top, width, height, dispose, delay, transparent: Some(0), buffer: Cow::Owned(pixels), ..Default::default() }).unwrap();
        }
    }
    bytes
}

#[test]
fn composes_partial_frames_with_previous_background_and_transparency() {
    let source = GifSource::decode(&fixture()).unwrap();
    assert_eq!(source.frames.iter().map(|f|f.duration).collect::<Vec<_>>(), vec![0.1,0.02,0.2,0.1]);
    let red = [255,0,0,255]; let green=[0,255,0,255]; let blue=[0,0,255,255];
    for _ in 0..2 {
        assert!(source.frame(0).unwrap().bytes().chunks_exact(4).all(|p|p==red));
        assert_eq!(source.frame(1).unwrap().pixel(1,0),green);
        let p=source.frame(2).unwrap(); assert_eq!(p.pixel(1,0),red); assert_eq!(p.pixel(0,1),blue);
        let p=source.frame(3).unwrap(); assert_eq!(p.pixel(0,1),[0;4]); assert_eq!(p.pixel(2,0),red); assert_eq!(p.pixel(2,1),green);
        assert!(source.frame(3).unwrap().same_pixels(&p));
    }
    assert!(source.frame(4).is_err());
}

#[test]
fn stable_identities_preserve_edits_and_timing_across_duplicate_move_delete() {
    let source=GifSource::decode(&fixture()).unwrap();
    let mut a=GifAnimation::new(source.clone());
    let painted=Raster::from_straight(3,2,&[77;24]);
    a.frame_pixels.insert(1,painted.clone()); a.timing.set(1,0.3);
    assert!(a.duplicate(1)); assert_eq!(a.selected_frame_index,4);
    assert!(a.frame(4).unwrap().same_pixels(&painted)); assert_eq!(a.duration(a.references()[2]),0.3);
    assert!(a.move_frame(4,0)); assert!(a.delete(1)); assert!(a.valid());
    assert_eq!(a.references().iter().map(|f|f.index).collect::<Vec<_>>(),vec![4,0,2,3]);
    assert!(std::sync::Arc::ptr_eq(&a.source,&source));
    for id in [0,2,3] { assert!(a.delete(id)); }
    assert!(!a.delete(4)); assert!(a.valid());
    let mut invalid=a.clone(); invalid.frame_order.as_mut().unwrap()[0].index=8192; assert!(!invalid.valid());
    invalid=a.clone(); invalid.timing.frame_delays.push(FrameDelay{index:4,duration:0.3}); assert!(!invalid.valid());
}

#[test]
fn navigation_does_not_dirty_or_clear_redo_and_edits_roundtrip_v17() {
    let mut engine=Engine::new();let bytes=fixture();
    let id=engine.import_image(None,&bytes,"Animation",None).unwrap();
    let layer=engine.state(id).unwrap().active_layer_id.unwrap();
    engine.mark_saved(id,None);
    let initial_revision=engine.state(id).unwrap().layers[0].pixels_revision;
    engine.execute(id,Command::SelectGifFrame{id:layer,index:1}).unwrap();
    assert_ne!(engine.state(id).unwrap().layers[0].pixels_revision,initial_revision);
    assert_eq!(engine.pixels_delta(id,layer,initial_revision).unwrap(),None,"frame navigation must invalidate the uploaded texture");
    assert!(!engine.state(id).unwrap().is_modified);assert_eq!(engine.state(id).unwrap().undo_depth,0);
    engine.execute(id,Command::InvertPixels{id:layer,mask:false}).unwrap();
    let edited=engine.document(id).unwrap().layer(layer).unwrap().pixels.clone().unwrap();
    engine.undo(id).unwrap();
    engine.execute(id,Command::SelectGifFrame{id:layer,index:3}).unwrap();assert!(engine.state(id).unwrap().can_redo);
    engine.redo(id).unwrap(); assert_eq!(engine.document(id).unwrap().layer(layer).unwrap().pixels,Some(edited.clone()));
    engine.execute(id,Command::EditGif{id:layer,edit:GifEdit::Duplicate{index:1}}).unwrap();
    let saved=engine.save_package(id).unwrap();let manifest=Manifest::parse(&saved.manifest_json).unwrap();assert_eq!(manifest.version,17);
    assert_eq!(saved.images.iter().find(|(name,_)|name.ends_with(".gif")).unwrap().1,bytes);
    let opened=engine.open_package(&saved,None).unwrap();
    assert_eq!(engine.state(opened).unwrap().layers[0].gif,engine.state(id).unwrap().layers[0].gif);
    assert_eq!(engine.document(opened).unwrap().layers[0].pixels,Some(edited));
    let mut bad=saved.clone();bad.manifest_json=bad.manifest_json.replace("\"version\": 17","\"version\": 16");assert!(engine.open_package(&bad,None).is_err());
    let mut bad=saved;let mut value:serde_json::Value=serde_json::from_str(&bad.manifest_json).unwrap();value["layers"][0]["gif"]["sourceFile"]="../outside.gif".into();bad.manifest_json=value.to_string();assert!(engine.open_package(&bad,None).is_err());
}

#[test]
fn geometry_preserves_transparent_outside_and_source_stays_immutable() {
    let source=GifSource::decode(&fixture()).unwrap();let mut a=GifAnimation::new(source);
    a.geometry=Some(FrameGeometry::new(6,2,Affine::scale(0.5,1.0),Sampling::Nearest));assert!(a.valid());
    let p=a.frame(0).unwrap();assert_eq!(p.pixel(2,0),[255,0,0,255]);assert_eq!(p.pixel(3,0),[0;4]);
    assert_eq!(a.source.width,3);
}

#[test]
fn export_merges_clocks_holds_short_tracks_and_preserves_final_frames() {
    use animation::export::{schedule,ScheduledFrame};
    assert_eq!(schedule(&[vec![3,3],vec![2,2,2]]).unwrap(),vec![
        ScheduledFrame{start:0,sample:0,ticks:2},ScheduledFrame{start:2,sample:2,ticks:2},ScheduledFrame{start:4,sample:4,ticks:2}]);
    assert_eq!(schedule(&[vec![3,2],vec![2,3]]).unwrap(),vec![ScheduledFrame{start:0,sample:0,ticks:2},ScheduledFrame{start:2,sample:3,ticks:3}]);
    assert_eq!(schedule(&[vec![65535,2]]).unwrap().iter().map(|f|f.ticks as u64).sum::<u64>(),65537);
    assert!(schedule(&[vec![1]]).is_err());
    let mut engine=Engine::new();let doc=engine.import_image(None,&fixture(),"GIF",None).unwrap();
    let options=export_as::ExportOptions{width:3,height:2,format:export_as::ExportFormat::GIF,quality:1.,background:None,channel_sources:None,gif_loop:Some(false)};
    let bytes=engine.export_as(doc,&options,0).unwrap();
    use image::AnimationDecoder;
    let decoded=image::codecs::gif::GifDecoder::new(std::io::Cursor::new(&bytes)).unwrap().into_frames().collect_frames().unwrap();
    assert_eq!(decoded.len(),4);assert_eq!(decoded[1].delay().numer_denom_ms(),(20,1));
    let expected=GifSource::decode(&fixture()).unwrap();
    for (i,frame) in decoded.iter().enumerate() { assert_eq!(frame.buffer().as_raw(),&expected.frame(i).unwrap().to_straight()); }
    assert_eq!(engine.state(doc).unwrap().undo_depth,0);
    let mut big=options.clone();big.width=30000;big.height=3000;assert!(engine.export_as(doc,&big,0).is_err());
}

#[test]
fn resized_animation_keeps_all_frames_and_edited_grid_on_reopen() {
    let mut engine=Engine::new();let doc=engine.import_image(None,&fixture(),"GIF",None).unwrap();
    let id=engine.state(doc).unwrap().active_layer_id.unwrap();
    engine.execute(doc,Command::SelectGifFrame{id,index:2}).unwrap();
    engine.execute(doc,Command::InvertPixels{id,mask:false}).unwrap();
    engine.execute(doc,Command::ImageSize{width:6,height:4,resolution:72.,sampling:Sampling::Nearest}).unwrap();
    let layer=&engine.document(doc).unwrap().layers[0];assert_eq!(layer.extra.gif.as_ref().unwrap().dimensions(),(6,4));
    engine.execute(doc,Command::SelectGifFrame{id,index:3}).unwrap();
    let saved=engine.save_package(doc).unwrap();let opened=engine.open_package(&saved,None).unwrap();
    assert_eq!(engine.document(opened).unwrap().layers[0].pixels,engine.document(doc).unwrap().layers[0].pixels);
    engine.undo(doc).unwrap();assert_eq!(engine.document(doc).unwrap().layers[0].extra.gif.as_ref().unwrap().dimensions(),(3,2));
}

#[test]
fn whole_layer_clipboard_and_history_share_source_but_keep_frame_edits_independent() {
    let mut engine=Engine::new();let doc=engine.import_image(None,&fixture(),"GIF",None).unwrap();
    let id=engine.state(doc).unwrap().active_layer_id.unwrap();
    assert!(engine.capture_gif_clipboard(doc,id,"test-token".into()).unwrap());
    let next=engine.new_document(3,2,false).unwrap();engine.paste_gif_clipboard(next,"test-token").unwrap();
    let pasted=engine.state(next).unwrap().active_layer_id.unwrap();
    assert!(std::sync::Arc::ptr_eq(&engine.document(doc).unwrap().layers[0].extra.gif.as_ref().unwrap().source,&engine.document(next).unwrap().layers[0].extra.gif.as_ref().unwrap().source));
    engine.execute(next,Command::InvertPixels{id:pasted,mask:false}).unwrap();
    assert!(engine.document(doc).unwrap().layers[0].extra.gif.as_ref().unwrap().frame_pixels.is_empty());
    assert_eq!(engine.document(next).unwrap().layers[0].extra.gif.as_ref().unwrap().frame_pixels.len(),1);
    assert!(engine.paste_gif_clipboard(next,"expired-token").is_err());
    let before=engine.document(next).unwrap().clone();let mut history=History::with_limits(100,usize::MAX);history.push(before.clone());
    let mut empty=before;empty.layers.clear();
    assert!(history.retained_bytes(&empty)>fixture().len(),"source bytes, thumbnails and edited pixels all count");
}

#[test]
fn filter_growth_is_cropped_back_to_the_animation_grid_and_is_undoable() {
    let mut engine=Engine::new();let doc=engine.new_document(20,20,false).unwrap();engine.import_image(Some(doc),&fixture(),"GIF",Some(Point{x:10.,y:10.})).unwrap();
    let mut seeded=engine.document(doc).unwrap().clone();seeded.layers[0].mask=Some(Mask{pixels:GrayRaster::from_bytes(3,2,vec![0,64,128,192,230,255]),enabled:true,placement:None,linked:None});
    let doc=engine.insert_document(seeded);
    let id=engine.state(doc).unwrap().active_layer_id.unwrap();let original=engine.document(doc).unwrap().layers[0].clone();
    assert!(engine.execute(doc,Command::ConvertPixelDepth{id,depth:PixelDepth::Sixteen}).is_err());
    engine.execute(doc,Command::ApplyFilter{id,params:FilterParams::GaussianBlur{radius:2.,preserve_transparency:false}}).unwrap();
    let layer=&engine.document(doc).unwrap().layers[0];assert_eq!(layer.transform,original.transform);assert_eq!((layer.pixels.as_ref().unwrap().width,layer.pixels.as_ref().unwrap().height),(3,2));
    assert_eq!(layer.mask,original.mask);
    assert_eq!(layer.extra.gif.as_ref().unwrap().frame_pixels.len(),1);
    engine.execute(doc,Command::SelectGifFrame{id,index:1}).unwrap();assert_eq!(engine.document(doc).unwrap().layers[0].transform,original.transform);
    engine.undo(doc).unwrap();assert!(engine.document(doc).unwrap().layers[0].extra.gif.as_ref().unwrap().frame_pixels.is_empty());
}
