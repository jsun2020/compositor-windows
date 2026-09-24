use compositor_engine::*;

fn solid(name: &str, rgba: [u8; 4], x: f64) -> Layer {
    Layer::with_pixels(name, Raster::from_premultiplied(2, 1, rgba.repeat(2)), Point { x, y: 0.0 })
}
fn folder(doc: &Document, opacity: f64) -> Layer {
    let mut f = Layer::blank("Folder", doc.size()); f.is_group = true; f.opacity = opacity; f
}
fn px(doc: &Document, x: u32) -> [u8; 4] { composite(doc, Rect { x: 0.0, y: 0.0, width: 3.0, height: 1.0 }, 3, 1).pixel(x, 0) }
/// Within one level per channel: whether the compositor rounds to 8 bits between layers moves a
/// channel by at most one (for example alpha 191.25 vs 191.5), while every alternative these
/// tests rule out (Photoshop's group fade, ignoring the folder) differs by 63 or more in some channel.
fn near(a: [u8; 4], b: [u8; 4]) -> bool { a.iter().zip(b).all(|(x, y)| (*x as i16 - y as i16).abs() <= 1) }

#[test]
fn a_dimmed_folder_dims_each_child_so_overlapping_children_show_through() {
    // Red at x 0-1, blue at x 1-2, both inside a 50% folder. The Mac multiplies the folder's
    // opacity into each child (LayerGroups.swift:49-64, R 4.1); it never composites the folder as
    // a unit, so at x=1 the red shows through the blue. Photoshop's group fade would give (0,0,128,128).
    let mut doc = Document::new(3, 1);
    let f = folder(&doc, 0.5);
    let mut red = solid("Red", [255, 0, 0, 255], 0.0); red.parent_id = Some(f.id);
    let mut blue = solid("Blue", [0, 0, 255, 255], 1.0); blue.parent_id = Some(f.id);
    doc.layers = vec![f, red, blue];
    assert!(near(px(&doc, 1), [64, 0, 128, 191]), "{:?}", px(&doc, 1));
    assert!(near(px(&doc, 0), [128, 0, 0, 128]), "{:?}", px(&doc, 0));
}

#[test]
fn nested_folders_multiply() {
    let mut doc = Document::new(3, 1);
    let outer = folder(&doc, 0.5);
    let mut inner = folder(&doc, 0.5); inner.parent_id = Some(outer.id);
    let mut white = solid("W", [255, 255, 255, 255], 0.0); white.parent_id = Some(inner.id); white.opacity = 0.8;
    doc.layers = vec![outer, inner, white];
    assert!((px(&doc, 0)[3] as i16 - 51).abs() <= 1, "0.8 x 0.5 x 0.5 = 0.2: {:?}", px(&doc, 0));
}

#[test]
fn a_dimmed_folder_weakens_an_adjustment_inside_it() {
    // A Levels layer mapping everything to black, at full own opacity, inside a 25% folder, over a
    // white layer outside the folder: the adjustment reaches the white (folders are pass-through)
    // at 25% strength, 255 * 0.75 = 191.
    let mut doc = Document::new(3, 1);
    let white = solid("W", [255, 255, 255, 255], 0.0);
    let f = folder(&doc, 0.25);
    let mut adj = Layer::blank("Levels", doc.size());
    let mut a = LayerAdjustment::new(AdjustmentKind::Levels);
    a.levels.ranges[0].output_white = 0.0;
    adj.extra.adjustment = Some(a); adj.parent_id = Some(f.id);
    doc.layers = vec![white, f, adj];
    assert!((px(&doc, 0)[0] as i16 - 191).abs() <= 1, "{:?}", px(&doc, 0));
}

#[test]
fn a_clipped_child_in_a_dimmed_folder_is_dimmed_twice_as_on_the_mac() {
    // R 4.1, from reading LiveMaskRenderer.swift:78-111 (not yet confirmed by a Mac render; the
    // Task 7 probe asks the user to check): colour mix(B, C, c*f), alpha A*b*f. White base, red
    // child clipped to it, both in a 50% folder: colour (1, .5, .5), alpha .5, so premultiplied
    // (128, 64, 64, 128). Photoshop would give (128, 0, 0, 128).
    let mut doc = Document::new(3, 1);
    let f = folder(&doc, 0.5);
    let mut base = solid("Base", [255, 255, 255, 255], 0.0); base.parent_id = Some(f.id);
    let mut child = solid("Child", [255, 0, 0, 255], 0.0); child.parent_id = Some(f.id); child.mask_source_id = Some(base.id);
    doc.layers = vec![f, base, child];
    assert!(near(px(&doc, 0), [128, 64, 64, 128]), "{:?}", px(&doc, 0));
}

#[test]
fn folders_take_opacity_but_stay_normal() {
    let mut doc = Document::new(3, 1);
    let f = folder(&doc, 1.0); let id = f.id; doc.layers = vec![f];
    ops::appearance::set_opacity(&mut doc, id, 0.4).unwrap();
    assert_eq!(doc.layers[0].opacity, 0.4);
    assert!(ops::appearance::set_blend_mode(&mut doc, id, BlendMode::Multiply).is_err());
}

/// A folder record with an opacity of its own, in Task 1's `rich_manifest` style.
fn folder_package(version: u32) -> Package {
    let manifest = serde_json::json!({
        "format": "com.compositor.project", "version": version, "colorSpace": "sRGB",
        "documentID": "0B6C6B1E-4F1B-4B4E-9E0A-111111111111", "width": 40, "height": 30,
        "layers": [{ "id": "0B6C6B1E-4F1B-4B4E-9E0A-666666666666", "name": "Folder", "isVisible": true,
            "isGroup": true, "opacity": 0.5,
            "transform": { "origin": [0, 0], "size": [40, 30], "rotation": 0, "flipX": false, "flipY": false, "sampling": "High quality" } }]
    });
    Package { manifest_json: manifest.to_string(), images: vec![] }
}

#[test]
fn folder_opacity_needs_version_8_and_survives_a_re_save() {
    // ProjectStore.swift:212-216: before v8 a folder's opacity must be 1.
    assert!(matches!(open_package(&folder_package(7)), Err(ProjectError::Invalid)));
    let doc = open_package(&folder_package(8)).expect("v8 folders carry an opacity");
    let saved: serde_json::Value = serde_json::from_str(&save_package(&doc).unwrap().manifest_json).unwrap();
    assert_eq!(saved["layers"][0]["opacity"], serde_json::json!(0.5));
}
