//! Bounded PSD/PSB RGB8 import. The record layout and conversions follow the MIT
//! Compositor 1.4.5 reader (086f163), with checked section-local cursors.
use crate::*;
use serde::Serialize;
use std::collections::BTreeMap;
use uuid::Uuid;
#[path="psd_semantics.rs"] mod semantics;
#[path="psd_mac_roman.rs"] mod mac_roman;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PsdConversion { pub layer_name: String, pub message: String }
pub struct PsdImport { pub document: Document, pub conversions: Vec<PsdConversion> }
fn error(message: &str) -> CommandError { CommandError::Refused(format!("Photoshop import: {message}")) }
fn truncated() -> CommandError { error("truncated or inconsistent section length") }

struct Cursor<'a> { bytes: &'a [u8], at: usize }
impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self { Self { bytes, at: 0 } }
    fn take(&mut self, n: usize) -> Result<&'a [u8], CommandError> {
        let end = self.at.checked_add(n).ok_or_else(truncated)?;
        let out = self.bytes.get(self.at..end).ok_or_else(truncated)?; self.at=end; Ok(out)
    }
    fn skip(&mut self, n: usize) -> Result<(), CommandError> { self.take(n).map(|_| ()) }
    fn u8(&mut self) -> Result<u8, CommandError> { Ok(self.take(1)?[0]) }
    fn u16(&mut self) -> Result<u16, CommandError> { Ok(u16::from_be_bytes(self.take(2)?.try_into().unwrap())) }
    fn i16(&mut self) -> Result<i16, CommandError> { Ok(self.u16()? as i16) }
    fn u32(&mut self) -> Result<u32, CommandError> { Ok(u32::from_be_bytes(self.take(4)?.try_into().unwrap())) }
    fn i32(&mut self) -> Result<i32, CommandError> { Ok(self.u32()? as i32) }
    fn u64(&mut self) -> Result<u64, CommandError> { Ok(u64::from_be_bytes(self.take(8)?.try_into().unwrap())) }
    fn length(&mut self, big: bool) -> Result<usize, CommandError> {
        let n=if big { self.u64()? } else { self.u32()? as u64 };
        usize::try_from(n).map_err(|_| truncated())
    }
    fn section(&mut self, big: bool) -> Result<Cursor<'a>, CommandError> { let n=self.length(big)?; Ok(Cursor::new(self.take(n)?)) }
    fn remaining(&self) -> usize { self.bytes.len()-self.at }
    fn key(&mut self) -> Result<&'a str, CommandError> { std::str::from_utf8(self.take(4)?).map_err(|_| truncated()) }
}
#[derive(Clone, Copy, Debug, Default)]
struct Bounds { top: i64, left: i64, bottom: i64, right: i64 }
impl Bounds {
    fn read(c: &mut Cursor) -> Result<Self, CommandError> { Ok(Self { top:c.i32()? as i64,left:c.i32()? as i64,bottom:c.i32()? as i64,right:c.i32()? as i64 }) }
    fn dimensions(self) -> Result<(usize,usize),CommandError> {
        let (w,h)=(self.right-self.left,self.bottom-self.top);
        if w<0 || h<0 || w>MAX_SIDE || h>MAX_SIDE { return Err(error("layer or mask bounds exceed the size limit")); }
        Ok((w as usize,h as usize))
    }
    fn pixels(self) -> Result<u64,CommandError> { let (w,h)=self.dimensions()?;Ok(w as u64*h as u64) }
    fn cropped(self,w:u32,h:u32)->Self { Self {left:self.left.clamp(0,w as i64),right:self.right.clamp(0,w as i64),top:self.top.clamp(0,h as i64),bottom:self.bottom.clamp(0,h as i64)} }
    fn origin(self)->Point { Point { x:self.left as f64,y:self.top as f64 } }
}
struct Record<'a> {
    name:String, bounds:Bounds, retained:Bounds, channels:Vec<(i16,usize)>, blend:String,
    opacity:u8,fill:u8,clipping:bool,hidden:bool,section:u32,extra:BTreeMap<String,&'a [u8]>,
    mask_bounds:Bounds,mask_retained:Bounds,mask_default:u8,mask_flags:u8,has_mask:bool,
    pixels:Option<Raster>,mask:Option<GrayRaster>,cropped:bool,
}
impl Record<'_>{fn owns_pixels(&self)->bool{self.section==0&&self.channels.iter().any(|(id,_)|(0..=2).contains(id))}}
fn read_record<'a>(c:&mut Cursor<'a>,big:bool)->Result<Record<'a>,CommandError> {
    let bounds=Bounds::read(c)?;bounds.dimensions()?;
    let count=c.u16()? as usize;if count>56 { return Err(error("too many layer channels")); }
    let mut channels=Vec::with_capacity(count);
    for _ in 0..count { channels.push((c.i16()?,c.length(big)?)); }
    if c.key()?!="8BIM" { return Err(truncated()); }
    let blend=c.key()?.to_string();let opacity=c.u8()?;let clipping=c.u8()?!=0;let hidden=c.u8()?&2!=0;c.skip(1)?;
    let mut extra_cursor=c.section(false)?;
    let mut mask_cursor=extra_cursor.section(false)?;
    let has_mask=mask_cursor.remaining()>=18;
    let (mask_bounds,mask_default,mask_flags)=if has_mask { (Bounds::read(&mut mask_cursor)?,mask_cursor.u8()?,mask_cursor.u8()?) } else { (Bounds::default(),255,0) };
    mask_bounds.dimensions()?;
    let n=extra_cursor.u32()? as usize;extra_cursor.skip(n)?;
    let name_len=extra_cursor.u8()? as usize;
    // Unicode 'luni' overrides the legacy Pascal name below.
    let mut name=extra_cursor.take(name_len)?.iter().map(|&b|if b<128 {b as char}else{mac_roman::MAC_ROMAN[(b-128) as usize]}).collect::<String>();
    extra_cursor.skip((4-(name_len+1)%4)%4)?;
    let mut extra=BTreeMap::new();let mut section=0;let mut fill=255;
    while extra_cursor.remaining()>=12 {
        let signature=extra_cursor.key()?;
        if signature!="8BIM" && signature!="8B64" { return Err(error("invalid additional layer information signature")); }
        let key=extra_cursor.key()?;
        let large=signature=="8B64" || big && ["LMsk","Lr16","Lr32","Layr","Mt16","Mt32","Mtrn","Alph","FMsk","lnk2","FEid","FXid","PxSD"].contains(&key);
        let n=extra_cursor.length(large)?;let payload=extra_cursor.take(n)?;
        if n%2!=0 && extra_cursor.remaining()>0 { extra_cursor.skip(1)?; }
        if key=="luni" { let mut u=Cursor::new(payload);let count=u.u32()? as usize;if count>100_000{return Err(error("layer name exceeds the character limit"));}let bytes=u.take(count.checked_mul(2).ok_or_else(truncated)?)?;
            let units:Vec<_>=bytes.chunks_exact(2).map(|p|u16::from_be_bytes([p[0],p[1]])).collect();name=String::from_utf16_lossy(&units).trim_end_matches('\0').to_string(); }
        if key=="iOpa" { fill=*payload.first().ok_or_else(truncated)?; }
        if key=="lsct" || key=="lsdk" { section=Cursor::new(payload).u32()?; }
        extra.insert(key.to_string(),payload);
    }
    Ok(Record {name,bounds,retained:bounds,channels,blend,opacity,fill,clipping,hidden,section,extra,mask_bounds,mask_retained:mask_bounds,mask_default,mask_flags,has_mask,pixels:None,mask:None,cropped:false})
}
fn alloc(n:usize,value:u8)->Result<Vec<u8>,CommandError> { let mut v=Vec::new();v.try_reserve_exact(n).map_err(|_|error("not enough memory to import this file"))?;v.resize(n,value);Ok(v) }
/// PackBits is row-local: an overrun/underrun is rejected rather than spilling into the next row.
fn packbits(bytes:&[u8],row:&mut [u8])->Result<(),CommandError> {
    let mut c=Cursor::new(bytes);let mut out:usize=0;
    while c.remaining()>0 {
        let n=c.u8()? as i8;
        if n==-128 { continue; }
        let length=if n>=0 { n as usize+1 } else { (1-n as i16) as usize };
        let end=out.checked_add(length).ok_or_else(truncated)?;let dst=row.get_mut(out..end).ok_or_else(truncated)?;
        if n>=0 { dst.copy_from_slice(c.take(length)?); } else { dst.fill(c.u8()?); } out=end;
    }
    if out!=row.len() { return Err(truncated()); } Ok(())
}
fn decode_plane(bytes:&[u8],bounds:Bounds,retained:Bounds,big:bool,default:u8)->Result<Vec<u8>,CommandError> {
    let (w,h)=bounds.dimensions()?;let (rw,rh)=retained.dimensions()?;
    let mut c=Cursor::new(bytes);let compression=c.u16()?;
    if compression>1 { return Err(error("ZIP-compressed channels are not supported; save an RGB 8-bit PSD with RLE compression")); }
    let mut out=alloc(rw*rh,default)?;
    let counts=if compression==1 { let mut counts=Vec::with_capacity(h);for _ in 0..h {counts.push(if big {c.u32()? as usize}else{c.u16()? as usize});}counts } else { Vec::new() };
    let mut row=if compression==1 {alloc(w,0)?}else{Vec::new()};
    for y in 0..h {
        let bytes=if compression==1 { let data=c.take(counts[y])?;packbits(data,&mut row)?;row.as_slice() } else { c.take(w)? };
        let document_y=bounds.top+y as i64;
        if rw>0 && rh>0 && document_y>=retained.top && document_y<retained.bottom {
            let x=usize::try_from(retained.left-bounds.left).map_err(|_|truncated())?;let iy=(document_y-retained.top) as usize;
            out[iy*rw..(iy+1)*rw].copy_from_slice(bytes.get(x..x+rw).ok_or_else(truncated)?);
        }
    }
    if c.remaining()!=0 { return Err(error("channel length does not match its pixel data")); }Ok(out)
}
fn decode_record(c:&mut Cursor,r:&mut Record,big:bool)->Result<(),CommandError> {
    let (w,h)=r.retained.dimensions()?;let n=w*h;
    let owns_pixels=r.owns_pixels();
    let mut rgba=if owns_pixels && n>0 { alloc(n*4,0)? } else { Vec::new() };
    for p in rgba.chunks_exact_mut(4) {p[3]=255;}
    for &(id,len) in &r.channels {
        let data=c.take(len)?;
        if id== -2 && r.has_mask && r.mask_flags&8==0 {
            let (mw,mh)=r.mask_retained.dimensions()?;
            let plane=decode_plane(data,r.mask_bounds,r.mask_retained,big,r.mask_default)?;
            if mw>0&&mh>0 { r.mask=Some(GrayRaster::from_bytes(mw as u32,mh as u32,plane)); }
        } else if owns_pixels&&(-1..=2).contains(&id) {
            let plane=decode_plane(data,r.bounds,r.retained,big,if id== -1 {255}else{0})?;
            if !rgba.is_empty() {let channel=if id== -1 {3}else{id as usize};for (p,v) in rgba.chunks_exact_mut(4).zip(plane) {p[channel]=v;}}
        }
    }
    if !rgba.is_empty() { for p in rgba.chunks_exact_mut(4) { for i in 0..3 {p[i]=((p[i] as u16*p[3] as u16+127)/255) as u8;} }r.pixels=Some(Raster::from_premultiplied(w as u32,h as u32,rgba)); }Ok(())
}
fn blend(key:&str)->Option<BlendMode> {Some(match key {
    "norm"|"pass"=>BlendMode::Normal,"mul "=>BlendMode::Multiply,"scrn"=>BlendMode::Screen,"over"=>BlendMode::Overlay,"dark"=>BlendMode::Darken,"lite"=>BlendMode::Lighten,
    "diff"=>BlendMode::Difference,"div "=>BlendMode::ColorDodge,"idiv"=>BlendMode::ColorBurn,"hue "=>BlendMode::Hue,"sat "=>BlendMode::Saturation,"colr"=>BlendMode::Color,"lum "=>BlendMode::Luminosity,
    "lbrn"=>BlendMode::LinearBurn,"lddg"=>BlendMode::LinearDodge,"sLit"=>BlendMode::SoftLight,"hLit"=>BlendMode::HardLight,"vLit"=>BlendMode::VividLight,"lLit"=>BlendMode::LinearLight,"pLit"=>BlendMode::PinLight,"hMix"=>BlendMode::HardMix,"smud"=>BlendMode::Exclusion,"fsub"=>BlendMode::Subtract,"fdiv"=>BlendMode::Divide,_=>return None})}
fn has(r:&Record,keys:&[&str])->bool { keys.iter().any(|k|r.extra.contains_key(*k)) }
fn adjustment(r:&Record)->Option<LayerAdjustment> {
    if let Some(data)=r.extra.get("levl") {
        if data.len()<292 {return None;}let mut c=Cursor::new(data);c.u16().ok()?;let mut a=LayerAdjustment::new(AdjustmentKind::Levels);
        for range in &mut a.levels.ranges {let black=c.u16().ok()? as f64;let white=c.u16().ok()? as f64;let output_black=c.u16().ok()? as f64;let output_white=c.u16().ok()? as f64;let gamma=c.u16().ok()? as f64/100.;*range=LevelRange{black,white,output_black,output_white,gamma}.normalized();}return Some(a);
    }
    if let Some(data)=r.extra.get("curv") {
        let mut c=Cursor::new(data);if data.first()==Some(&0) {c.skip(1).ok()?;}let version=c.u16().ok()?;if version!=1&&version!=4 {return None;}let count=c.u16().ok()?;let mut a=LayerAdjustment::new(AdjustmentKind::Curves);
        for i in 0..count.min(4) as usize { let n=c.u16().ok()?;if n>32 {return None;}let mut points=Vec::new();for _ in 0..n {let y=(c.u16().ok()? as f64).min(255.);let x=(c.u16().ok()? as f64).min(255.);points.push(CurvePoint{x,y});}
            if points.len()>=2 {points.sort_by(|a,b|a.x.total_cmp(&b.x));if points[0].x!=0. {points.insert(0,CurvePoint{x:0.,y:points[0].y});}if points.last()?.x!=255. {points.push(CurvePoint{x:255.,y:points.last()?.y});}a.curves.channels[i]=points;}}
        return a.curves.is_valid().then_some(a);
    }
    if let Some(data)=r.extra.get("hue2").or_else(||r.extra.get("hue ")) {
        let mut c=Cursor::new(data);c.u16().ok()?;let colorize=c.u8().ok()?!=0;c.skip(1).ok()?;
        let values=|c:&mut Cursor|->Option<RangeAdjustment>{Some(RangeAdjustment{hue:c.i16().ok()? as f64,saturation:c.i16().ok()? as f64,lightness:c.i16().ok()? as f64})};
        let colored=values(&mut c)?;let master=values(&mut c)?;let mut s=HueSaturationSettings::default();s.colorize=colorize;s.adjustments.insert(ColorRange::Master,if colorize {colored}else{master});
        if !colorize {for range in ColorRange::ALL.iter().filter(|&&r|r!=ColorRange::Master) {if c.remaining()<14 {break;}let mut degree=||c.i16().ok().map(|n|(n as f64).rem_euclid(360.));let band=HueBand{falloff_start:degree()?,range_start:degree()?,range_end:degree()?,falloff_end:degree()?};s.bands.insert(*range,band);s.adjustments.insert(*range,values(&mut c)?);}}
        let mut a=LayerAdjustment::new(AdjustmentKind::Hsv);if !s.is_valid() {return None;}a.hsv_settings=Some(s);return Some(a);
    }None
}
const ADJUSTMENTS:&[&str]=&["levl","curv","hue2","hue ","expA","grdm","brit","blnc","nvrt","thrs","post","mixr","selc","blwh","phfl","vibA"];

pub fn matches(bytes:&[u8])->bool {bytes.starts_with(b"8BPS")}
pub fn read(bytes:&[u8],remaining_pixels:u64,remaining_masks:u64)->Result<PsdImport,CommandError> {
    if bytes.len() as u64>MAX_ASSET_BYTES {return Err(error("file exceeds 512 MiB"));}
    let mut c=Cursor::new(bytes);if c.key()?!="8BPS" {return Err(error("invalid signature"));}let version=c.u16()?;
    if version!=1&&version!=2 {return Err(error("unsupported PSD/PSB version"));}let big=version==2;c.skip(6)?;let channels=c.u16()?;
    let height=c.u32()?;let width=c.u32()?;
    if width==0||height==0||width as i64>MAX_SIDE||height as i64>MAX_SIDE||width as u64*height as u64>MAX_PIXELS {return Err(error("canvas exceeds the 30,000-pixel side or 100 MP limit"));}
    if c.u16()?!=8 {return Err(error("only 8-bit Photoshop documents are supported"));}if c.u16()?!=3 {return Err(error("only RGB Photoshop documents are supported"));}
    let n=c.u32()? as usize;c.skip(n)?;let mut resources=c.section(false)?;let mut resolution=72.;
    while resources.remaining()>=12 {if resources.key()?!="8BIM" {return Err(truncated());}let id=resources.u16()?;let n=resources.u8()? as usize;resources.skip(n)?;if (n+1)%2!=0 {resources.skip(1)?;}let mut data=resources.section(false)?;
        if id==1005&&data.remaining()>=4 {resolution=(data.u32()? as f64/65536.).clamp(1.,9600.);}if data.bytes.len()%2!=0 {resources.skip(1)?;}}
    let mut section=c.section(big)?;let mut records=Vec::new();
    if section.remaining()>=if big {8}else{4} {let mut layers=section.section(big)?;
        if layers.remaining()>=2 {let count=(layers.i16()? as i32).unsigned_abs() as usize;if count>MAX_LAYERS {return Err(error("too many layers"));}for _ in 0..count {records.push(read_record(&mut layers,big)?);}
            let sums=|rs:&[Record]|->Result<(u64,u64),CommandError>{let mut images=0;let mut masks=0;for r in rs {if r.owns_pixels() {images+=r.retained.pixels()?;}if r.has_mask {masks+=r.mask_retained.pixels()?;}}Ok((images,masks))};
            let (images,masks)=sums(&records)?;
            if images>remaining_pixels || masks>remaining_masks { for r in &mut records {r.retained=r.bounds.cropped(width,height);r.mask_retained=r.mask_bounds.cropped(width,height);r.cropped=r.retained.pixels()?!=r.bounds.pixels()?||r.mask_retained.pixels()?!=r.mask_bounds.pixels()?;}
                let (images,masks)=sums(&records)?;if images>remaining_pixels||masks>remaining_masks {return Err(error("layers or masks exceed the remaining 100 MP document budget"));}}
            for r in &mut records {decode_record(&mut layers,r,big)?;}
        }
    }
    let mut reserved_images=records.iter().filter(|r|r.section==0).filter_map(|r|r.pixels.as_ref()).map(|p|p.width as u64*p.height as u64).sum::<u64>();
    let mut document=Document::new(width,height);document.resolution=resolution;let mut conversions=Vec::new();let mut groups=Vec::new();let mut bases=BTreeMap::new();let mut converted_masks=0u64;
    for mut r in records.into_iter().rev() {
        if r.section==3 {if groups.len()>=MAX_NESTING {return Err(error("folder nesting exceeds 64"));}groups.push(Uuid::new_v4());continue;}
        let group=r.section==1||r.section==2;
        let mut layer=if let Some(p)=r.pixels.take().filter(|_|!group) {Layer::with_pixels(&r.name,p,r.retained.origin())}else{Layer::blank(&r.name,document.size())};
        if group {layer.id=groups.pop().ok_or_else(||error("unbalanced folder records"))?;layer.is_group=true;}
        layer.parent_id=groups.last().copied();layer.visible=!r.hidden;
        let effects=has(&r,&["lfx2","lrFX","lmfx"]);
        layer.opacity=r.opacity as f64/255.*if effects {1.}else{r.fill as f64/255.};
        let mut notes=Vec::new();if r.cropped {notes.push("Pixels outside the canvas were cropped so the file fits in memory.".to_string());}
        if group {if r.blend!="pass"&&r.blend!="norm" {notes.push(format!("Folder blend mode '{}' was converted to pass-through.",r.blend));}}
        else if let Some(mode)=blend(&r.blend) {layer.blend_mode=mode;}else{notes.push(format!("Blend mode '{}' was converted to Normal.",r.blend));}
        if effects {notes.push("Photoshop layer effects were discarded; appearance may differ.".into());}
        if has(&r,&["SoLd","SoLE"]) {notes.push("The smart object was rasterized; linked contents are not editable.".into());}
        if !group&&has(&r,&["TySh","tySh","txt2"]) {if let Some((style,text_notes))=semantics::text(&r.extra).filter(|_|layer.pixels.is_some()) {layer.extra.text=Some(style);notes.extend(text_notes);}else{notes.push("The text layer was imported as pixels; editable text conversion is unavailable for this record.".into());}}
        else if !group&&has(&r,&["vmsk","vsms","vogk"]) {if let Some((spec,color,shape_notes))=semantics::shape(&r.extra) {
            let bounds=spec.bounds();let new_pixels=bounds.width as u64*bounds.height as u64;let old_pixels=layer.pixels.as_ref().map_or(0,|p|p.width as u64*p.height as u64);
            reserved_images=reserved_images-old_pixels+new_pixels;if reserved_images>remaining_pixels {return Err(error("editable shapes exceed the remaining document budget"));}
            layer.pixels=Some(ops::shape::shape_raster(&spec,color));layer.transform=LayerTransform::axis_aligned(Point{x:bounds.x,y:bounds.y},Size{width:bounds.width,height:bounds.height});layer.extra.shape=Some(ops::shape::shape_record(&spec,color));notes.extend(shape_notes);
        }else{notes.push("The vector layer was imported as pixels; editable shape conversion is unavailable for this record.".into());}}
        if !group&&has(&r,ADJUSTMENTS) {layer.extra.adjustment=adjustment(&r);if layer.extra.adjustment.is_some() {layer.pixels=None;layer.transform=LayerTransform::axis_aligned(Point{x:0.,y:0.},document.size());}notes.push(if layer.extra.adjustment.is_some() {"Adjustment parameters may differ from Photoshop.".into()}else{"This adjustment type is unsupported and was skipped.".into()});}
        let missing_fallback=!group&&layer.pixels.is_none()&&layer.extra.text.is_none()&&layer.extra.shape.is_none()&&!has(&r,ADJUSTMENTS)&&has(&r,&["TySh","tySh","txt2","vmsk","vsms","vogk","SoLd","SoLE"]);
        if missing_fallback{notes.retain(|n|!n.contains("imported as pixels")&&!n.contains("rasterized"));notes.push("This unsupported editable record has no cached pixels and was skipped.".into());}
        conversions.extend(notes.into_iter().map(|message|PsdConversion{layer_name:r.name.clone(),message}));
        if missing_fallback{continue;}
        if !group&&has(&r,ADJUSTMENTS)&&layer.extra.adjustment.is_none() {continue;}
        if r.has_mask&&r.mask_flags&8==0 {let (mw,mh)=r.mask_retained.dimensions()?;
            // Keep Photoshop's default mask outside its rectangle by materializing it on the
            // layer's grid, like PSDDocumentBuilder.maskOnLayerGrid on Mac.
            let (lw,lh)=layer.pixels.as_ref().map(|p|(p.width,p.height)).unwrap_or((width,height));
            let origin=if layer.pixels.is_some() {layer.transform.origin}else{Point{x:0.,y:0.}};
            converted_masks+=lw as u64*lh as u64;if converted_masks>remaining_masks {return Err(error("converted masks exceed the remaining document budget"));}
            let mut gray=alloc(lw as usize*lh as usize,r.mask_default)?;
            if let Some(mask)=r.mask {for y in 0..lh as usize {let my=origin.y as i64+y as i64-r.mask_retained.top;if my<0||my>=mh as i64 {continue;}for x in 0..lw as usize {let mx=origin.x as i64+x as i64-r.mask_retained.left;if mx>=0&&mx<mw as i64 {gray[y*lw as usize+x]=mask.bytes()[my as usize*mw+mx as usize];}}}}
            layer.mask=Some(Mask{pixels:GrayRaster::from_bytes(lw,lh,gray),enabled:r.mask_flags&2==0,linked:Some(r.mask_flags&1==0),placement:None});
        }
        if r.clipping {layer.mask_source_id=bases.get(&layer.parent_id).copied();if layer.mask_source_id.is_none() {conversions.push(PsdConversion{layer_name:r.name,message:"The clipping base is missing; clipping was removed.".into()});}}
        else if !group {bases.insert(layer.parent_id,layer.id);}
        document.layers.push(layer);
    }
    if !groups.is_empty() {return Err(error("unbalanced folder records"));}
    if document.layers.is_empty() {
        if channels<3||channels>56 {return Err(error("invalid merged channel count"));}
        let n=width as usize*height as usize;if n as u64>remaining_pixels {return Err(error("merged image exceeds the remaining document budget"));}
        let compression=c.u16()?;if compression>1 {return Err(error("unsupported merged image compression"));}
        let counts=if compression==1 {let mut rows=Vec::new();for _ in 0..channels as usize*height as usize {rows.push(if big {c.u32()? as usize}else{c.u16()? as usize});}rows}else{Vec::new()};
        let mut rgba=alloc(n*4,0)?;for p in rgba.chunks_exact_mut(4) {p[3]=255;}let mut row=alloc(width as usize,0)?;
        for ch in 0..channels as usize {for y in 0..height as usize {let bytes=if compression==1 {let bytes=c.take(counts[ch*height as usize+y])?;packbits(bytes,&mut row)?;row.as_slice()}else{c.take(width as usize)?};if ch<4 {for (x,&v) in bytes.iter().enumerate() {rgba[(y*width as usize+x)*4+ch]=v;}}}}
        for p in rgba.chunks_exact_mut(4) {for i in 0..3 {p[i]=((p[i] as u16*p[3] as u16+127)/255) as u8;}}
        document.layers.push(Layer::with_pixels("Merged image",Raster::from_premultiplied(width,height,rgba),Point{x:0.,y:0.}));
    }
    if document.used_pixels()>remaining_pixels||document.used_mask_pixels()>remaining_masks {return Err(error("converted layers exceed the remaining document budget"));}
    document.active_layer_id=document.layers.iter().rev().find(|l|!l.is_group).map(|l|l.id);
    document.manifest().validate()?;Ok(PsdImport{document,conversions})
}
