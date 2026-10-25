use compositor_engine::cmyk;
fn profile()->Vec<u8>{
    // Structural ICC fixture only; tests container embedding, not a color transform.
    let mut p=vec![0;152];p[..4].copy_from_slice(&152u32.to_be_bytes());p[12..16].copy_from_slice(b"prtr");p[16..20].copy_from_slice(b"CMYK");p[20..24].copy_from_slice(b"Lab ");p[36..40].copy_from_slice(b"acsp");p[128..132].copy_from_slice(&1u32.to_be_bytes());p[132..136].copy_from_slice(b"desc");p[136..140].copy_from_slice(&144u32.to_be_bytes());p[140..144].copy_from_slice(&8u32.to_be_bytes());p[144..148].copy_from_slice(b"desc");p
}
#[test]fn profiles_reject_wrong_colorspace_class_and_invalid_tag_ranges(){
    let p=profile();cmyk::validate_profile(&p).unwrap();
    for (offset,value) in [(16,b"RGB "),(12,b"mntr"),(36,b"xxxx"),(136,b"\xff\xff\xff\xff")]{let mut bad=p.clone();bad[offset..offset+4].copy_from_slice(value);assert!(cmyk::validate_profile(&bad).is_err());}
    assert!(cmyk::validate_profile(&p[..150]).is_err());
}
#[test]fn tiff_preserves_ink_channels_profile_and_print_resolution(){
    use tiff::{decoder::{Decoder,DecodingResult},tags::Tag};
    let p=profile();let pixels=[0,0,0,0,255,0,0,0,0,255,0,0,0,0,255,0,0,0,0,255];
    let bytes=cmyk::encode(&pixels,5,1,300.,"TIFF",1.,&p).unwrap();
    let mut decoder=Decoder::new(std::io::Cursor::new(&bytes)).unwrap();assert_eq!(decoder.colortype().unwrap(),tiff::ColorType::CMYK(8));assert_eq!(decoder.get_tag_u8_vec(Tag::IccProfile).unwrap(),p);
    assert_eq!(decoder.get_tag(Tag::XResolution).unwrap(),tiff::decoder::ifd::Value::Rational(300000,1000));
    let DecodingResult::U8(result)=decoder.read_image().unwrap()else{panic!()};assert_eq!(result,pixels);
}
#[test]fn jpeg_and_pdf_embed_four_channel_profile_and_valid_lengths(){
    let p=profile();let pixels=[0,64,128,192].repeat(64);
    let jpg=cmyk::encode(&pixels,8,8,300.,"JPEG",1.,&p).unwrap();assert!(jpg.windows(12).any(|s|s==b"ICC_PROFILE\0"));assert!(jpg.windows(p.len()).any(|s|s==p));
    // The independent JPEG decoder must recognize a four-channel Adobe JPEG.
    let decoded=image::load_from_memory_with_format(&jpg,image::ImageFormat::Jpeg).unwrap();assert_eq!((decoded.width(),decoded.height()),(8,8));
    let pdf=cmyk::encode(&pixels,8,8,300.,"PDF",1.,&p).unwrap();let text=String::from_utf8_lossy(&pdf);
    assert!(text.contains("/ColorSpace [/ICCBased 6 0 R]"));assert!(text.contains("/N 4 /Alternate /DeviceCMYK /Length 152"));assert!(text.contains("/MediaBox [0 0 1.92000000 1.92000000]"));assert!(pdf.windows(pixels.len()).any(|s|s==pixels));
    let xref=text.rsplit("startxref\n").next().unwrap().lines().next().unwrap().parse::<usize>().unwrap();assert_eq!(&pdf[xref..xref+4],b"xref");
    if let Ok(path)=std::env::var("CMYK_TEST_OUTPUT"){std::fs::create_dir_all(&path).unwrap();std::fs::write(format!("{path}/channels.jpg"),jpg).unwrap();std::fs::write(format!("{path}/channels.pdf"),pdf).unwrap();}
}
#[test]fn output_rejects_invalid_buffers_formats_and_resolution(){let p=profile();for (w,h,dpi,f) in [(0,1,72.,"PDF"),(1,1,f64::NAN,"PDF"),(1,1,72.,"PNG"),(30001,1,72.,"JPEG")]{assert!(cmyk::encode(&[0;4],w,h,dpi,f,1.,&p).is_err());}assert!(cmyk::encode(&[0;3],1,1,72.,"TIFF",1.,&p).is_err());}
