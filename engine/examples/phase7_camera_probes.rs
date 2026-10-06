//! Produce fresh local-only cross-platform Camera Raw acceptance probes.
use compositor_engine::{*,adjust::camera_raw::*};
use std::{fs,path::Path};
fn package(document:&Document,path:&Path)->Result<(),Box<dyn std::error::Error>>{let saved=save_package(document)?;fs::create_dir_all(path.join("images"))?;fs::write(path.join("manifest.json"),saved.manifest_json)?;for(name,bytes)in saved.images{fs::write(path.join("images").join(name),bytes)?;}Ok(())}
fn main()->Result<(),Box<dyn std::error::Error>>{
    let path=std::env::args().nth(1).ok_or("usage: phase7_camera_probes new-output-directory")?;let root=Path::new(&path);if root.exists(){return Err("output exists; preserve the previous evidence".into());}fs::create_dir_all(root)?;
    let(w,h)=(512u32,320u32);let mut bytes=Vec::new();
    for y in 0..h{for x in 0..w{let a=if y<40{(x*255/(w-1))as u8}else{255};let rgb=if y<160{let v=(x*255/(w-1))as u8;[v,v,v]}else if y<240{match x/64{0=>[220,40,30],1=>[230,120,30],2=>[220,210,30],3=>[30,200,50],4=>[30,200,200],5=>[30,60,220],6=>[150,40,200],_=>[210,40,120]}}else{if (x/8+y/8)%2==0{[50,95,140]}else{[185,135,70]}};bytes.extend([rgb[0],rgb[1],rgb[2],a]);}}
    let source=Raster::from_straight(w,h,&bytes);let mut original=Document::new(w,h);let layer=Layer::with_pixels("Camera Raw probe",source.clone(),Point{x:0.,y:0.});original.active_layer_id=Some(layer.id);original.layers.push(layer);package(&original,&root.join("00-source.comp"))?;fs::write(root.join("00-source.png"),compositor::export_png(&original)?)?;
    let mut light=CameraRawSettings::default();light.exposure=0.5;light.contrast=12.;light.highlights=-20.;light.shadows=15.;light.temperature=10.;light.tint=-6.;light.vibrance=18.;light.curve.darks=-10.;
    let mut color=CameraRawSettings::default();color.mixer.hue[0]=12.;color.mixer.saturation[5]=-20.;color.mixer.luminance[3]=10.;color.grading.shadows.hue=210.;color.grading.shadows.saturation=15.;color.grading.highlights.hue=40.;color.grading.highlights.saturation=12.;
    let mut effects=CameraRawSettings::default();effects.texture=12.;effects.clarity=10.;effects.dehaze=8.;effects.vignette_amount=-15.;effects.detail.sharpen_amount=35.;effects.detail.noise_luminance=15.;effects.optics.distortion=8.;
    let mut geometry=CameraRawSettings::default();geometry.geometry.rotate=3.;geometry.geometry.vertical=12.;geometry.geometry.horizontal=-6.;geometry.geometry.aspect=8.;geometry.calibration.shadow_tint=10.;geometry.calibration.red_hue=15.;geometry.calibration.blue_saturation=-10.;
    let cases=[("01-light-color-curve",light),("02-mixer-grading",color),("03-effects-detail-optics",effects),("04-geometry-calibration",geometry)];
    let mut recipes=Vec::new();for(name,settings)in cases{package(&original,&root.join(format!("{name}-source.comp")))?;let mut edited=original.clone();edited.layers[0].pixels=Some(settings.apply(&source)?);edited.layers[0].pixels_revision+=1;package(&edited,&root.join(format!("{name}-windows-expected.comp")))?;fs::write(root.join(format!("{name}-windows-expected.png")),compositor::export_png(&edited)?)?;recipes.push(serde_json::json!({"name":name,"settings":settings}));}
    fs::write(root.join("recipes.json"),serde_json::to_string_pretty(&recipes)?)?;println!("Four independent Camera Raw cases retained: {}",root.display());Ok(())
}
