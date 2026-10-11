//! Export a synthetic .comp probe without changing the input package.
use compositor_engine::*;
use std::{path::Path, error::Error};
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 { return Err("usage: phase35d_sampling_probe input.comp new-output.png".into()); }
    let input = Path::new(&args[1]); let output = Path::new(&args[2]);
    if output.exists() { return Err("output already exists; refusing to overwrite".into()); }
    let manifest_json = std::fs::read_to_string(input.join("manifest.json"))?;
    let images = std::fs::read_dir(input.join("images"))?.map(|entry| {
        let entry = entry?;
        Ok((entry.file_name().to_string_lossy().into_owned(), std::fs::read(entry.path())?))
    }).collect::<Result<_, std::io::Error>>()?;
    let doc = open_package(&Package { manifest_json, images })?;
    std::fs::write(output, compositor::export_png(&doc)?)?;
    println!("Exported {} x {}: {}", doc.width, doc.height, output.display());
    Ok(())
}
