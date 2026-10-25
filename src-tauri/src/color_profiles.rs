//! User-owned ICC profiles, stored separately from document pixels.
#[cfg(windows)] #[path="color_profiles_windows.rs"] mod platform;
use std::path::{Path,PathBuf};
use compositor_engine::cmyk::validate_profile;
#[derive(serde::Serialize)] pub struct ProfileInfo {pub path:String,pub name:String}
#[derive(serde::Serialize)] pub struct Proof {pub profile:ProfileInfo,pub lut:Vec<u8>}
fn read(path:&Path)->Result<Vec<u8>,String>{
    let meta=std::fs::metadata(path).map_err(|e|e.to_string())?;
    if !meta.is_file()||meta.len()>16*1024*1024{return Err("Choose an ICC profile under 16 MiB".into());}
    let bytes=std::fs::read(path).map_err(|e|e.to_string())?;validate_profile(&bytes)?;Ok(bytes)
}
fn info(path:&Path)->ProfileInfo{ProfileInfo{path:path.to_string_lossy().into_owned(),name:path.file_stem().unwrap_or_default().to_string_lossy().into_owned()}}
fn store_profile(folder:&Path,name:&str,bytes:&[u8])->Result<PathBuf,String>{
    validate_profile(bytes)?;std::fs::create_dir_all(folder).map_err(|e|e.to_string())?;
    let hash=bytes.iter().fold(0xcbf29ce484222325u64,|h,b|(h^*b as u64).wrapping_mul(0x100000001b3));
    let name=name.chars().filter(|c|c.is_alphanumeric()||[' ','-','_'].contains(c)).take(80).collect::<String>();
    let target=folder.join(format!("{name}-{hash:016x}.icc"));
    if !target.exists(){let temp=folder.join(format!("{}.tmp",uuid::Uuid::new_v4()));std::fs::write(&temp,bytes).map_err(|e|e.to_string())?;if let Err(e)=std::fs::rename(&temp,&target){let _=std::fs::remove_file(temp);return Err(e.to_string());}}Ok(target)
}
pub fn list(folder:&Path)->Vec<ProfileInfo>{
    let mut folders=vec![folder.to_owned()];
    #[cfg(windows)] if let Ok(path)=platform::directory(){folders.push(path);}
    let mut profiles=Vec::new();
    for folder in folders {if let Ok(files)=std::fs::read_dir(folder){for file in files.flatten().take(10000){let path=file.path();if path.extension().and_then(|x|x.to_str()).is_some_and(|s|s.eq_ignore_ascii_case("icc")||s.eq_ignore_ascii_case("icm"))&&read(&path).is_ok(){profiles.push(info(&path));}}}}
    profiles.sort_by_key(|p|p.name.to_lowercase());profiles.dedup_by(|a,b|a.path==b.path);profiles
}
pub fn proof(path:&str,folder:&Path,import:bool)->Result<Proof,String>{
    let source=PathBuf::from(path);let bytes=read(&source)?;
    #[cfg(windows)] let lut=platform::proof(&bytes)?;
    #[cfg(not(windows))] let lut:Vec<u8>={return Err("ICC proofing requires Windows".into());};
    #[allow(unreachable_code)] {
        let path=if import {
            store_profile(folder,&source.file_stem().unwrap_or_default().to_string_lossy(),&bytes)?
        }else{source};
        Ok(Proof{profile:info(&path),lut})
    }
}

pub fn converted_wire(imported:compositor_engine::psd::PsdImport,profile:ProfileInfo)->Result<Vec<u8>,String>{
    let package=compositor_engine::save_package(&imported.document).map_err(|e|e.to_string())?;
    let header=serde_json::to_vec(&serde_json::json!({"manifest":package.manifest_json,"images":package.images.iter().map(|(name,bytes)|serde_json::json!({"name":name,"length":bytes.len()})).collect::<Vec<_>>(),"conversions":imported.conversions,"artboards":imported.artboard_bounds,"profile":profile})).map_err(|e|e.to_string())?;
    if header.len()>8*1024*1024{return Err("Converted image metadata exceeds 8 MiB".into());}
    let size=package.images.iter().try_fold(header.len()+8,|n,(_,b)|n.checked_add(b.len())).filter(|n|*n<=512*1024*1024).ok_or("Converted image exceeds 512 MiB")?;
    let mut out=Vec::new();out.try_reserve_exact(size).map_err(|_|"Not enough memory for color-managed image")?;
    out.extend(b"NCP1");out.extend((header.len()as u32).to_le_bytes());out.extend(header);for(_,bytes)in package.images{out.extend(bytes);}Ok(out)
}
#[cfg(windows)]
pub fn import_file(path:&str,fallback:Option<&str>,folder:&Path)->Result<Vec<u8>,String>{
    use compositor_engine::{Document,Layer,Point,psd,MAX_PIXELS,CommandError};
    let path=Path::new(path);let meta=std::fs::metadata(path).map_err(|e|e.to_string())?;if !meta.is_file()||meta.len()>512*1024*1024{return Err("Choose an image under 512 MiB".into());}
    let ext=path.extension().and_then(|e|e.to_str()).unwrap_or("").to_ascii_lowercase();
    if !["psd","psb","jpg","jpeg","tif","tiff"].contains(&ext.as_str()){return Err("Unsupported CMYK image format".into());}
    // An old profile preference must not prevent RGB files, or CMYK files with
    // their own embedded profile, from opening after that profile was removed.
    let fallback=||fallback.map(|path|read(Path::new(path))).transpose();
    let (imported,icc)=if ["psd","psb"].contains(&ext.as_str()){
        use std::io::Read;
        let mut file=std::fs::File::open(path).map_err(|e|e.to_string())?;let mut header=[0;26];file.read_exact(&mut header).map_err(|e|e.to_string())?;
        if &header[..4]!=b"8BPS"||u16::from_be_bytes(header[24..26].try_into().unwrap())!=4{return Ok(Vec::new());}
        let bytes=std::fs::read(path).map_err(|e|e.to_string())?;
        let embedded=psd::embedded_cmyk_profile(&bytes).map_err(|e|e.to_string())?;
        let icc=match embedded{Some(profile)=>Some(profile),None=>fallback()?}.ok_or("This CMYK PSD has no ICC profile. Choose its print profile in View > CMYK Proof before importing.")?;
        let transform=platform::InkTransform::new(&icc)?;
        let converter=|pixels:&mut[u8],w,h|transform.apply(pixels,w,h).map_err(CommandError::Argument);
        (psd::read_with_cmyk(&bytes,MAX_PIXELS,MAX_PIXELS,Some(&converter)).map_err(|e|e.to_string())?,icc)
    }else{
        let Some(image)=crate::cmyk_wic::decode(path,fallback,&platform::srgb_bytes()?)?else{return Ok(Vec::new());};
        validate_profile(&image.profile)?;
        let bytes=std::fs::read(path).map_err(|e|e.to_string())?;
        let raster=compositor_engine::cmyk::oriented_rgba(&bytes,image.width,image.height,image.rgba)?;
        let mut document=Document::new(raster.width,raster.height);document.resolution=image.dpi;
        let layer=Layer::with_pixels(&path.file_stem().unwrap_or_default().to_string_lossy(),raster,Point{x:0.,y:0.});document.active_layer_id=Some(layer.id);document.layers.push(layer);
        (psd::PsdImport{document,conversions:Vec::new(),artboard_bounds:Vec::new()},image.profile)
    };
    let saved=store_profile(folder,"Embedded CMYK",&icc)?;converted_wire(imported,info(&saved))
}
#[cfg(not(windows))]
pub fn import_file(_path:&str,_fallback:Option<&str>,_folder:&Path)->Result<Vec<u8>,String>{Err("CMYK image decoding requires Windows".into())}
pub fn export(png:&[u8],path:&str,format:&str,quality:f64,dpi:f64)->Result<Vec<u8>,String>{
    let icc=read(Path::new(path))?;
    if !png.starts_with(b"\x89PNG\r\n\x1a\n")||png.len()>512*1024*1024{return Err("Image exceeds 512 MiB".into());}
    // PNG decoder enforces the engine's 100 MP and 30,000-pixel limits.
    let raster=compositor_engine::decode_image(png).map_err(|e|e.to_string())?.raster;
    let (w,h)=(raster.width,raster.height);let mut pixels=raster.into_bytes();
    if pixels.chunks_exact(4).any(|p|p[3]!=255){return Err("CMYK output must be flattened over a background".into());}
    #[cfg(windows)] platform::to_cmyk(&icc,&mut pixels,w,h)?;
    #[cfg(not(windows))] return Err("CMYK conversion requires Windows".into());
    #[allow(unreachable_code)] compositor_engine::cmyk::encode(&pixels,w,h,dpi,format,quality,&icc).map_err(|e|e.to_string())
}
