//! Local-only acceptance helper: user samples are never repository fixtures.
use compositor_engine::*;
fn main()->Result<(),Box<dyn std::error::Error>> {
    let args:Vec<_>=std::env::args().collect();
    if args.len()!=3 {return Err("usage: phase7_psd_probe input.psd new-output-directory".into());}
    let input=std::path::Path::new(&args[1]);let output=std::path::Path::new(&args[2]);
    if output.exists() {return Err("output directory already exists; refusing to overwrite".into());}
    let bytes=std::fs::read(input)?;let imported=psd::read(&bytes,MAX_PIXELS,MAX_PIXELS)?;
    let package=save_package(&imported.document)?;
    let png=compositor::export_png(&imported.document)?;
    std::fs::create_dir_all(output.join("imported.comp/images"))?;
    std::fs::write(output.join("imported.comp/manifest.json"),&package.manifest_json)?;
    for (name,bytes) in package.images {std::fs::write(output.join("imported.comp/images").join(name),bytes)?;}
    std::fs::write(output.join("imported.png"),png)?;
    let receipt=serde_json::json!({"source":input.to_string_lossy(),"width":imported.document.width,"height":imported.document.height,"imagePixels":imported.document.used_pixels(),"maskPixels":imported.document.used_mask_pixels(),"layers":imported.document.layers.iter().map(|l|serde_json::json!({"name":l.name,"id":l.id,"parent":l.parent_id,"group":l.is_group,"visible":l.visible,"opacity":l.opacity,"blend":l.blend_mode,"hasPixels":l.pixels.is_some(),"hasMask":l.mask.is_some(),"clipping":l.mask_source_id,"adjustment":l.extra.adjustment,"text":l.extra.text,"shape":l.extra.shape})).collect::<Vec<_>>(),"conversions":imported.conversions});
    std::fs::write(output.join("receipt.json"),serde_json::to_string_pretty(&receipt)?)?;
    println!("Imported {} layers, {} conversion notices, {} x {}. Retained: {}",imported.document.layers.len(),imported.conversions.len(),imported.document.width,imported.document.height,output.display());Ok(())
}
