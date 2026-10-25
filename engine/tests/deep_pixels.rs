use compositor_engine::*;
#[test]
fn sixteen_bit_png_preserves_distinctions_hidden_by_eight_bit_preview() {
    let source=Raster::from_rgba16(3,1,vec![30000,31000,32000,65535,30001,31001,32001,65535,30002,31002,32002,65535]);
    assert_eq!(source.depth(),PixelDepth::Sixteen);
    let encoded=encode_deep_png(&source,300.).unwrap();
    let decoded=decode_image(&encoded).unwrap().raster;
    assert_eq!(decoded.deep_pixels(),source.deep_pixels());
    assert!(decode_package_image(&encoded,"test.png",11).is_err());
    assert_eq!(decode_package_image(&encoded,"test.png",12).unwrap(),source);
    assert!(package_image_size(&encoded,"test.mask.png",12).is_err());
    assert_eq!(source.storage_bytes(),36);
}
#[test]
fn hdr_exr_retains_highlights_and_exposure_recovers_them() {
    let source=Raster::from_rgba_float(2,1,vec![2.,3.,4.,1.,0.2,0.3,0.4,1.]);
    assert_eq!(source.depth(),PixelDepth::Hdr);
    let bytes=encode_exr(&source).unwrap();let decoded=decode_image(&bytes).unwrap().raster;
    assert_eq!(decoded.depth(),PixelDepth::Hdr);
    let actual=decoded.deep_pixels().unwrap().straight_f32();
    for (got,want) in actual.iter().zip(source.deep_pixels().unwrap().straight_f32()) {assert!((got-want).abs()<0.005);}
    let reduced=decoded.hdr_exposure(-5.,0.,1.);
    assert!(reduced.deep_pixels().unwrap().straight_f32()[0]<1.);
    assert!(reduced.pixel(0,0)[0]<255);
    assert!(decode_package_image(&bytes,"test.exr",12).is_err());
    assert!(decode_package_image(&bytes,"test.exr",13).is_ok());
}
#[test]
fn paint_keeps_untouched_deep_values_and_depth_conversion_is_explicit() {
    let original=Raster::from_rgba16(2,1,vec![12345,23456,34567,65535,45678,12345,23456,65535]);
    let mut bytes=original.bytes().to_vec();bytes[..4].copy_from_slice(&[240,20,40,255]);
    let painted=original.with_painted(&Raster::from_premultiplied(2,1,bytes));
    assert_eq!(&painted.deep_pixels().unwrap().samples[4..],&original.deep_pixels().unwrap().samples[4..]);
    assert_eq!(painted.deep_pixels().unwrap().samples[0],240*257);
    assert_eq!(painted.to_depth(PixelDepth::Eight).depth(),PixelDepth::Eight);
    assert_eq!(original.to_depth(PixelDepth::Sixteen),original);
}
#[test]
fn reconstructed_inversion_keeps_sub_byte_detail() {
    let source=Raster::from_rgba16(2,1,vec![10000,20000,30000,65535,10020,20020,30020,65535]);
    let output=source.map_deep::<()>(false,|r|Ok(adjust::tonal::invert_raster(r))).unwrap();
    for (a,b) in source.deep_pixels().unwrap().samples.iter().zip(&output.deep_pixels().unwrap().samples).enumerate() {
        if a%4==3 {assert_eq!(*b.1,65535);}else{assert!(((*b.0 as i32+*b.1 as i32)-65535).abs()<=1);}
    }
}

#[test]
fn package_worker_and_undo_retain_sixteen_bit_samples() {
    let source=Raster::from_rgba16(3,1,vec![10000,20000,30000,65535,10001,20001,30001,65535,10002,20002,30002,65535]);
    let mut engine=Engine::new();
    let id=engine.new_document(3,1,false).unwrap();
    engine.import_image(Some(id),&encode_deep_png(&source,72.).unwrap(),"Deep",None).unwrap();
    let layer=engine.document(id).unwrap().layers[0].id;
    let (input,pixels,mask,points)=engine.job_input(id,layer).unwrap();
    let input:JobInput=serde_json::from_str(&serde_json::to_string(&input).unwrap()).unwrap();
    assert_eq!(input.pixel_depth,PixelDepth::Sixteen);
    assert_eq!(pixels.as_ref().unwrap().transfer_len(),24);
    let command=Command::InvertPixels{id:layer,mask:false};
    let (output,new_pixels,new_mask,_)=run_edit_job(&input,pixels,mask,points.as_deref(),command,0.).unwrap();
    assert_eq!(output.pixel_depth,PixelDepth::Sixteen);
    let edited=new_pixels.clone().unwrap();
    let mut wrong=output.clone();wrong.pixel_depth=PixelDepth::Eight;
    assert!(engine.install_job(id,layer,input.stamp.clone(),wrong,new_pixels.clone(),new_mask.clone(),None).is_err());
    engine.install_job(id,layer,input.stamp,output,new_pixels,new_mask,None).unwrap();
    let package=save_package(engine.document(id).unwrap()).unwrap();
    assert_eq!(Manifest::parse(&package.manifest_json).unwrap().version,12);
    assert_eq!(open_package(&package).unwrap().layers[0].pixels.as_ref().unwrap(),&edited);
    engine.undo(id).unwrap();
    assert_eq!(engine.document(id).unwrap().layers[0].pixels.as_ref().unwrap(),&source);
    engine.redo(id).unwrap();
    assert_eq!(engine.document(id).unwrap().layers[0].pixels.as_ref().unwrap(),&edited);
}

#[test]
fn brush_growth_keeps_unpainted_deep_samples_and_undo_restores_grid() {
    let source=Raster::from_rgba16(2,2,[12345,23456,34567,65535].repeat(4));
    let mut engine=Engine::new();let id=engine.new_document(12,12,false).unwrap();
    engine.import_image(Some(id),&encode_deep_png(&source,72.).unwrap(),"Deep",Some(Point{x:6.,y:6.})).unwrap();
    let layer=engine.document(id).unwrap().layers[0].id;
    let original=engine.document(id).unwrap().layers[0].clone();
    engine.execute(id,Command::BrushStroke{id:layer,mask:false,brush:BrushSpec{ pressures:vec![],pressure_size:false,pressure_opacity:false,diameter:2.,hardness:1.,opacity:1.,points:vec![Point{x:1.,y:1.}],color:[1.,0.,0.,1.],erasing:false,operation:BrushOperation::Paint}}).unwrap();
    let painted=engine.document(id).unwrap().layers[0].pixels.as_ref().unwrap();
    assert_eq!(painted.depth(),PixelDepth::Sixteen);
    assert!(painted.width>2);
    assert_eq!(painted.deep_pixels().unwrap().samples.chunks_exact(4).filter(|p|*p==[12345,23456,34567,65535]).count(),4);
    engine.undo(id).unwrap();
    assert_eq!(engine.document(id).unwrap().layers[0].pixels,original.pixels);
    assert_eq!(engine.document(id).unwrap().layers[0].transform,original.transform);
}

#[test]
fn high_precision_composite_preserves_values_opacity_and_hdr_highlights() {
    let mut doc=Document::new(2,1);
    let source=Raster::from_rgba16(2,1,vec![20000,30000,40000,65535,20001,30001,40001,65535]);
    let mut layer=Layer::blank("deep",doc.size());layer.set_pixels(Some(source.clone()));doc.layers.push(layer);
    let result=compositor::render_full(&doc).unwrap();
    assert_eq!(result.deep_pixels(),source.deep_pixels());
    let mut top=Layer::blank("HDR",doc.size());top.set_pixels(Some(Raster::from_rgba_float(2,1,[4.,3.,2.,1.].repeat(2))));top.opacity=0.5;doc.layers.push(top);
    let result=compositor::render_full(&doc).unwrap();
    assert_eq!(result.depth(),PixelDepth::Hdr);
    let p=result.pixel_f32(0,0);assert!((p[0]-(2.+20000./65535.*0.5)).abs()<0.002);assert_eq!(p[3],1.);
}

#[test]
fn spatial_filters_and_image_resampling_keep_high_precision() {
    let source=Raster::from_rgba16(8,8,[20001,30002,40003,65535].repeat(64));
    for result in [gaussian_blur(&source,0.1),motion_blur(&source,0.,0.1)] {
        assert_eq!(result.depth(),PixelDepth::Sixteen);
        assert_eq!(&result.deep_pixels().unwrap().samples[(4*8+4)*4..][..4],&[20001,30002,40003,65535]);
    }
    let mut doc=Document::new(8,8);let mut layer=Layer::blank("deep",doc.size());layer.set_pixels(Some(source));doc.layers.push(layer);
    let resized=ops::image_size::image_size(&doc,ops::image_size::ImageSizeOptions{width:4,height:4,resolution:72.,sampling:Sampling::Nearest}).unwrap();
    let p=resized.layers[0].pixels.as_ref().unwrap();assert_eq!(p.depth(),PixelDepth::Sixteen);
    assert!(p.deep_pixels().unwrap().samples.chunks_exact(4).all(|p|p==[20001,30002,40003,65535]));
}

#[test]
fn export_formats_preserve_precision_resolution_and_preview_contract() {
    use export_as::{ExportFormat,ExportOptions};
    let source=Raster::from_rgba16(2,1,vec![20000,30000,40000,65535,20001,30001,40001,65535]);
    for format in [ExportFormat::PNG16,ExportFormat::TIFF,ExportFormat::EXR] {
        let o=ExportOptions{gif_loop:None,channel_sources:None,width:2,height:1,format,quality:1.,background:None};
        let bytes=export_as::encode(source.clone(),300.,&o,false).unwrap();
        let decoded=decode_image(&bytes).unwrap().raster;
        assert_ne!(decoded.depth(),PixelDepth::Eight);
        if format!=ExportFormat::EXR {assert_eq!(decoded.deep_pixels(),source.deep_pixels());}
        if format==ExportFormat::TIFF {
            let mut reader=tiff::decoder::Decoder::new(std::io::Cursor::new(bytes)).unwrap();
            assert_eq!(reader.get_tag_u32(tiff::tags::Tag::ResolutionUnit).unwrap(),2);
            assert_eq!(reader.get_tag(tiff::tags::Tag::XResolution).unwrap(),tiff::decoder::ifd::Value::Rational(300000,1000));
        }
        let preview=export_as::encode(source.clone(),300.,&o,true).unwrap();
        assert!(preview.starts_with(b"\x89PNG"));
        assert_eq!(decode_image(&preview).unwrap().raster.depth(),PixelDepth::Eight);
    }
}
