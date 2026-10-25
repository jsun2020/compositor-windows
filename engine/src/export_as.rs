//! Output sizing and lossless PDF images; this never changes a document/history.
use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ExportFormat {
    GIF,
    PNG,
    ChannelPack,
    PNG16,
    EXR,
    TIFF,
    JPEG,
    PDF,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportOptions {
    pub width: u32,
    pub height: u32,
    pub format: ExportFormat,
    pub quality: f64,
    pub background: Option<[f64; 3]>,
    #[serde(default)] pub channel_sources: Option<[crate::channel_pack::ChannelSource;4]>,
    #[serde(default)] pub gif_loop: Option<bool>,
}
impl ExportOptions {
    pub fn validate(&self) -> Result<(), CommandError> {
        if self.width == 0
            || self.height == 0
            || self.width > 30000
            || self.height > 30000
            || self.width as u64 * self.height as u64 > MAX_PIXELS
            || !self.quality.is_finite()
            || !(0.0..=1.0).contains(&self.quality)
            || self
                .background
                .is_some_and(|c| c.iter().any(|v| !v.is_finite() || !(0.0..=1.0).contains(v)))
        {
            return Err(CommandError::Argument(
                "invalid export dimensions, quality or background".into(),
            ));
        }
        Ok(())
    }
}
fn resized(source: Raster, w: u32, h: u32) -> Raster {
    if source.width == w && source.height == h {
        return source;
    }
    let mut layer = Layer::with_pixels("Export", source, Point { x: 0., y: 0. });
    layer.transform.size = Size {
        width: w as f64,
        height: h as f64,
    };
    if layer.pixels.as_ref().is_some_and(|p|p.deep_pixels().is_some()) {
        let mut doc=Document::new(w,h);doc.layers.push(layer);
        return compositor::composite(&doc,Rect{x:0.,y:0.,width:w as f64,height:h as f64},w,h);
    }
    let mut bytes = vec![0u8; w as usize * h as usize * 4];
    compositor::render_layer(
        &mut bytes,
        w,
        h,
        Rect {
            x: 0.,
            y: 0.,
            width: w as f64,
            height: h as f64,
        },
        &layer,
    );
    Raster::from_premultiplied(w, h, bytes)
}
pub fn render(
    doc: &Document,
    cache: &EffectsCache,
    o: &ExportOptions,
    preview: u32,
) -> Result<Raster, CommandError> {
    o.validate()?;
    if preview > 0 {
        let scale = (preview as f64 / o.width.max(o.height) as f64).min(1.);
        return Ok(compositor::composite_edit_with(
            doc,
            None,
            Rect {
                x: 0.,
                y: 0.,
                width: doc.width as f64,
                height: doc.height as f64,
            },
            (o.width as f64 * scale).round().max(1.) as u32,
            (o.height as f64 * scale).round().max(1.) as u32,
            cache,
        ));
    }
    Ok(resized(
        compositor::render_full_with(doc, cache)?,
        o.width,
        o.height,
    ))
}
pub(crate) fn flatten(source: Raster, bg: [f64; 3]) -> Raster {
    let (width, height) = (source.width, source.height);
    if source.deep_pixels().is_some() {
        let mut values=Vec::with_capacity(width as usize*height as usize*4);
        for y in 0..height {for x in 0..width {let p=source.pixel_f32(x,y);values.extend([p[0]+bg[0] as f32*(1.-p[3]),p[1]+bg[1] as f32*(1.-p[3]),p[2]+bg[2] as f32*(1.-p[3]),1.]);}}
        return Raster::from_premultiplied_f32(width,height,values,source.depth());
    }
    let mut bytes = source.into_bytes();
    for p in bytes.chunks_exact_mut(4) {
        for c in 0..3 {
            p[c] = (p[c] as f64 + bg[c] * (255 - p[3]) as f64)
                .round()
                .clamp(0., 255.) as u8;
        }
        p[3] = 255;
    }
    Raster::from_premultiplied(width, height, bytes)
}
pub fn encode(
    raster: Raster,
    resolution: f64,
    o: &ExportOptions,
    preview: bool,
) -> Result<Vec<u8>, CommandError> {
    if matches!(o.format, ExportFormat::GIF | ExportFormat::ChannelPack) {
        return Err(CommandError::Argument("This export format requires the document's layers".into()));
    }
    if o.format == ExportFormat::JPEG {
        return Ok(encode_jpeg(
            &raster,
            o.quality,
            o.background.unwrap_or([1.; 3]),
            resolution,
        )?);
    }
    let raster = match o.background {
        Some(bg) => flatten(raster, bg),
        None => raster,
    };
    if preview || o.format == ExportFormat::PNG {
        return Ok(encode_png(&raster, resolution)?);
    }
    match o.format {
        ExportFormat::PNG16=>Ok(encode_deep_png(&raster.to_depth(PixelDepth::Sixteen),resolution)?),
        ExportFormat::EXR=>Ok(encode_exr(&raster)?),
        ExportFormat::TIFF=>tiff_image(&raster,resolution),
        _=>pdf(&raster, resolution),
    }
}
// Encode one row at a time: a 100 MP PDF must not allocate a second complete
// straight RGBA raster, an RGB raster and an alpha raster beside the composite.
fn compressed(raster: &Raster, alpha: bool) -> Result<Vec<u8>, CommandError> {
    use std::io::Write;
    let fail = |_| CommandError::Argument("PDF encode failed".into());
    let channels = if alpha { 1 } else { 3 };
    let mut file = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut file, raster.width, raster.height);
        encoder.set_color(if alpha {
            png::ColorType::Grayscale
        } else {
            png::ColorType::Rgb
        });
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(fail)?;
        {
            let mut stream = writer.stream_writer().map_err(fail)?;
            let mut row = vec![0; raster.width as usize * channels];
            for source in raster.bytes().chunks_exact(raster.width as usize * 4) {
                for (pixel, target) in source.chunks_exact(4).zip(row.chunks_exact_mut(channels)) {
                    if alpha {
                        target[0] = pixel[3];
                    } else {
                        let a = pixel[3] as u32;
                        for c in 0..3 {
                            target[c] = if a > 0 && a < 255 {
                                ((pixel[c] as u32 * 255 + a / 2) / a).min(255) as u8
                            } else {
                                pixel[c]
                            };
                        }
                    }
                }
                stream
                    .write_all(&row)
                    .map_err(|_| CommandError::Argument("PDF encode failed".into()))?;
            }
            stream.finish().map_err(fail)?;
        }
        writer.finish().map_err(fail)?;
    }
    // Compact IDAT bytes in place; avoid another full compressed image copy.
    let (mut read, mut write) = (8, 0);
    while read + 12 <= file.len() {
        let n = u32::from_be_bytes(file[read..read + 4].try_into().unwrap()) as usize;
        if &file[read + 4..read + 8] == b"IDAT" {
            file.copy_within(read + 8..read + 8 + n, write);
            write += n;
        }
        read += n + 12;
    }
    file.truncate(write);
    Ok(file)
}
fn pdf(raster: &Raster, dpi: f64) -> Result<Vec<u8>, CommandError> {
    if !dpi.is_finite() || dpi <= 0. {
        return Err(CommandError::Argument("invalid PDF resolution".into()));
    }
    let transparent = raster.bytes().chunks_exact(4).any(|p| p[3] != 255);
    let width = raster.width as f64 / dpi * 72.;
    let height = raster.height as f64 / dpi * 72.;
    let mut out = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = vec![0];
    fn object(out: &mut Vec<u8>, offsets: &mut Vec<usize>, bytes: &[u8]) {
        offsets.push(out.len());
        out.extend(format!("{} 0 obj\n", offsets.len() - 1).bytes());
        out.extend_from_slice(bytes);
        out.extend_from_slice(b"\nendobj\n");
    }
    fn stream(
        out: &mut Vec<u8>,
        offsets: &mut Vec<usize>,
        header: String,
        bytes: Vec<u8>,
    ) -> Result<(), CommandError> {
        out.try_reserve_exact(header.len() + bytes.len() + 96)
            .map_err(|_| CommandError::Argument("PDF output exceeds available memory".into()))?;
        offsets.push(out.len());
        out.extend(
            format!(
                "{} 0 obj\n<< {header} /Length {} >>\nstream\n",
                offsets.len() - 1,
                bytes.len()
            )
            .bytes(),
        );
        out.extend_from_slice(&bytes);
        out.extend_from_slice(b"\nendstream\nendobj\n");
        Ok(())
    }
    object(&mut out, &mut offsets, b"<< /Type /Catalog /Pages 2 0 R >>");
    object(
        &mut out,
        &mut offsets,
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    );
    object(&mut out, &mut offsets, format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {width:.8} {height:.8}] /Resources << /XObject << /Im0 5 0 R >> >> /Contents 4 0 R >>").as_bytes());
    stream(
        &mut out,
        &mut offsets,
        String::new(),
        format!("q\n{width:.8} 0 0 {height:.8} 0 0 cm\n/Im0 Do\nQ\n").into_bytes(),
    )?;
    stream(&mut out, &mut offsets, format!("/Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode /DecodeParms << /Predictor 15 /Colors 3 /BitsPerComponent 8 /Columns {} >> {}", raster.width, raster.height, raster.width, if transparent { "/SMask 6 0 R" } else { "" }), compressed(raster, false)?)?;
    if transparent {
        stream(&mut out, &mut offsets, format!("/Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceGray /BitsPerComponent 8 /Filter /FlateDecode /DecodeParms << /Predictor 15 /Colors 1 /BitsPerComponent 8 /Columns {} >>", raster.width, raster.height, raster.width), compressed(raster, true)?)?;
    }
    let start = out.len();
    out.extend(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len()).bytes());
    for offset in offsets.iter().skip(1) {
        out.extend(format!("{offset:010} 00000 n \n").bytes());
    }
    out.extend(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{start}\n%%EOF\n",
            offsets.len()
        )
        .bytes(),
    );
    Ok(out)
}

fn tiff_image(raster:&Raster,dpi:f64)->Result<Vec<u8>,CommandError> {
    use tiff::encoder::{TiffEncoder,colortype,Rational};
    use tiff::tags::{Tag,ResolutionUnit};
    let mut out=std::io::Cursor::new(Vec::new());
    let fail=|_|CommandError::Argument("TIFF encode failed".into());
    let mut encoder=TiffEncoder::new(&mut out).map_err(fail)?;
    let resolution=Rational{n:(dpi*1000.).round() as u32,d:1000};
    if raster.depth()==PixelDepth::Eight {
        let mut image=encoder.new_image::<colortype::RGBA8>(raster.width,raster.height).map_err(fail)?;
        image.resolution(ResolutionUnit::Inch,resolution);
        image.encoder().write_tag(Tag::ExtraSamples,&[2u16][..]).map_err(fail)?;
        image.write_data(&raster.to_straight()).map_err(fail)?;
    }else{
        let mut image=encoder.new_image::<colortype::RGBA16>(raster.width,raster.height).map_err(fail)?;
        image.resolution(ResolutionUnit::Inch,resolution);
        image.encoder().write_tag(Tag::ExtraSamples,&[2u16][..]).map_err(fail)?;
        image.write_data(&raster.to_depth(PixelDepth::Sixteen).deep_pixels().unwrap().straight_u16()).map_err(fail)?;
    }
    Ok(out.into_inner())
}
