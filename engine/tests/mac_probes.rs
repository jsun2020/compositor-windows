//! Probe `.comp` projects for the user to open in Compositor 1.2.6 on the Mac (Task 7). Each
//! one is built and saved through this build's own `save_package`, so it is a real, valid v9
//! project; the ignored test below re-opens every one with `open_package` as a sanity floor,
//! then writes it (and a README telling the user what to do with it) to
//! `build-artifacts/mac-probes/`, which is git-ignored.
//!
//! Two of these are Phase 3.5b oracles that cannot be checked from Rust alone (R 4.1's clipped-
//! child-in-a-dimmed-folder math, and the Mac's 1.2.6 grain roughness kernel): the user renders
//! them on the Mac and sends the PNGs back.

use compositor_engine::*;
use std::fs;
use std::path::{Path, PathBuf};

fn probes_dir() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../build-artifacts/mac-probes"))
}

fn solid_rect(name: &str, rgba: [u8; 4], width: u32, height: u32, x: f64, y: f64) -> Layer {
    Layer::with_pixels(name, Raster::from_premultiplied(width, height, rgba.repeat((width * height) as usize)), Point { x, y })
}

fn folder(doc: &Document, opacity: f64) -> Layer {
    let mut f = Layer::blank("Folder", doc.size());
    f.is_group = true;
    f.opacity = opacity;
    f
}

/// A straight red-to-blue horizontal gradient, alpha 255 throughout.
fn red_to_blue_gradient(width: u32, height: u32) -> Raster {
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for _y in 0..height {
        for x in 0..width {
            let t = x as f64 / (width - 1).max(1) as f64;
            let r = (255.0 * (1.0 - t)).round() as u8;
            let b = (255.0 * t).round() as u8;
            data.extend_from_slice(&[r, 0, b, 255]);
        }
    }
    Raster::from_premultiplied(width, height, data)
}

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (u8, u8, u8) {
    let c = v * s;
    let hp = h / 60.0;
    let x = c * (1.0 - (hp.rem_euclid(2.0) - 1.0).abs());
    let (r1, g1, b1) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    (((r1 + m) * 255.0).round() as u8, ((g1 + m) * 255.0).round() as u8, ((b1 + m) * 255.0).round() as u8)
}

/// A full-hue sweep, for the two adjustment-layer probes: colourful enough that a Black & White
/// or Grain adjustment on top has something visible to act on.
fn colourful_gradient(width: u32, height: u32) -> Raster {
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for _y in 0..height {
        for x in 0..width {
            let (r, g, b) = hsv_to_rgb(360.0 * x as f64 / width as f64, 0.85, 0.95);
            data.extend_from_slice(&[r, g, b, 255]);
        }
    }
    Raster::from_premultiplied(width, height, data)
}

/// 1. The overlapping red/blue children in a 50% folder (Task 4's first test, at 120 x 60).
fn folder_opacity_doc() -> Document {
    let mut doc = Document::new(120, 60);
    let f = folder(&doc, 0.5);
    let fid = f.id;
    let mut red = solid_rect("Red", [255, 0, 0, 255], 80, 60, 0.0, 0.0);
    red.parent_id = Some(fid);
    let mut blue = solid_rect("Blue", [0, 0, 255, 255], 80, 60, 40.0, 0.0);
    blue.parent_id = Some(fid);
    doc.layers = vec![f, red, blue];
    doc
}

/// 2. Task 4's double-dim case (a clipped child in a dimmed folder), at 120 x 60.
fn clipped_in_dimmed_folder_doc() -> Document {
    let mut doc = Document::new(120, 60);
    let f = folder(&doc, 0.5);
    let fid = f.id;
    let mut base = solid_rect("Base", [255, 255, 255, 255], 120, 60, 0.0, 0.0);
    base.parent_id = Some(fid);
    let base_id = base.id;
    let mut child = solid_rect("Child", [255, 0, 0, 255], 120, 60, 0.0, 0.0);
    child.parent_id = Some(fid);
    child.mask_source_id = Some(base_id);
    doc.layers = vec![f, base, child];
    doc
}

/// 3. A white layer with a vertical guide at 30 and a horizontal guide at 45.5.
fn guides_doc() -> Document {
    let mut doc = Document::new(120, 60);
    let white = solid_rect("Layer 1", [255, 255, 255, 255], 120, 60, 0.0, 0.0);
    doc.layers = vec![white];
    doc.guides = vec![
        Guide { id: uuid::Uuid::new_v4(), axis: GuideAxis::Vertical, position: 30.0 },
        Guide { id: uuid::Uuid::new_v4(), axis: GuideAxis::Horizontal, position: 45.5 },
    ];
    doc
}

/// 4. Eleven half-alpha green columns, one per new blend mode in `NEW` order (see
/// `blend_modes_v9.rs`), over a red-to-blue gradient. A Phase 3.5b oracle.
fn new_blend_modes_doc() -> Document {
    const MODES: [BlendMode; 11] = [
        BlendMode::LinearBurn, BlendMode::LinearDodge, BlendMode::SoftLight, BlendMode::HardLight,
        BlendMode::VividLight, BlendMode::LinearLight, BlendMode::PinLight, BlendMode::HardMix,
        BlendMode::Exclusion, BlendMode::Subtract, BlendMode::Divide,
    ];
    let mut doc = Document::new(220, 60);
    let backdrop = Layer::with_pixels("Gradient", red_to_blue_gradient(220, 60), Point { x: 0.0, y: 0.0 });
    let mut layers = vec![backdrop];
    for (i, mode) in MODES.iter().enumerate() {
        let mut column = solid_rect(&format!("Column {i}"), [0, 128, 0, 128], 20, 60, (i as f64) * 20.0, 0.0);
        column.blend_mode = *mode;
        layers.push(column);
    }
    doc.layers = layers;
    doc
}

/// 5. A colourful gradient with a default Black & White adjustment layer on top -- an
/// adjustment kind this build parses and preserves but does not yet draw. A Phase 3.5b oracle.
fn new_adjustment_layers_doc() -> Document {
    let mut doc = Document::new(120, 60);
    let backdrop = Layer::with_pixels("Gradient", colourful_gradient(120, 60), Point { x: 0.0, y: 0.0 });
    let mut adjustment = Layer::blank("Black & White", doc.size());
    adjustment.extra.adjustment = Some(LayerAdjustment::new(AdjustmentKind::BlackWhite));
    doc.layers = vec![backdrop, adjustment];
    doc
}

/// 6. A colourful gradient with a default Grain adjustment layer on top (RULING, added to
/// Task 7): a Phase 3.5b oracle for the 1.2.6 grain roughness kernel (`Document::undrawn`'s
/// "the Compositor 1.2.6 grain roughness" disclosure fires for this exact case).
fn grain_doc() -> Document {
    let mut doc = Document::new(120, 60);
    let backdrop = Layer::with_pixels("Gradient", colourful_gradient(120, 60), Point { x: 0.0, y: 0.0 });
    let mut adjustment = Layer::blank("Grain", doc.size());
    adjustment.extra.adjustment = Some(LayerAdjustment::new(AdjustmentKind::Grain));
    doc.layers = vec![backdrop, adjustment];
    doc
}

const README_TXT: &str = "\
This folder holds test projects for Compositor on the Mac.

For each project listed below:

1. Open it in Compositor 1.2.6 on the Mac.
2. Confirm it opens without an error.
3. File > Export > PNG, at 100%, into a folder named mac-exports, using the file name given below.
4. Send the mac-exports folder back.

Projects:

- folder-opacity.comp           -> folder-opacity.png
- clipped-in-dimmed-folder.comp -> clipped-in-dimmed-folder.png
- guides.comp                   -> guides.png
- new-blend-modes.comp          -> new-blend-modes.png
- new-adjustment-layers.comp    -> new-adjustment-layers.png
- grain.comp                    -> grain.png
- edited-rich-file.comp         -> edited-rich-file.png (confirm it opens, and export a PNG)
";

/// 7. RULING (F5, replacing the M9 tautological final-existence loop): the Mac acceptance probe.
/// The other six probes are rendering oracles for documents this build only opened (or built)
/// and re-saved unmodified; none of them sends the Mac a file this build has actually EDITED
/// while carrying effects, live text, an unknown-version adjustment kind and unknown keys at
/// both levels -- the case this phase's "the Mac accepts what we write" constraint is really
/// about. Starts from the same kind of rich v9 document as `preservation_through_commands.rs`,
/// restricted to features the Mac's own validation/decode accepts (no free-form `shape`, whose
/// exact Mac schema this build does not know), then edits it through `Engine::execute` before
/// saving, exactly as a user would.
fn edited_rich_file_doc() -> Document {
    let folder_id = "0B6C6B1E-4F1B-4B4E-9E0A-EEEEEEEEEEE1";
    let text_id = "0B6C6B1E-4F1B-4B4E-9E0A-EEEEEEEEEEE2";
    let adj_id = "0B6C6B1E-4F1B-4B4E-9E0A-EEEEEEEEEEE3";
    let image_file = format!("{text_id}.png");
    let manifest = serde_json::json!({
        "format": "com.compositor.project", "version": 9, "colorSpace": "sRGB",
        "documentID": "0B6C6B1E-4F1B-4B4E-9E0A-EEEEEEEEEEE0", "width": 120, "height": 60,
        "activeLayerID": text_id,
        "guides": [ { "axis": "vertical", "id": "0B6C6B1E-4F1B-4B4E-9E0A-EEEEEEEEEEE4", "position": 30 } ],
        "futureDocumentKey": { "nested": [1, 2.5, "x"] },
        "layers": [
            { "id": folder_id, "name": "Folder", "isVisible": true, "isGroup": true, "opacity": 0.5,
              "transform": { "origin": [0, 0], "size": [120, 60], "rotation": 0, "flipX": false, "flipY": false, "sampling": "High quality" },
              "futureFolderKey": true },
            { "id": text_id, "name": "Titled", "isVisible": true, "imageFile": image_file,
              "transform": { "origin": [10, 10], "size": [60, 40], "rotation": 0, "flipX": false, "flipY": false, "sampling": "High quality" },
              "effects": { "shadow": { "angle": 90, "blue": 0, "blur": 20, "distance": 20, "green": 0, "opacity": 0.5, "red": 0 } },
              "text": { "alignment": "Left", "blue": 0, "content": "Hi", "fontName": "Helvetica", "fontSize": 72,
                        "green": 0, "leading": 0, "red": 0, "tracking": 0 },
              "futureLayerKey": 7 },
            { "id": adj_id, "name": "Blur", "isVisible": true,
              "transform": { "origin": [0, 0], "size": [120, 60], "rotation": 0, "flipX": false, "flipY": false, "sampling": "High quality" },
              "adjustment": { "kind": "Gaussian Blur", "hue": 0, "saturation": 0, "lightness": 0, "colorize": false,
                  "levels": { "channel": "RGB", "ranges": [ {"black":0,"gamma":1,"white":255,"outputBlack":0,"outputWhite":255},
                      {"black":0,"gamma":1,"white":255,"outputBlack":0,"outputWhite":255}, {"black":0,"gamma":1,"white":255,"outputBlack":0,"outputWhite":255},
                      {"black":0,"gamma":1,"white":255,"outputBlack":0,"outputWhite":255} ] },
                  "curves": { "channel": "RGB", "channels": [ [{"x":0,"y":0},{"x":255,"y":255}], [{"x":0,"y":0},{"x":255,"y":255}],
                      [{"x":0,"y":0},{"x":255,"y":255}], [{"x":0,"y":0},{"x":255,"y":255}] ] },
                  "blurRadius": 24 },
              "futureAdjLayerKey": "z" }
        ]
    }).to_string();
    let image = encode_png(&colourful_gradient(60, 40), 72.0).unwrap();
    let package = Package { manifest_json: manifest, images: vec![(image_file, image)] };

    let mut e = Engine::new();
    let id = e.open_package(&package, None).unwrap_or_else(|err| panic!("edited-rich-file.comp: does not open: {err:?}"));
    let text_uuid = uuid::Uuid::parse_str(text_id).unwrap();
    let adj_uuid = uuid::Uuid::parse_str(adj_id).unwrap();
    e.execute(id, Command::DuplicateLayer { id: text_uuid }).unwrap();
    let dup_id = e.state(id).unwrap().active_layer_id.unwrap();
    e.execute(id, Command::GroupLayers { ids: vec![dup_id, adj_uuid] }).unwrap();
    e.execute(id, Command::CanvasSize { width: 160, height: 80, anchor: 4, fill: None }).unwrap();
    e.execute(id, Command::FlipCanvas { horizontal: true }).unwrap();
    e.document(id).unwrap().clone()
}

/// Saves `doc` as `<dir>/<filename>/manifest.json` plus its `images/`, then re-opens the saved
/// package with `open_package` -- every probe must be openable by this build's own reader before
/// it is ever sent to a Mac.
fn write_probe(dir: &Path, filename: &str, doc: &Document) {
    let package = save_package(doc).unwrap_or_else(|e| panic!("{filename}: failed to save: {e:?}"));
    let comp_dir = dir.join(filename);
    let images_dir = comp_dir.join("images");
    fs::create_dir_all(&images_dir).unwrap_or_else(|e| panic!("{filename}: failed to create {images_dir:?}: {e}"));
    fs::write(comp_dir.join("manifest.json"), &package.manifest_json).unwrap_or_else(|e| panic!("{filename}: failed to write manifest.json: {e}"));
    for (name, bytes) in &package.images {
        fs::write(images_dir.join(name), bytes).unwrap_or_else(|e| panic!("{filename}: failed to write image {name}: {e}"));
    }
    open_package(&package).unwrap_or_else(|e| panic!("{filename}: does not re-open with open_package: {e:?}"));
}

#[test]
#[ignore]
fn write_mac_probes() {
    let dir = probes_dir();
    fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("failed to create {dir:?}: {e}"));

    write_probe(&dir, "folder-opacity.comp", &folder_opacity_doc());
    write_probe(&dir, "clipped-in-dimmed-folder.comp", &clipped_in_dimmed_folder_doc());
    write_probe(&dir, "guides.comp", &guides_doc());
    write_probe(&dir, "new-blend-modes.comp", &new_blend_modes_doc());
    write_probe(&dir, "new-adjustment-layers.comp", &new_adjustment_layers_doc());
    write_probe(&dir, "grain.comp", &grain_doc());
    write_probe(&dir, "edited-rich-file.comp", &edited_rich_file_doc());

    fs::write(dir.join("README.txt"), README_TXT).unwrap_or_else(|e| panic!("failed to write README.txt: {e}"));
    assert!(README_TXT.is_ascii(), "README.txt must be ASCII only");
}
