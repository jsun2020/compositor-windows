//! CMYK containers. Conversion is provided by the native ICC color manager.
use crate::{CommandError, MAX_PIXELS};
const MAX_PROFILE: usize = 16 * 1024 * 1024;

pub fn validate_profile(data: &[u8]) -> Result<(), String> {
    let fail = "Choose a valid CMYK output ICC profile under 16 MiB";
    if data.len() < 132 || data.len() > MAX_PROFILE { return Err(fail.into()); }
    let n = u32::from_be_bytes(data[..4].try_into().unwrap()) as usize;
    if n != data.len() || &data[36..40] != b"acsp" || &data[12..16] != b"prtr" || &data[16..20] != b"CMYK" || (&data[20..24] != b"XYZ " && &data[20..24] != b"Lab ") { return Err(fail.into()); }
    let tags = u32::from_be_bytes(data[128..132].try_into().unwrap()) as usize;
    if tags == 0 || tags > (n-132)/12 { return Err(fail.into()); }
    for tag in data[132..132+tags*12].chunks_exact(12) {
        let start = u32::from_be_bytes(tag[4..8].try_into().unwrap()) as usize;
        let size = u32::from_be_bytes(tag[8..12].try_into().unwrap()) as usize;
        if start < 132+tags*12 || size < 8 || start.checked_add(size).is_none_or(|end|end>n) { return Err(fail.into()); }
    }
    Ok(())
}

pub fn encode(pixels: &[u8], width: u32, height: u32, dpi: f64, format: &str, quality: f64, icc: &[u8]) -> Result<Vec<u8>, CommandError> {
    let fail = |e: String| CommandError::Argument(e);
    validate_profile(icc).map_err(fail)?;
    if width==0 || height==0 || width>30000 || height>30000 || width as u64*height as u64>MAX_PIXELS || pixels.len()!=width as usize*height as usize*4 || !dpi.is_finite() || !(1.0..=9600.).contains(&dpi) || !quality.is_finite() || !(0.0..=1.0).contains(&quality) { return Err(fail("Invalid CMYK export dimensions or settings".into())); }
    match format {
        "JPEG" => {
            let mut out=Vec::new(); let mut encoder=jpeg_encoder::Encoder::new(&mut out,(quality*100.).round().clamp(1.,100.) as u8);
            encoder.set_density(jpeg_encoder::Density::Inch{x:dpi.round() as u16,y:dpi.round() as u16});
            encoder.add_icc_profile(icc).map_err(|e|fail(e.to_string()))?;
            encoder.encode(pixels,width as u16,height as u16,jpeg_encoder::ColorType::Cmyk).map_err(|e|fail(e.to_string()))?; Ok(out)
        }
        "TIFF" => {
            use tiff::{encoder::{TiffEncoder,colortype,Rational},tags::{Tag,ResolutionUnit}};
            let mut out=std::io::Cursor::new(Vec::new());
            let mut encoder=TiffEncoder::new(&mut out).map_err(|e|fail(e.to_string()))?;
            let mut image=encoder.new_image::<colortype::CMYK8>(width,height).map_err(|e|fail(e.to_string()))?;
            image.resolution(ResolutionUnit::Inch,Rational{n:(dpi*1000.).round() as u32,d:1000});
            image.encoder().write_tag(Tag::IccProfile,icc).map_err(|e|fail(e.to_string()))?;
            image.write_data(pixels).map_err(|e|fail(e.to_string()))?; Ok(out.into_inner())
        }
        "PDF" => pdf(pixels,width,height,dpi,icc),
        _ => Err(fail("CMYK export supports JPEG, TIFF and PDF".into()))
    }
}

fn pdf(pixels:&[u8],w:u32,h:u32,dpi:f64,icc:&[u8])->Result<Vec<u8>,CommandError>{
    let (pw,ph)=(w as f64/dpi*72.,h as f64/dpi*72.);
    let mut out=Vec::new();out.try_reserve_exact(pixels.len()+icc.len()+4096).map_err(|_|CommandError::Argument("Not enough memory for CMYK PDF".into()))?;
    out.extend(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");let mut offsets=vec![0];
    fn object(out:&mut Vec<u8>,offsets:&mut Vec<usize>,body:&str,data:Option<&[u8]>){
        offsets.push(out.len());out.extend(format!("{} 0 obj\n{body}",offsets.len()-1).as_bytes());
        if let Some(data)=data{out.extend(b"\nstream\n");out.extend(data);out.extend(b"\nendstream");}out.extend(b"\nendobj\n");
    }
    object(&mut out,&mut offsets,"<< /Type /Catalog /Pages 2 0 R >>",None);
    object(&mut out,&mut offsets,"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",None);
    object(&mut out,&mut offsets,&format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {pw:.8} {ph:.8}] /Resources << /XObject << /Im0 5 0 R >> >> /Contents 4 0 R >>"),None);
    let content=format!("q\n{pw:.8} 0 0 {ph:.8} 0 0 cm\n/Im0 Do\nQ\n");
    object(&mut out,&mut offsets,&format!("<< /Length {} >>",content.len()),Some(content.as_bytes()));
    object(&mut out,&mut offsets,&format!("<< /Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace [/ICCBased 6 0 R] /BitsPerComponent 8 /Length {} >>",pixels.len()),Some(pixels));
    object(&mut out,&mut offsets,&format!("<< /N 4 /Alternate /DeviceCMYK /Length {} >>",icc.len()),Some(icc));
    let start=out.len();out.extend(format!("xref\n0 {}\n0000000000 65535 f \n",offsets.len()).as_bytes());for offset in &offsets[1..]{out.extend(format!("{offset:010} 00000 n \n").as_bytes());}
    out.extend(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{start}\n%%EOF\n",offsets.len()).as_bytes());Ok(out)
}

/// Apply only source metadata orientation after a native ICC decode.
pub fn oriented_rgba(bytes:&[u8],width:u32,height:u32,rgba:Vec<u8>)->Result<crate::Raster,String>{
    use image::ImageDecoder;
    let mut decoder=image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format().map_err(|e|e.to_string())?.into_decoder().map_err(|e|e.to_string())?;
    let orientation=decoder.orientation().map_err(|e|e.to_string())?;
    let image=image::RgbaImage::from_raw(width,height,rgba).ok_or("Invalid native pixel dimensions")?;
    let mut image=image::DynamicImage::ImageRgba8(image);image.apply_orientation(orientation);let image=image.into_rgba8();
    Ok(crate::Raster::from_straight(image.width(),image.height(),image.as_raw()))
}
