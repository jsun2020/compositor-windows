//! Export the fixed Mac CG/CI sampling matrix without editing its input projects.
use compositor_engine::*;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{error::Error, fs, io::Write, path::Path};

#[derive(Deserialize)]
struct Case {
    name: String,
    operation: String,
    package: String,
    sampling: Option<Sampling>,
    corners: Option<[[f64; 2]; 4]>,
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    fs::OpenOptions::new().write(true).create_new(true).open(path)?.write_all(bytes)?;
    Ok(())
}

fn record(output: &Path, name: &str, raster: &Raster) -> Result<Value, Box<dyn Error>> {
    write_new(&output.join(format!("{name}.rgba8")), raster.bytes())?;
    write_new(&output.join(format!("{name}.png")), &encode_png(raster, 72.0)?)?;
    Ok(json!({"name":name,"width":raster.width,"height":raster.height,"channels":4,"bytes":raster.bytes().len()}))
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 { return Err("usage: phase35d_sampling_matrix input-directory new-output-directory".into()); }
    let input = Path::new(&args[1]); let output = Path::new(&args[2]);
    let cases: Vec<Case> = serde_json::from_slice(&fs::read(input.join("cases.json"))?)?;
    // Preserve both earlier successful outputs and partial failure evidence.
    fs::create_dir(output)?;
    let mut records = Vec::new();
    for case in &cases {
        if case.name.is_empty() || !case.name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-') {
            return Err("case name must be a simple file stem".into());
        }
        let package = input.join(&case.package);
        let manifest_json = fs::read_to_string(package.join("manifest.json"))?;
        let images = fs::read_dir(package.join("images"))?.map(|entry| {
            let entry = entry?;
            Ok((entry.file_name().to_string_lossy().into_owned(), fs::read(entry.path())?))
        }).collect::<Result<_, std::io::Error>>()?;
        let doc = open_package(&Package { manifest_json, images })?;
        let layer = doc.layers.first().ok_or("missing test layer")?;
        match case.operation.as_str() {
            "layer" => {
                let raster = composite(&doc, Rect { x:0.0, y:0.0, width:doc.width as f64, height:doc.height as f64 }, doc.width, doc.height);
                records.push(record(output, &case.name, &raster)?);
            }
            "warp" => {
                let mut transform = layer.transform;
                transform.sampling = case.sampling.ok_or("missing warp sampling")?;
                let corners = case.corners.ok_or("missing warp corners")?.map(|p| Point { x:p[0], y:p[1] });
                let pixels = layer.pixels.as_ref().ok_or("missing test pixels")?;
                let (warped, placed) = ops::distort::warp(pixels, &transform, &corners, transform.sampling == Sampling::Nearest)?;
                let mut result = record(output, &case.name, &warped)?;
                result["transform"] = serde_json::to_value(placed)?;
                records.push(result);
                if let Some(mask) = &layer.mask {
                    let (warped, _) = ops::distort::warp_mask(&mask.pixels, &transform, &corners, mask.background())?;
                    let name = format!("{}-mask", case.name);
                    write_new(&output.join(format!("{name}.gray8")), warped.bytes())?;
                    write_new(&output.join(format!("{name}.png")), &encode_gray_png(&warped)?)?;
                    records.push(json!({"name":name,"width":warped.width,"height":warped.height,"channels":1,"bytes":warped.bytes().len()}));
                }
            }
            _ => return Err("unknown sampling operation".into()),
        }
        println!("Completed {}", case.name);
    }
    write_new(&output.join("result.json"), &serde_json::to_vec_pretty(&json!({
        "scope":"Windows engine on the fixed synthetic Mac comparison inputs; not Mac acceptance evidence.",
        "caseCount":cases.len(),"records":records
    }))?)?;
    Ok(())
}
