# Compositor for Windows, Phase 1 (Canvas and Files) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A portable Windows build of Compositor that creates canvases, imports images, opens and saves macOS `.comp` packages, crops, resizes the canvas and the image, flips the canvas, zooms and pans, works in tabs, and exports PNG and JPEG.

**Architecture:** A pure Rust crate (`engine`) owns documents, pixels, history and codecs and is tested natively. A `wasm-bindgen` wrapper (`engine-wasm`) runs it inside the WebView. A Vite + React + TypeScript app (`app`) draws the document with WebGL2 (CPU fallback) and hosts the tools. A Tauri 2 shell (`src-tauri`) does windows, dialogs and file bytes only.

**Tech Stack:** Rust 1.95 stable, `wasm32-unknown-unknown`, wasm-pack, serde, serde_json, uuid, image 0.25, png 0.17, jpeg-encoder 0.6, thiserror; Node 22, pnpm, Vite 6, React 18, TypeScript 5, zustand, vitest, Playwright; Tauri 2 with the dialog plugin; PowerShell build script.

**Spec:** `docs/superpowers/specs/2026-09-20-windows-port-design.md`

## Global Constraints

- Limits, everywhere they apply: canvas and image sides 1 to 30,000 px; 100,000,000 source pixels per project plus 100,000,000 mask pixels; 10,000 layers; 64 nesting levels; manifest at most 4 MiB; each encoded asset at most 512 MiB; resolution 1 to 9600 pixels/inch, default 72; layer transform size 1 to 300,000, origin magnitude at most 1,000,000.
- Project format: folder `<name>.comp` with `manifest.json` and `images/<UUID>.png`, `images/<UUID>.mask.png`. Read versions 1 to 7, write 7. UUIDs uppercase. `origin` is `[x, y]`, `size` is `[width, height]`. Enum strings: sampling `"Nearest"`, `"Smooth"`, `"High quality"`; blend modes `"Normal"`, `"Multiply"`, `"Screen"`, `"Overlay"`, `"Darken"`, `"Lighten"`, `"Difference"`, `"Color Dodge"`, `"Color Burn"`, `"Hue"`, `"Saturation"`, `"Color"`, `"Luminosity"`. Absent optionals are omitted. Keys sorted, pretty-printed.
- Fields for later phases (`adjustment`, `shape`, `maskPlacement`, `maskLinked`, masks themselves) are preserved verbatim through open and save.
- Pixels in the engine are premultiplied RGBA8, top-left origin. Saved PNGs are straight (unpremultiplied) RGBA.
- Supported imports: PNG, JPEG, TIFF, WebP, BMP. Anything else is `Unsupported`. Damaged files are `Unreadable`.
- ASCII only in strings passed to shell commands and packaging tools (Windows encoding rule). Do not probe google hosts for connectivity.
- Phase 1 renders layers with Normal blend and opacity only. Blend modes, masks and clipping arrive in Phase 2; the compositor is structured so they slot in.
- Phase 1 stores each layer's pixels as one contiguous buffer behind a tile-shaped API (`tile_rgba`). Copy-on-write is at layer granularity. Phase 4 (painting) switches storage to 256 px tiles behind the same API.
- Interactive drag geometry (crop frame, snapping, zoom) lives in TypeScript with vitest tests; pixel and document operations live in Rust.
- Every commit compiles and passes `cargo test` and `pnpm test`.

---

## File Structure

```
compositor-windows/
  Cargo.toml                      workspace: engine, engine-wasm, src-tauri
  package.json                    pnpm scripts: dev, build, test, e2e, wasm, tauri
  vite.config.ts                  root = app/, define __APP_VERSION__
  vitest.config.ts
  playwright.config.ts
  engine/
    Cargo.toml
    src/lib.rs                    re-exports
    src/geometry.rs               Point, Size, Rect, Sampling, LayerTransform (Swift-compatible serde)
    src/ids.rs                    uuid serde helpers (uppercase)
    src/error.rs                  ProjectError, ImportError, ExportError, CommandError
    src/manifest.rs               Manifest, LayerRecord, BlendMode, validate()
    src/raster.rs                 Raster (RGBA8 premultiplied), GrayRaster, tile_rgba, halved
    src/codec.rs                  decode_image, encode_png, encode_jpeg, decode_package_png
    src/document.rs               Document, Layer, Mask, LayerExtra, DocumentState
    src/package.rs                Package, open_package, save_package
    src/compositor.rs             composite(), sampling
    src/history.rs                History (document snapshots), saved marker
    src/command.rs                Command enum, Dirty
    src/engine.rs                 Engine, Session, execute, undo/redo, import, export
    src/ops/mod.rs
    src/ops/canvas_size.rs        CanvasSizeOptions, canvas_size()
    src/ops/image_size.rs         ImageSizeOptions, image_size()
    src/ops/flip.rs               flip_canvas()
    src/ops/layers.rs             add_blank_layer, rename, visibility, delete, import placement
    tests/fixtures.rs             shared fixture builders (64x32 red-left PNG etc.)
    tests/*.rs                    integration tests per module
  engine-wasm/
    Cargo.toml
    src/lib.rs                    WasmEngine
  app/
    index.html
    src/main.tsx
    src/App.tsx
    src/build-info.ts             generated by scripts; BUILD_MARKER literal
    src/engine/client.ts          EngineClient (typed wasm wrapper)
    src/engine/types.ts           DocumentState, Command, Dirty
    src/state/store.ts            zustand store: documents, active, tool, viewport, crop
    src/canvas/viewport.ts        Viewport (port of CanvasViewport)
    src/canvas/gl-renderer.ts     WebGL2 renderer
    src/canvas/cpu-renderer.ts    fallback renderer
    src/canvas/layer-textures.ts  chunked textures per layer
    src/canvas/CanvasView.tsx     canvas element, pointer handling, overlay canvas
    src/canvas/overlay.ts         checkerboard is GL; pixel grid + crop frame drawn on 2D overlay
    src/tools/crop-geometry.ts    CropGeometry, CropDrag, CropSnap (ports)
    src/tools/canvas-size-draft.ts CanvasSizeDraft (port)
    src/shell/bridge.ts           ShellBridge interface + chooser
    src/shell/tauri-bridge.ts
    src/shell/mock-bridge.ts
    src/panels/ToolRail.tsx
    src/panels/LayersList.tsx
    src/panels/ProjectTabs.tsx
    src/panels/MenuBar.tsx
    src/sheets/NewCanvasSheet.tsx
    src/sheets/CanvasSizeSheet.tsx
    src/sheets/ImageSizeSheet.tsx
    src/sheets/JpegExportSheet.tsx
    src/shortcuts/keymap.ts
    src/test-api.ts               window.__compositor hooks (dev/test only)
    tests/unit/*.test.ts          vitest
    tests/e2e/*.spec.ts           Playwright, mock bridge
  src-tauri/
    Cargo.toml
    tauri.conf.json
    capabilities/default.json
    build.rs
    src/main.rs
    src/lib.rs                    run(), plugin + command registration
    src/commands/package.rs       read_package_manifest, read_package_image, write_package_* , commit
    src/commands/files.rs         read_file, write_file
    src/commands/recent.rs        recent_packages, add_recent_package
    src/atomic.rs                 staged directory swap
    icons/                        generated by `pnpm tauri icon`
  scripts/
    build-windows-x64.ps1
    write-build-info.ps1
  docs/
    superpowers/specs, plans
    README.md
```

---

### Task 1: Workspace and engine geometry

**Files:**
- Create: `Cargo.toml`, `engine/Cargo.toml`, `engine/src/lib.rs`, `engine/src/geometry.rs`, `engine/src/ids.rs`, `engine/src/error.rs`
- Test: `engine/tests/geometry.rs`

**Interfaces:**
- Produces: `Point {x,y}`, `Size {width,height}`, `Rect {x,y,width,height}`, `Sampling {Nearest, Smooth, High}`, `LayerTransform {origin, size, rotation, flip_x, flip_y, sampling}` with `center()`, `radians()`, `point(unit: Point) -> Point`, `corners() -> [Point;4]`, `is_valid()`, `mirrored(horizontal: bool, axis: f64)`, `pixel_to_document(width, height) -> Affine`, `Affine {a,b,c,d,tx,ty}` with `apply`, `invert`; `ProjectError`, `ImportError`, `ExportError`, `CommandError`; `ids::upper` serde module.

- [ ] **Step 1: Create the workspace and crate**

`Cargo.toml` (repo root):
```toml
[workspace]
resolver = "2"
members = ["engine", "engine-wasm", "src-tauri"]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
```

`engine/Cargo.toml`:
```toml
[package]
name = "compositor-engine"
version.workspace = true
edition.workspace = true

[lib]
name = "compositor_engine"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4", "serde", "js"] }
thiserror = "2"
image = { version = "0.25", default-features = false, features = ["png", "jpeg", "tiff", "webp", "bmp"] }
png = "0.17"
jpeg-encoder = "0.6"

[dev-dependencies]
```

Until `engine-wasm` and `src-tauri` exist (Tasks 10 and 13), keep `members = ["engine"]` and extend the list when those crates are added.

`engine/src/lib.rs`:
```rust
pub mod error;
pub mod geometry;
pub mod ids;

pub use error::*;
pub use geometry::*;
```

`engine/src/error.rs`:
```rust
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum ProjectError {
    #[error("This is not a valid Compositor project, or its metadata is damaged.")]
    Invalid,
    #[error("This project uses format version {0}. This app supports versions 1-7.")]
    Version(u32),
    #[error("An image inside the project is missing or damaged. The current document has not been replaced.")]
    MissingImage,
    #[error("This project exceeds the supported canvas, layer, file-size, or 100-megapixel image limit.")]
    TooLarge,
    #[error("An image could not be saved. The previous project has not been replaced.")]
    Encode,
}

#[derive(Debug, Error, Clone, PartialEq)]
pub enum ImportError {
    #[error("The image could not be read. It may be damaged or unavailable.")]
    Unreadable,
    #[error("Choose a JPEG, PNG, TIFF, WebP, or BMP image.")]
    Unsupported,
    #[error("This import exceeds the current 100-megapixel document budget or 30,000-pixel side limit.")]
    TooLarge,
}

#[derive(Debug, Error, Clone, PartialEq)]
pub enum ExportError {
    #[error("Image export supports canvases up to 100 megapixels and 30,000 pixels per side.")]
    TooLarge,
    #[error("The canvas could not be rendered. Try a smaller canvas.")]
    Render,
    #[error("The image could not be encoded.")]
    Encode,
}

#[derive(Debug, Error, Clone, PartialEq)]
pub enum CommandError {
    #[error("No document with that id.")]
    NoDocument,
    #[error("No layer with that id.")]
    NoLayer,
    #[error("{0}")]
    Project(#[from] ProjectError),
    #[error("{0}")]
    Import(#[from] ImportError),
    #[error("{0}")]
    Export(#[from] ExportError),
    #[error("Invalid argument: {0}")]
    Argument(String),
}
```

`engine/src/ids.rs`:
```rust
//! UUID serde helpers. Swift writes uppercase hyphenated UUIDs; we accept any case and write uppercase.
pub mod upper {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use uuid::Uuid;

    pub fn serialize<S: Serializer>(id: &Uuid, s: S) -> Result<S::Ok, S::Error> {
        id.as_hyphenated().to_string().to_uppercase().serialize(s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Uuid, D::Error> {
        let text = String::deserialize(d)?;
        Uuid::parse_str(&text).map_err(serde::de::Error::custom)
    }
}

pub mod upper_opt {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use uuid::Uuid;

    pub fn serialize<S: Serializer>(id: &Option<Uuid>, s: S) -> Result<S::Ok, S::Error> {
        match id {
            Some(id) => id.as_hyphenated().to_string().to_uppercase().serialize(s),
            None => s.serialize_none(),
        }
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Uuid>, D::Error> {
        let text: Option<String> = Option::deserialize(d)?;
        match text {
            Some(t) => Uuid::parse_str(&t).map(Some).map_err(serde::de::Error::custom),
            None => Ok(None),
        }
    }
}

pub fn upper_string(id: &uuid::Uuid) -> String {
    id.as_hyphenated().to_string().to_uppercase()
}
```

- [ ] **Step 2: Write the failing geometry tests**

`engine/tests/geometry.rs`:
```rust
use compositor_engine::*;

fn transform() -> LayerTransform {
    LayerTransform { origin: Point { x: 10.0, y: 20.0 }, size: Size { width: 100.0, height: 50.0 },
        rotation: 0.0, flip_x: false, flip_y: false, sampling: Sampling::High }
}

#[test]
fn serializes_like_swift() {
    let json = serde_json::to_value(transform()).unwrap();
    assert_eq!(json["origin"], serde_json::json!([10.0, 20.0]));
    assert_eq!(json["size"], serde_json::json!([100.0, 50.0]));
    assert_eq!(json["sampling"], "High quality");
    assert_eq!(json["flipX"], false);
    let back: LayerTransform = serde_json::from_value(json).unwrap();
    assert_eq!(back, transform());
}

#[test]
fn deserializes_defaults_for_missing_fields() {
    let t: LayerTransform = serde_json::from_str(r#"{"origin":[1,2],"size":[3,4]}"#).unwrap();
    assert_eq!(t.rotation, 0.0);
    assert_eq!(t.sampling, Sampling::High);
    assert!(!t.flip_x && !t.flip_y);
}

#[test]
fn point_rotates_clockwise_around_center() {
    let mut t = transform();
    t.rotation = 90.0;
    let c = t.center();
    assert_eq!(c, Point { x: 60.0, y: 45.0 });
    // The top-left unit corner (-50,-25 from center) turns to (25,-50): up and right of center.
    let p = t.point(Point { x: 0.0, y: 0.0 });
    assert!((p.x - 85.0).abs() < 1e-9 && (p.y - -5.0).abs() < 1e-9);
}

#[test]
fn validity_limits() {
    let mut t = transform();
    assert!(t.is_valid());
    t.size.width = 0.5;
    assert!(!t.is_valid());
    t.size.width = 300_001.0;
    assert!(!t.is_valid());
    t.size.width = 10.0;
    t.origin.x = 1_000_001.0;
    assert!(!t.is_valid());
    t.origin.x = f64::NAN;
    assert!(!t.is_valid());
}

#[test]
fn mirrored_flips_across_axis() {
    let mut t = transform();
    t.rotation = 30.0;
    let m = t.mirrored(true, 200.0);
    assert!(m.flip_x && !m.flip_y);
    assert_eq!(m.rotation, -30.0);
    // Center x 60 crosses to 340; origin = 340 - 50.
    assert!((m.origin.x - 290.0).abs() < 1e-9);
    assert_eq!(m.origin.y, 20.0);
}

#[test]
fn pixel_to_document_and_inverse_round_trip() {
    let mut t = transform();
    t.rotation = 38.0;
    t.flip_x = true;
    let m = t.pixel_to_document(64, 32);
    let inv = m.invert().unwrap();
    let doc = m.apply(Point { x: 5.0, y: 7.0 });
    let back = inv.apply(doc);
    assert!((back.x - 5.0).abs() < 1e-9 && (back.y - 7.0).abs() < 1e-9);
    // Pixel center of the image maps to the transform center.
    let c = m.apply(Point { x: 32.0, y: 16.0 });
    assert!((c.x - 60.0).abs() < 1e-9 && (c.y - 45.0).abs() < 1e-9);
}
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test -p compositor-engine --test geometry`
Expected: compile error, `geometry` items not found.

- [ ] **Step 4: Implement geometry**

`engine/src/geometry.rs`:
```rust
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(from = "(f64, f64)", into = "(f64, f64)")]
pub struct Point { pub x: f64, pub y: f64 }
impl From<(f64, f64)> for Point { fn from((x, y): (f64, f64)) -> Self { Point { x, y } } }
impl From<Point> for (f64, f64) { fn from(p: Point) -> Self { (p.x, p.y) } }

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(from = "(f64, f64)", into = "(f64, f64)")]
pub struct Size { pub width: f64, pub height: f64 }
impl From<(f64, f64)> for Size { fn from((width, height): (f64, f64)) -> Self { Size { width, height } } }
impl From<Size> for (f64, f64) { fn from(s: Size) -> Self { (s.width, s.height) } }

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rect { pub x: f64, pub y: f64, pub width: f64, pub height: f64 }
impl Rect {
    pub fn max_x(&self) -> f64 { self.x + self.width }
    pub fn max_y(&self) -> f64 { self.y + self.height }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Sampling {
    #[serde(rename = "Nearest")] Nearest,
    #[serde(rename = "Smooth")] Smooth,
    #[default]
    #[serde(rename = "High quality")] High,
}

/// Unrotated bounds in document pixels; rotation is clockwise (y down) around their center.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct LayerTransform {
    pub origin: Point,
    pub size: Size,
    #[serde(default)] pub rotation: f64,
    #[serde(default, rename = "flipX")] pub flip_x: bool,
    #[serde(default, rename = "flipY")] pub flip_y: bool,
    #[serde(default)] pub sampling: Sampling,
}

impl LayerTransform {
    pub fn axis_aligned(origin: Point, size: Size) -> Self {
        LayerTransform { origin, size, rotation: 0.0, flip_x: false, flip_y: false, sampling: Sampling::High }
    }
    pub fn center(&self) -> Point {
        Point { x: self.origin.x + self.size.width / 2.0, y: self.origin.y + self.size.height / 2.0 }
    }
    pub fn radians(&self) -> f64 { (self.rotation % 360.0) * std::f64::consts::PI / 180.0 }
    pub fn is_valid(&self) -> bool {
        [self.origin.x, self.origin.y, self.size.width, self.size.height, self.rotation].iter().all(|v| v.is_finite())
            && (1.0..=300_000.0).contains(&self.size.width) && (1.0..=300_000.0).contains(&self.size.height)
            && self.origin.x.abs() <= 1_000_000.0 && self.origin.y.abs() <= 1_000_000.0
    }
    /// Unit square (0..1, y down) point placed on the document.
    pub fn point(&self, unit: Point) -> Point {
        let x = (unit.x - 0.5) * self.size.width;
        let y = (unit.y - 0.5) * self.size.height;
        let (s, c) = self.radians().sin_cos();
        let center = self.center();
        Point { x: center.x + x * c - y * s, y: center.y + x * s + y * c }
    }
    pub fn corners(&self) -> [Point; 4] {
        [self.point(Point { x: 0.0, y: 0.0 }), self.point(Point { x: 1.0, y: 0.0 }),
         self.point(Point { x: 1.0, y: 1.0 }), self.point(Point { x: 0.0, y: 1.0 })]
    }
    /// Upright bounding box of the rotated rectangle.
    pub fn bounds(&self) -> Rect {
        let c = self.corners();
        let min_x = c.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
        let max_x = c.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
        let min_y = c.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
        let max_y = c.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
        Rect { x: min_x, y: min_y, width: max_x - min_x, height: max_y - min_y }
    }
    /// Mirrored across a vertical line at `axis` (horizontal = true) or a horizontal one.
    pub fn mirrored(&self, horizontal: bool, axis: f64) -> Self {
        let mut r = *self;
        let center = self.center();
        if horizontal {
            r.flip_x = !r.flip_x;
            r.origin.x = 2.0 * axis - center.x - self.size.width / 2.0;
        } else {
            r.flip_y = !r.flip_y;
            r.origin.y = 2.0 * axis - center.y - self.size.height / 2.0;
        }
        r.rotation = -self.rotation;
        r
    }
    /// Maps layer pixel coordinates (0..width, 0..height, y down) to document coordinates.
    pub fn pixel_to_document(&self, width: u32, height: u32) -> Affine {
        let center = self.center();
        let sx = self.size.width / width.max(1) as f64 * if self.flip_x { -1.0 } else { 1.0 };
        let sy = self.size.height / height.max(1) as f64 * if self.flip_y { -1.0 } else { 1.0 };
        Affine::translation(center.x, center.y)
            .then(Affine::rotation(self.radians()))
            .then(Affine::scale(sx, sy))
            .then(Affine::translation(-(width as f64) / 2.0, -(height as f64) / 2.0))
    }
}

/// Row-vector affine map: x' = a*x + c*y + tx, y' = b*x + d*y + ty (CGAffineTransform layout).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine { pub a: f64, pub b: f64, pub c: f64, pub d: f64, pub tx: f64, pub ty: f64 }

impl Affine {
    pub const IDENTITY: Affine = Affine { a: 1.0, b: 0.0, c: 0.0, d: 1.0, tx: 0.0, ty: 0.0 };
    pub fn translation(tx: f64, ty: f64) -> Self { Affine { tx, ty, ..Self::IDENTITY } }
    pub fn scale(sx: f64, sy: f64) -> Self { Affine { a: sx, d: sy, ..Self::IDENTITY } }
    pub fn rotation(radians: f64) -> Self {
        let (s, c) = radians.sin_cos();
        Affine { a: c, b: s, c: -s, d: c, tx: 0.0, ty: 0.0 }
    }
    /// `self` applied after `inner`: matches CGAffineTransform's `.rotated(by:)`, `.scaledBy` chaining,
    /// where each call pre-multiplies, so `then(inner)` applies `inner` to points first.
    pub fn then(self, inner: Affine) -> Affine {
        Affine {
            a: inner.a * self.a + inner.b * self.c,
            b: inner.a * self.b + inner.b * self.d,
            c: inner.c * self.a + inner.d * self.c,
            d: inner.c * self.b + inner.d * self.d,
            tx: inner.tx * self.a + inner.ty * self.c + self.tx,
            ty: inner.tx * self.b + inner.ty * self.d + self.ty,
        }
    }
    pub fn apply(&self, p: Point) -> Point {
        Point { x: self.a * p.x + self.c * p.y + self.tx, y: self.b * p.x + self.d * p.y + self.ty }
    }
    pub fn invert(&self) -> Option<Affine> {
        let det = self.a * self.d - self.b * self.c;
        if det.abs() < 1e-12 || !det.is_finite() { return None; }
        let a = self.d / det; let b = -self.b / det; let c = -self.c / det; let d = self.a / det;
        Some(Affine { a, b, c, d, tx: -(a * self.tx + c * self.ty), ty: -(b * self.tx + d * self.ty) })
    }
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p compositor-engine --test geometry`
Expected: 6 passed.

- [ ] **Step 6: Commit**

```
git add Cargo.toml engine
git commit -m "feat(engine): workspace, geometry and Swift-compatible transform serde"
```

---

### Task 2: Manifest model and validation

**Files:**
- Create: `engine/src/manifest.rs`
- Modify: `engine/src/lib.rs` (add `pub mod manifest; pub use manifest::*;`)
- Test: `engine/tests/manifest.rs`

**Interfaces:**
- Produces: `BlendMode` enum (13 variants, display-string serde); `Manifest { format, version, color_space, resolution: Option<f64>, document_id, width: i64, height: i64, active_layer_id: Option<Uuid>, layers: Vec<LayerRecord> }`; `LayerRecord { id, name, is_visible, transform, image_file, parent_id, is_group, opacity, blend_mode, mask_file, mask_enabled, mask_source_id, adjustment: Option<serde_json::Value>, mask_placement, mask_linked, shape: Option<serde_json::Value> }`; `Manifest::validate(&self) -> Result<(), ProjectError>`; `Manifest::parse(json: &str) -> Result<Manifest, ProjectError>` (header check then full decode then validate); `Manifest::to_json_pretty(&self) -> Result<String, ProjectError>` (sorted keys, size check); `LayerRecord::image_filename(id)`, `LayerRecord::mask_filename(id)`; constants `MANIFEST_FORMAT`, `CURRENT_VERSION = 7`, `MAX_SIDE = 30_000`, `MAX_PIXELS = 100_000_000`, `MAX_LAYERS = 10_000`, `MAX_MANIFEST_BYTES = 4 MiB`, `MAX_ASSET_BYTES = 512 MiB`.

- [ ] **Step 1: Write the failing tests**

`engine/tests/manifest.rs`:
```rust
use compositor_engine::*;
use uuid::Uuid;

fn record(id: Uuid) -> LayerRecord {
    LayerRecord::new(id, "Layer 1", LayerTransform::axis_aligned(Point { x: 0.0, y: 0.0 }, Size { width: 100.0, height: 80.0 }), None)
}

fn manifest() -> Manifest {
    let id = Uuid::new_v4();
    Manifest::new(Uuid::new_v4(), 100, 80, Some(id), vec![record(id)])
}

#[test]
fn round_trips_json_with_swift_shapes() {
    let m = manifest();
    let json = m.to_json_pretty().unwrap();
    assert!(json.contains("\"format\": \"com.compositor.project\""));
    assert!(json.contains("\"version\": 7"));
    assert!(json.contains("\"colorSpace\": \"sRGB\""));
    assert!(!json.contains("\"parentID\""), "absent optionals are omitted");
    let upper = ids::upper_string(&m.layers[0].id);
    assert!(json.contains(&upper) && upper == upper.to_uppercase());
    let back = Manifest::parse(&json).unwrap();
    assert_eq!(back, m);
}

#[test]
fn future_version_is_rejected_with_its_number() {
    let mut m = manifest();
    m.version = 42;
    let json = serde_json::to_string(&m).unwrap();
    assert_eq!(Manifest::parse(&json), Err(ProjectError::Version(42)));
}

#[test]
fn corrupt_and_wrong_format_are_invalid() {
    assert_eq!(Manifest::parse("not json"), Err(ProjectError::Invalid));
    let mut m = manifest();
    m.format = "com.other".into();
    assert_eq!(Manifest::parse(&serde_json::to_string(&m).unwrap()), Err(ProjectError::Invalid));
}

#[test]
fn image_file_must_be_the_layer_uuid_png() {
    let mut m = manifest();
    m.layers[0].image_file = Some("../../outside.png".into());
    assert_eq!(m.validate(), Err(ProjectError::Invalid));
    m.layers[0].image_file = Some(format!("{}.png", ids::upper_string(&m.layers[0].id)));
    assert_eq!(m.validate(), Ok(()));
}

#[test]
fn limits_are_too_large() {
    let mut m = manifest();
    m.width = 30_001;
    assert_eq!(m.validate(), Err(ProjectError::TooLarge));
    m.width = 100;
    m.resolution = Some(9601.0);
    assert_eq!(m.validate(), Err(ProjectError::Invalid));
}

#[test]
fn version_gates_optional_features() {
    let mut m = manifest();
    m.version = 2;
    m.layers[0].opacity = Some(0.5);
    assert_eq!(m.validate(), Err(ProjectError::Invalid), "opacity needs version 3");
    m.version = 3;
    assert_eq!(m.validate(), Ok(()));
    m.layers[0].mask_file = Some(format!("{}.mask.png", ids::upper_string(&m.layers[0].id)));
    assert_eq!(m.validate(), Err(ProjectError::Invalid), "layer masks need version 4");
    m.version = 4;
    assert_eq!(m.validate(), Ok(()));
    m.layers[0].mask_file = Some("wrong.png".into());
    assert_eq!(m.validate(), Err(ProjectError::Invalid));
}

#[test]
fn hierarchy_rules() {
    let mut m = manifest();
    let group = Uuid::new_v4();
    let mut g = record(group);
    g.is_group = Some(true);
    m.layers[0].parent_id = Some(group);
    m.layers.push(g);
    m.version = 2;
    assert_eq!(m.validate(), Ok(()));
    m.version = 1;
    assert_eq!(m.validate(), Err(ProjectError::Invalid), "groups need version 2");
    m.version = 2;
    m.layers[1].parent_id = Some(m.layers[0].id);
    assert_eq!(m.validate(), Err(ProjectError::Invalid), "parent must be a group");
    m.layers[1].parent_id = Some(group);
    assert_eq!(m.validate(), Err(ProjectError::Invalid), "cycle");
    m.layers[1].parent_id = None;
    m.layers[1].image_file = Some(format!("{}.png", ids::upper_string(&group)));
    assert_eq!(m.validate(), Err(ProjectError::Invalid), "groups carry no image");
}

#[test]
fn clipping_mask_rules() {
    let mut m = manifest();
    let other = Uuid::new_v4();
    m.layers.push(record(other));
    m.layers[0].mask_source_id = Some(other);
    m.version = 4;
    assert_eq!(m.validate(), Err(ProjectError::Invalid), "needs version 5");
    m.version = 5;
    assert_eq!(m.validate(), Ok(()));
    m.layers[1].mask_source_id = Some(m.layers[0].id);
    assert_eq!(m.validate(), Err(ProjectError::Invalid), "cycle");
    m.layers[1].mask_source_id = None;
    m.layers[0].mask_source_id = Some(Uuid::new_v4());
    assert_eq!(m.validate(), Err(ProjectError::Invalid), "missing source");
}

#[test]
fn active_layer_must_exist_and_ids_unique() {
    let mut m = manifest();
    m.active_layer_id = Some(Uuid::new_v4());
    assert_eq!(m.validate(), Err(ProjectError::Invalid));
    m.active_layer_id = None;
    let dup = m.layers[0].clone();
    m.layers.push(dup);
    assert_eq!(m.validate(), Err(ProjectError::Invalid));
}

#[test]
fn unknown_later_phase_fields_survive() {
    let mut m = manifest();
    m.layers[0].adjustment = Some(serde_json::json!({"kind": "Levels", "levels": {"black": 0}}));
    let json = m.to_json_pretty().unwrap();
    let back = Manifest::parse(&json).unwrap();
    assert_eq!(back.layers[0].adjustment, m.layers[0].adjustment);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p compositor-engine --test manifest`
Expected: compile error, `manifest` items missing.

- [ ] **Step 3: Implement the manifest**

`engine/src/manifest.rs`:
```rust
use crate::{ids, LayerTransform, ProjectError};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

pub const MANIFEST_FORMAT: &str = "com.compositor.project";
pub const CURRENT_VERSION: u32 = 7;
pub const MAX_SIDE: i64 = 30_000;
pub const MAX_PIXELS: u64 = 100_000_000;
pub const MAX_LAYERS: usize = 10_000;
pub const MAX_NESTING: usize = 64;
pub const MAX_MANIFEST_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_ASSET_BYTES: u64 = 512 * 1024 * 1024;
pub const DEFAULT_RESOLUTION: f64 = 72.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BlendMode {
    #[default]
    #[serde(rename = "Normal")] Normal,
    #[serde(rename = "Multiply")] Multiply,
    #[serde(rename = "Screen")] Screen,
    #[serde(rename = "Overlay")] Overlay,
    #[serde(rename = "Darken")] Darken,
    #[serde(rename = "Lighten")] Lighten,
    #[serde(rename = "Difference")] Difference,
    #[serde(rename = "Color Dodge")] ColorDodge,
    #[serde(rename = "Color Burn")] ColorBurn,
    #[serde(rename = "Hue")] Hue,
    #[serde(rename = "Saturation")] Saturation,
    #[serde(rename = "Color")] Color,
    #[serde(rename = "Luminosity")] Luminosity,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LayerRecord {
    #[serde(with = "ids::upper")] pub id: Uuid,
    pub name: String,
    #[serde(rename = "isVisible")] pub is_visible: bool,
    pub transform: LayerTransform,
    #[serde(rename = "imageFile", default, skip_serializing_if = "Option::is_none")] pub image_file: Option<String>,
    #[serde(rename = "parentID", default, with = "ids::upper_opt", skip_serializing_if = "Option::is_none")] pub parent_id: Option<Uuid>,
    #[serde(rename = "isGroup", default, skip_serializing_if = "Option::is_none")] pub is_group: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub opacity: Option<f64>,
    #[serde(rename = "blendMode", default, skip_serializing_if = "Option::is_none")] pub blend_mode: Option<BlendMode>,
    #[serde(rename = "maskFile", default, skip_serializing_if = "Option::is_none")] pub mask_file: Option<String>,
    #[serde(rename = "maskEnabled", default, skip_serializing_if = "Option::is_none")] pub mask_enabled: Option<bool>,
    #[serde(rename = "maskSourceID", default, with = "ids::upper_opt", skip_serializing_if = "Option::is_none")] pub mask_source_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub adjustment: Option<serde_json::Value>,
    #[serde(rename = "maskPlacement", default, skip_serializing_if = "Option::is_none")] pub mask_placement: Option<LayerTransform>,
    #[serde(rename = "maskLinked", default, skip_serializing_if = "Option::is_none")] pub mask_linked: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub shape: Option<serde_json::Value>,
}

impl LayerRecord {
    pub fn new(id: Uuid, name: &str, transform: LayerTransform, image_file: Option<String>) -> Self {
        LayerRecord { id, name: name.to_string(), is_visible: true, transform, image_file, parent_id: None,
            is_group: None, opacity: None, blend_mode: None, mask_file: None, mask_enabled: None,
            mask_source_id: None, adjustment: None, mask_placement: None, mask_linked: None, shape: None }
    }
    pub fn image_filename(id: &Uuid) -> String { format!("{}.png", ids::upper_string(id)) }
    pub fn mask_filename(id: &Uuid) -> String { format!("{}.mask.png", ids::upper_string(id)) }
    pub fn is_group(&self) -> bool { self.is_group == Some(true) }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub format: String,
    pub version: u32,
    #[serde(rename = "colorSpace")] pub color_space: String,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub resolution: Option<f64>,
    #[serde(rename = "documentID", with = "ids::upper")] pub document_id: Uuid,
    pub width: i64,
    pub height: i64,
    #[serde(rename = "activeLayerID", default, with = "ids::upper_opt", skip_serializing_if = "Option::is_none")] pub active_layer_id: Option<Uuid>,
    pub layers: Vec<LayerRecord>,
}

#[derive(Deserialize)]
struct Header { format: String, version: u32 }

impl Manifest {
    pub fn new(document_id: Uuid, width: i64, height: i64, active_layer_id: Option<Uuid>, layers: Vec<LayerRecord>) -> Self {
        Manifest { format: MANIFEST_FORMAT.into(), version: CURRENT_VERSION, color_space: "sRGB".into(),
            resolution: None, document_id, width, height, active_layer_id, layers }
    }

    /// Header check first so an unsupported version reports its number, then full decode, then validation.
    pub fn parse(json: &str) -> Result<Manifest, ProjectError> {
        if json.len() > MAX_MANIFEST_BYTES { return Err(ProjectError::TooLarge); }
        let header: Header = serde_json::from_str(json).map_err(|_| ProjectError::Invalid)?;
        if header.format != MANIFEST_FORMAT { return Err(ProjectError::Invalid); }
        if !(1..=CURRENT_VERSION).contains(&header.version) { return Err(ProjectError::Version(header.version)); }
        let manifest: Manifest = serde_json::from_str(json).map_err(|_| ProjectError::Invalid)?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Sorted keys, pretty printed, size-checked.
    pub fn to_json_pretty(&self) -> Result<String, ProjectError> {
        self.validate()?;
        let value = serde_json::to_value(self).map_err(|_| ProjectError::Encode)?;
        let text = serde_json::to_string_pretty(&value).map_err(|_| ProjectError::Encode)?;
        if text.len() > MAX_MANIFEST_BYTES { return Err(ProjectError::TooLarge); }
        Ok(text)
    }

    pub fn validate(&self) -> Result<(), ProjectError> {
        use ProjectError::*;
        if self.format != MANIFEST_FORMAT { return Err(Invalid); }
        if !(1..=CURRENT_VERSION).contains(&self.version) { return Err(Version(self.version)); }
        if self.color_space != "sRGB" { return Err(Invalid); }
        if let Some(r) = self.resolution {
            if !r.is_finite() || !(1.0..=9600.0).contains(&r) { return Err(Invalid); }
        }
        if !(1..=MAX_SIDE).contains(&self.width) || !(1..=MAX_SIDE).contains(&self.height) || self.layers.len() > MAX_LAYERS {
            return Err(TooLarge);
        }
        for layer in &self.layers {
            if let Some(adj) = &layer.adjustment {
                if self.version < 7 || layer.is_group() || layer.image_file.is_some() || !adj.is_object() { return Err(Invalid); }
            }
            if let Some(mask) = &layer.mask_file {
                let needed = if layer.is_group() { 6 } else { 4 };
                if self.version < needed || *mask != LayerRecord::mask_filename(&layer.id) { return Err(Invalid); }
            }
            if layer.mask_enabled.is_some() && layer.mask_file.is_none() { return Err(Invalid); }
            if let Some(p) = &layer.mask_placement {
                if !p.is_valid() || layer.mask_file.is_none() { return Err(Invalid); }
            }
            let opacity = layer.opacity.unwrap_or(1.0);
            let blend = layer.blend_mode.unwrap_or_default();
            if !opacity.is_finite() || !(0.0..=1.0).contains(&opacity) { return Err(Invalid); }
            if self.version < 3 && (opacity != 1.0 || blend != BlendMode::Normal) { return Err(Invalid); }
            if layer.is_group() && (opacity != 1.0 || blend != BlendMode::Normal) { return Err(Invalid); }
        }
        validate_hierarchy(&self.layers)?;
        validate_clipping(&self.layers)?;
        if self.version < 5 && self.layers.iter().any(|l| l.mask_source_id.is_some()) { return Err(Invalid); }
        if self.version == 1 && self.layers.iter().any(|l| l.parent_id.is_some() || l.is_group()) { return Err(Invalid); }
        let mut ids = HashSet::new();
        for layer in &self.layers {
            if !ids.insert(layer.id) || !layer.transform.is_valid() || layer.name.trim().is_empty() || layer.name.len() > 16_384 {
                return Err(Invalid);
            }
            if let Some(file) = &layer.image_file {
                if *file != LayerRecord::image_filename(&layer.id) { return Err(Invalid); }
            }
        }
        if let Some(active) = self.active_layer_id {
            if !ids.contains(&active) { return Err(Invalid); }
        }
        Ok(())
    }
}

pub fn validate_hierarchy(layers: &[LayerRecord]) -> Result<(), ProjectError> {
    let mut by_id: HashMap<Uuid, &LayerRecord> = HashMap::new();
    for layer in layers {
        if by_id.insert(layer.id, layer).is_some() || (layer.is_group() && layer.image_file.is_some()) {
            return Err(ProjectError::Invalid);
        }
    }
    for layer in layers {
        let mut seen = HashSet::from([layer.id]);
        let mut parent = layer.parent_id;
        while let Some(id) = parent {
            if seen.len() > MAX_NESTING || !seen.insert(id) { return Err(ProjectError::Invalid); }
            let node = by_id.get(&id).ok_or(ProjectError::Invalid)?;
            if !node.is_group() { return Err(ProjectError::Invalid); }
            parent = node.parent_id;
        }
        if layer.is_group() && seen.len() > MAX_NESTING { return Err(ProjectError::Invalid); }
    }
    Ok(())
}

pub fn validate_clipping(layers: &[LayerRecord]) -> Result<(), ProjectError> {
    let mut records: HashMap<Uuid, &LayerRecord> = HashMap::new();
    for layer in layers {
        if records.insert(layer.id, layer).is_some() { return Err(ProjectError::Invalid); }
    }
    for layer in layers {
        let mut path = HashSet::new();
        let mut current = Some(layer.id);
        while let Some(id) = current {
            if path.len() >= 256 || !path.insert(id) { return Err(ProjectError::Invalid); }
            let record = records.get(&id).ok_or(ProjectError::Invalid)?;
            if let Some(source) = record.mask_source_id {
                if record.is_group() { return Err(ProjectError::Invalid); }
                let src = records.get(&source).ok_or(ProjectError::Invalid)?;
                if src.is_group() || src.adjustment.is_some() { return Err(ProjectError::Invalid); }
            }
            current = record.mask_source_id;
        }
    }
    Ok(())
}

/// Layers whose every ancestor is visible, in manifest order. Groups are included.
pub fn visible_layers(layers: &[LayerRecord]) -> Vec<&LayerRecord> {
    let by_id: HashMap<Uuid, &LayerRecord> = layers.iter().map(|l| (l.id, l)).collect();
    layers.iter().filter(|layer| {
        let mut node = Some(*layer);
        let mut steps = 0;
        while let Some(n) = node {
            if !n.is_visible { return false; }
            steps += 1;
            if steps > MAX_NESTING + 1 { return false; }
            node = n.parent_id.and_then(|p| by_id.get(&p).copied());
        }
        true
    }).collect()
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p compositor-engine --test manifest`
Expected: 10 passed.

- [ ] **Step 5: Commit**

```
git add engine
git commit -m "feat(engine): manifest model with macOS validation rules"
```

---

### Task 3: Raster storage and image codecs

**Files:**
- Create: `engine/src/raster.rs`, `engine/src/codec.rs`, `engine/tests/fixtures.rs`
- Modify: `engine/src/lib.rs` (add `pub mod raster; pub mod codec; pub use raster::*; pub use codec::*;`)
- Test: `engine/tests/codec.rs`

**Interfaces:**
- Produces: `Raster { width: u32, height: u32 }` (premultiplied RGBA8, `Arc` shared) with `new_transparent(w,h)`, `from_premultiplied(w,h,Vec<u8>)`, `from_straight(w,h,&[u8])`, `bytes() -> &[u8]`, `pixel(x,y) -> [u8;4]`, `to_straight() -> Vec<u8>`, `same_pixels(&other) -> bool` (Arc pointer equality), `tile_rgba(tx, ty, out: &mut [u8])` (256 px tiles, zero-padded), `halved() -> Raster` (box filter); `GrayRaster { width, height }` with `from_bytes`, `bytes()`, `is_uniform() -> Option<u8>`; `TILE: u32 = 256`.
- `codec::decode_image(bytes: &[u8]) -> Result<DecodedImage, ImportError>` where `DecodedImage { raster: Raster }` applies EXIF orientation and converts to premultiplied sRGB; `codec::decode_package_png(bytes) -> Result<Raster, ProjectError>` (PNG only, 8-bit or less, one frame, else `MissingImage`); `codec::decode_package_mask(bytes) -> Result<GrayRaster, ProjectError>`; `codec::encode_png(raster: &Raster, dpi: f64) -> Result<Vec<u8>, ProjectError>` (straight alpha, pHYs); `codec::encode_gray_png(&GrayRaster) -> Result<Vec<u8>, ProjectError>`; `codec::encode_jpeg(raster: &Raster, quality: f64, matte: [f64;3], dpi: f64) -> Result<Vec<u8>, ExportError>`; `codec::png_dpi(bytes) -> Option<f64>` (test helper, public).
- `tests/fixtures.rs`: `fn red_left_png() -> Vec<u8>` (64x32, left 32 columns opaque red, right transparent), `fn checker_raster(w,h) -> Raster`.

- [ ] **Step 1: Write the fixture helpers and failing tests**

`engine/tests/fixtures.rs`:
```rust
#![allow(dead_code)]
use compositor_engine::*;

/// 64x32: left half opaque red, right half transparent. Same shape as the macOS test fixture.
pub fn red_left_raster() -> Raster {
    let mut data = vec![0u8; 64 * 32 * 4];
    for y in 0..32 {
        for x in 0..32 {
            let i = (y * 64 + x) * 4;
            data[i] = 255; data[i + 3] = 255;
        }
    }
    Raster::from_premultiplied(64, 32, data)
}

pub fn red_left_png() -> Vec<u8> { encode_png(&red_left_raster(), 72.0).unwrap() }

/// Opaque noise-like pattern with distinct neighbours, for JPEG quality comparisons.
pub fn pattern_raster(w: u32, h: u32) -> Raster {
    let mut data = vec![0u8; (w * h * 4) as usize];
    for y in 0..h { for x in 0..w {
        let i = ((y * w + x) * 4) as usize;
        data[i] = ((x * 37 + y * 17) % 256) as u8;
        data[i + 1] = ((x * 11 + y * 53) % 256) as u8;
        data[i + 2] = ((x * 79 + y * 7) % 256) as u8;
        data[i + 3] = 255;
    }}
    Raster::from_premultiplied(w, h, data)
}
```

`engine/tests/codec.rs`:
```rust
mod fixtures;
use compositor_engine::*;
use fixtures::*;

#[test]
fn png_round_trip_keeps_pixels_and_transparency() {
    let bytes = red_left_png();
    let decoded = decode_image(&bytes).unwrap().raster;
    assert_eq!((decoded.width, decoded.height), (64, 32));
    assert_eq!(decoded.pixel(0, 0), [255, 0, 0, 255]);
    assert_eq!(decoded.pixel(63, 0), [0, 0, 0, 0]);
}

#[test]
fn jpeg_tiff_webp_bmp_decode() {
    let raster = red_left_raster();
    for format in [image::ImageFormat::Jpeg, image::ImageFormat::Tiff, image::ImageFormat::WebP, image::ImageFormat::Bmp] {
        let img = image::RgbaImage::from_raw(64, 32, raster.to_straight()).unwrap();
        let mut out = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(img).to_rgb8().write_to(&mut out, format).unwrap();
        let decoded = decode_image(&out.into_inner()).unwrap().raster;
        assert_eq!((decoded.width, decoded.height), (64, 32), "{format:?}");
        assert!(decoded.pixel(0, 0)[0] > 240, "{format:?}");
    }
}

#[test]
fn exif_orientation_is_applied() {
    // Build a JPEG with orientation 6 (rotate 90 CW) by writing an APP1 EXIF segment.
    let raster = red_left_raster();
    let img = image::RgbaImage::from_raw(64, 32, raster.to_straight()).unwrap();
    let mut jpeg = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(img).to_rgb8().write_to(&mut jpeg, image::ImageFormat::Jpeg).unwrap();
    let with_exif = insert_exif_orientation(jpeg.into_inner(), 6);
    let decoded = decode_image(&with_exif).unwrap().raster;
    assert_eq!((decoded.width, decoded.height), (32, 64));
}

/// Minimal EXIF APP1 with a single Orientation tag, inserted after SOI.
fn insert_exif_orientation(jpeg: Vec<u8>, orientation: u16) -> Vec<u8> {
    let mut tiff = vec![0x4D, 0x4D, 0x00, 0x2A, 0x00, 0x00, 0x00, 0x08]; // big endian, IFD0 at 8
    tiff.extend_from_slice(&[0x00, 0x01]); // one entry
    tiff.extend_from_slice(&[0x01, 0x12, 0x00, 0x03, 0x00, 0x00, 0x00, 0x01]); // tag 0x112 SHORT count 1
    tiff.extend_from_slice(&orientation.to_be_bytes());
    tiff.extend_from_slice(&[0, 0, 0, 0, 0, 0]); // pad + next IFD 0
    let mut app1 = b"Exif\0\0".to_vec();
    app1.extend_from_slice(&tiff);
    let len = (app1.len() + 2) as u16;
    let mut out = vec![0xFF, 0xD8, 0xFF, 0xE1];
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(&app1);
    out.extend_from_slice(&jpeg[2..]);
    out
}

#[test]
fn unsupported_and_unreadable() {
    let gif = b"GIF89a\x01\x00\x01\x00\x80\x00\x00\x00\x00\x00\xff\xff\xff\x21\xf9\x04\x01\x00\x00\x00\x00\x2c\x00\x00\x00\x00\x01\x00\x01\x00\x00\x02\x02\x44\x01\x00\x3b";
    assert_eq!(decode_image(gif).unwrap_err(), ImportError::Unsupported);
    assert_eq!(decode_image(b"not an image").unwrap_err(), ImportError::Unreadable);
    let mut truncated = red_left_png();
    truncated.truncate(40);
    assert_eq!(decode_image(&truncated).unwrap_err(), ImportError::Unreadable);
}

#[test]
fn package_png_rejects_other_formats_and_deep_bit_depths() {
    let raster = red_left_raster();
    let img = image::RgbaImage::from_raw(64, 32, raster.to_straight()).unwrap();
    let mut jpeg = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(img.clone()).to_rgb8().write_to(&mut jpeg, image::ImageFormat::Jpeg).unwrap();
    assert_eq!(decode_package_png(&jpeg.into_inner()).unwrap_err(), ProjectError::MissingImage);
    let sixteen = image::DynamicImage::ImageRgba8(img).to_rgba16();
    let mut png16 = std::io::Cursor::new(Vec::new());
    sixteen.write_to(&mut png16, image::ImageFormat::Png).unwrap();
    assert_eq!(decode_package_png(&png16.into_inner()).unwrap_err(), ProjectError::MissingImage);
    assert!(decode_package_png(&red_left_png()).is_ok());
}

#[test]
fn png_carries_resolution() {
    let bytes = encode_png(&red_left_raster(), 300.0).unwrap();
    let dpi = png_dpi(&bytes).unwrap();
    assert!((dpi - 300.0).abs() < 1.0);
}

#[test]
fn jpeg_uses_matte_quality_and_is_opaque() {
    let transparent = Raster::new_transparent(20, 12);
    let white = encode_jpeg(&transparent, 0.85, [1.0, 1.0, 1.0], 72.0).unwrap();
    let blue = encode_jpeg(&transparent, 1.0, [0.0, 0.0, 1.0], 72.0).unwrap();
    let w = decode_image(&white).unwrap().raster;
    let b = decode_image(&blue).unwrap().raster;
    assert_eq!((w.width, w.height), (20, 12));
    assert!(w.pixel(0, 0)[0] > 247 && w.pixel(0, 0)[3] == 255);
    assert!(b.pixel(0, 0)[2] > 247 && b.pixel(0, 0)[0] < 8);
    let pattern = pattern_raster(128, 128);
    let low = encode_jpeg(&pattern, 0.1, [1.0; 3], 72.0).unwrap();
    let high = encode_jpeg(&pattern, 1.0, [1.0; 3], 72.0).unwrap();
    assert!(low.len() < high.len());
}

#[test]
fn tiles_and_halving() {
    let r = pattern_raster(300, 270);
    let mut tile = vec![0u8; (TILE * TILE * 4) as usize];
    r.tile_rgba(1, 1, &mut tile);
    // Tile (1,1) covers x 256..300, y 256..270: pixel (0,0) of the tile is raster (256,256).
    assert_eq!(&tile[0..4], &r.pixel(256, 256));
    // Outside the raster is zero.
    let outside = ((5 * TILE + 100) * 4) as usize;
    assert_eq!(&tile[outside..outside + 4], &[0, 0, 0, 0]);
    let half = r.halved();
    assert_eq!((half.width, half.height), (150, 135));
    let expected = {
        let a = r.pixel(0, 0); let b = r.pixel(1, 0); let c = r.pixel(0, 1); let d = r.pixel(1, 1);
        ((a[0] as u32 + b[0] as u32 + c[0] as u32 + d[0] as u32 + 2) / 4) as u8
    };
    assert_eq!(half.pixel(0, 0)[0], expected);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p compositor-engine --test codec`
Expected: compile error, `raster` and `codec` items missing.

- [ ] **Step 3: Implement raster**

`engine/src/raster.rs`:
```rust
use std::sync::Arc;

pub const TILE: u32 = 256;

/// Premultiplied RGBA8, row-major, top-left origin. Cloning shares the pixels.
#[derive(Clone, Debug)]
pub struct Raster { pub width: u32, pub height: u32, data: Arc<Vec<u8>> }

impl PartialEq for Raster {
    fn eq(&self, other: &Self) -> bool { self.width == other.width && self.height == other.height && self.data == other.data }
}

impl Raster {
    pub fn new_transparent(width: u32, height: u32) -> Raster {
        Raster { width, height, data: Arc::new(vec![0; (width as usize) * (height as usize) * 4]) }
    }
    pub fn from_premultiplied(width: u32, height: u32, data: Vec<u8>) -> Raster {
        assert_eq!(data.len(), (width as usize) * (height as usize) * 4);
        Raster { width, height, data: Arc::new(data) }
    }
    pub fn from_straight(width: u32, height: u32, straight: &[u8]) -> Raster {
        let mut data = straight.to_vec();
        for px in data.chunks_exact_mut(4) {
            let a = px[3] as u32;
            if a < 255 {
                px[0] = ((px[0] as u32 * a + 127) / 255) as u8;
                px[1] = ((px[1] as u32 * a + 127) / 255) as u8;
                px[2] = ((px[2] as u32 * a + 127) / 255) as u8;
            }
        }
        Raster::from_premultiplied(width, height, data)
    }
    pub fn bytes(&self) -> &[u8] { &self.data }
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * self.width + x) * 4) as usize;
        [self.data[i], self.data[i + 1], self.data[i + 2], self.data[i + 3]]
    }
    pub fn to_straight(&self) -> Vec<u8> {
        let mut out = self.data.as_ref().clone();
        for px in out.chunks_exact_mut(4) {
            let a = px[3] as u32;
            if a > 0 && a < 255 {
                px[0] = ((px[0] as u32 * 255 + a / 2) / a).min(255) as u8;
                px[1] = ((px[1] as u32 * 255 + a / 2) / a).min(255) as u8;
                px[2] = ((px[2] as u32 * 255 + a / 2) / a).min(255) as u8;
            }
        }
        out
    }
    pub fn same_pixels(&self, other: &Raster) -> bool { Arc::ptr_eq(&self.data, &other.data) }
    pub fn tiles_across(&self) -> u32 { (self.width + TILE - 1) / TILE }
    pub fn tiles_down(&self) -> u32 { (self.height + TILE - 1) / TILE }
    /// Copies tile (tx, ty) into `out` (TILE*TILE*4 bytes), zero beyond the raster.
    pub fn tile_rgba(&self, tx: u32, ty: u32, out: &mut [u8]) {
        assert_eq!(out.len(), (TILE * TILE * 4) as usize);
        out.fill(0);
        let x0 = tx * TILE; let y0 = ty * TILE;
        if x0 >= self.width || y0 >= self.height { return; }
        let w = (self.width - x0).min(TILE) as usize;
        for row in 0..(self.height - y0).min(TILE) as usize {
            let src = (((y0 as usize + row) * self.width as usize) + x0 as usize) * 4;
            let dst = row * TILE as usize * 4;
            out[dst..dst + w * 4].copy_from_slice(&self.data[src..src + w * 4]);
        }
    }
    /// Box-filtered 2x reduction (odd edges average the pixels that exist).
    pub fn halved(&self) -> Raster {
        let w = (self.width / 2).max(1); let h = (self.height / 2).max(1);
        let mut out = vec![0u8; (w * h * 4) as usize];
        for y in 0..h { for x in 0..w {
            let mut sum = [0u32; 4]; let mut n = 0;
            for dy in 0..2 { for dx in 0..2 {
                let sx = x * 2 + dx; let sy = y * 2 + dy;
                if sx < self.width && sy < self.height {
                    let p = self.pixel(sx, sy);
                    for c in 0..4 { sum[c] += p[c] as u32; }
                    n += 1;
                }
            }}
            let i = ((y * w + x) * 4) as usize;
            for c in 0..4 { out[i + c] = ((sum[c] + n / 2) / n) as u8; }
        }}
        Raster::from_premultiplied(w, h, out)
    }
}

/// 8-bit coverage, no alpha (white reveals, black hides).
#[derive(Clone, Debug, PartialEq)]
pub struct GrayRaster { pub width: u32, pub height: u32, data: Arc<Vec<u8>> }

impl GrayRaster {
    pub fn from_bytes(width: u32, height: u32, data: Vec<u8>) -> GrayRaster {
        assert_eq!(data.len(), (width as usize) * (height as usize));
        GrayRaster { width, height, data: Arc::new(data) }
    }
    pub fn bytes(&self) -> &[u8] { &self.data }
    pub fn is_uniform(&self) -> Option<u8> {
        let first = *self.data.first()?;
        self.data.iter().all(|&v| v == first).then_some(first)
    }
}
```

- [ ] **Step 4: Implement codecs**

`engine/src/codec.rs`:
```rust
use crate::{ExportError, GrayRaster, ImportError, ProjectError, Raster};
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader};
use std::io::Cursor;

pub struct DecodedImage { pub raster: Raster }

const SUPPORTED: [ImageFormat; 5] = [ImageFormat::Png, ImageFormat::Jpeg, ImageFormat::Tiff, ImageFormat::WebP, ImageFormat::Bmp];

/// Decodes an import, applies EXIF orientation and returns premultiplied RGBA8.
pub fn decode_image(bytes: &[u8]) -> Result<DecodedImage, ImportError> {
    let format = image::guess_format(bytes).map_err(|_| ImportError::Unreadable)?;
    if !SUPPORTED.contains(&format) { return Err(ImportError::Unsupported); }
    let reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut decoder = reader.into_decoder().map_err(|_| ImportError::Unreadable)?;
    let orientation = decoder.orientation().unwrap_or(image::metadata::Orientation::NoTransforms);
    let (w, h) = decoder.dimensions();
    if w == 0 || h == 0 || w > 30_000 || h > 30_000 { return Err(ImportError::TooLarge); }
    let mut img = DynamicImage::from_decoder(decoder).map_err(|_| ImportError::Unreadable)?;
    img.apply_orientation(orientation);
    let rgba = img.into_rgba8();
    let (w, h) = rgba.dimensions();
    Ok(DecodedImage { raster: Raster::from_straight(w, h, rgba.as_raw()) })
}

fn png_header(bytes: &[u8]) -> Result<png::Info<'static>, ProjectError> {
    if image::guess_format(bytes).ok() != Some(ImageFormat::Png) { return Err(ProjectError::MissingImage); }
    let decoder = png::Decoder::new(Cursor::new(bytes));
    let reader = decoder.read_info().map_err(|_| ProjectError::MissingImage)?;
    let info = reader.info();
    if info.bit_depth as u8 > 8 || info.animation_control.is_some() { return Err(ProjectError::MissingImage); }
    Ok(info.clone())
}

/// Package assets are PNG only, at most 8 bits per channel, one frame.
pub fn decode_package_png(bytes: &[u8]) -> Result<Raster, ProjectError> {
    png_header(bytes)?;
    let img = image::load_from_memory_with_format(bytes, ImageFormat::Png).map_err(|_| ProjectError::MissingImage)?;
    let rgba = img.into_rgba8();
    let (w, h) = rgba.dimensions();
    Ok(Raster::from_straight(w, h, rgba.as_raw()))
}

pub fn decode_package_mask(bytes: &[u8]) -> Result<GrayRaster, ProjectError> {
    let info = png_header(bytes)?;
    if info.color_type != png::ColorType::Grayscale { return Err(ProjectError::Invalid); }
    let img = image::load_from_memory_with_format(bytes, ImageFormat::Png).map_err(|_| ProjectError::MissingImage)?;
    let gray = img.into_luma8();
    let (w, h) = gray.dimensions();
    Ok(GrayRaster::from_bytes(w, h, gray.into_raw()))
}

fn dpi_to_ppm(dpi: f64) -> u32 { (dpi / 0.0254).round().max(1.0) as u32 }

pub fn encode_png(raster: &Raster, dpi: f64) -> Result<Vec<u8>, ProjectError> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, raster.width, raster.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_pixel_dims(Some(png::PixelDimensions { xppu: dpi_to_ppm(dpi), yppu: dpi_to_ppm(dpi), unit: png::Unit::Meter }));
        let mut writer = encoder.write_header().map_err(|_| ProjectError::Encode)?;
        writer.write_image_data(&raster.to_straight()).map_err(|_| ProjectError::Encode)?;
    }
    Ok(out)
}

pub fn encode_gray_png(mask: &GrayRaster) -> Result<Vec<u8>, ProjectError> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, mask.width, mask.height);
        encoder.set_color(png::ColorType::Grayscale);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|_| ProjectError::Encode)?;
        writer.write_image_data(mask.bytes()).map_err(|_| ProjectError::Encode)?;
    }
    Ok(out)
}

/// Flattens onto `matte` (0..1 RGB), encodes at `quality` (0..1) with JFIF density.
pub fn encode_jpeg(raster: &Raster, quality: f64, matte: [f64; 3], dpi: f64) -> Result<Vec<u8>, ExportError> {
    let m: [u32; 3] = [0, 1, 2].map(|i| (matte[i].clamp(0.0, 1.0) * 255.0).round() as u32);
    let mut rgb = Vec::with_capacity((raster.width * raster.height * 3) as usize);
    for px in raster.bytes().chunks_exact(4) {
        let a = px[3] as u32;
        for c in 0..3 { rgb.push((px[c] as u32 + (m[c] * (255 - a) + 127) / 255).min(255) as u8); }
    }
    let q = (quality.clamp(0.0, 1.0) * 100.0).round().max(1.0) as u8;
    let mut out = Vec::new();
    let mut encoder = jpeg_encoder::Encoder::new(&mut out, q);
    encoder.set_density(jpeg_encoder::Density::Inch { x: dpi.round() as u16, y: dpi.round() as u16 });
    encoder.encode(&rgb, raster.width as u16, raster.height as u16, jpeg_encoder::ColorType::Rgb).map_err(|_| ExportError::Encode)?;
    Ok(out)
}

/// Horizontal DPI from a PNG pHYs chunk, if present in metres.
pub fn png_dpi(bytes: &[u8]) -> Option<f64> {
    let decoder = png::Decoder::new(Cursor::new(bytes));
    let reader = decoder.read_info().ok()?;
    let dims = reader.info().pixel_dims?;
    match dims.unit { png::Unit::Meter => Some(dims.xppu as f64 * 0.0254), _ => None }
}
```

If `decoder.orientation()` is not available in the resolved `image` version, pin `image = "0.25.6"` or newer; the method exists from 0.25.5.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p compositor-engine --test codec`
Expected: 8 passed.

- [ ] **Step 6: Commit**

```
git add engine
git commit -m "feat(engine): raster storage and PNG/JPEG/TIFF/WebP/BMP codecs"
```

---

### Task 4: Document model and package open/save

**Files:**
- Create: `engine/src/document.rs`, `engine/src/package.rs`
- Modify: `engine/src/lib.rs` (add `pub mod document; pub mod package; pub use document::*; pub use package::*;`)
- Test: `engine/tests/package.rs`

**Interfaces:**
- Produces: `Layer { id, name, visible, transform, pixels: Option<Raster>, pixels_revision: u64, parent_id, is_group, opacity, blend_mode, mask: Option<Mask>, mask_source_id, extra: LayerExtra }`, `Mask { pixels: GrayRaster, enabled: bool, placement: Option<LayerTransform>, linked: Option<bool> }`, `LayerExtra { adjustment: Option<Value>, shape: Option<Value> }`; `Layer::blank(name, canvas: Size)`, `Layer::with_pixels(name, raster, origin)`, `Layer::set_pixels(Option<Raster>)` (bumps revision), `Layer::record() -> LayerRecord`; `Document { id, width: u32, height: u32, resolution: f64, layers: Vec<Layer>, active_layer_id }` with `Document::new(w,h)`, `size() -> Size`, `manifest() -> Manifest`, `used_pixels() -> u64`, `layer(id) -> Option<&Layer>`, `layer_mut(id)`, `index_of(id)`; `Package { manifest_json: String, images: Vec<(String, Vec<u8>)> }`; `open_package(&Package) -> Result<Document, ProjectError>`; `save_package(&Document) -> Result<Package, ProjectError>`.

- [ ] **Step 1: Write the failing tests**

`engine/tests/package.rs`:
```rust
mod fixtures;
use compositor_engine::*;
use fixtures::*;

#[test]
fn round_trip_keeps_metadata_pixels_and_blank_layers() {
    let mut doc = Document::new(200, 100);
    let mut image = Layer::with_pixels("Paint & sky \u{1F33B}", red_left_raster(), Point { x: -27.5, y: 88.25 });
    image.transform.size = Size { width: 123.0, height: 47.0 };
    image.transform.rotation = 38.0;
    image.transform.flip_x = true; image.transform.flip_y = true;
    image.transform.sampling = Sampling::Nearest;
    image.visible = false;
    doc.layers.push(image);
    doc.layers.push(Layer::blank("Layer 2", doc.size()));
    doc.active_layer_id = Some(doc.layers[1].id);
    doc.resolution = 300.0;

    let pkg = save_package(&doc).unwrap();
    assert_eq!(pkg.images.len(), 1);
    assert_eq!(pkg.images[0].0, LayerRecord::image_filename(&doc.layers[0].id));
    let back = open_package(&pkg).unwrap();
    assert_eq!(back.id, doc.id);
    assert_eq!((back.width, back.height, back.resolution), (200, 100, 300.0));
    assert_eq!(back.layers.iter().map(|l| l.id).collect::<Vec<_>>(), doc.layers.iter().map(|l| l.id).collect::<Vec<_>>());
    assert_eq!(back.layers[0].name, doc.layers[0].name);
    assert_eq!(back.layers[0].transform, doc.layers[0].transform);
    assert!(!back.layers[0].visible);
    assert!(back.layers[1].pixels.is_none());
    assert_eq!(back.active_layer_id, doc.active_layer_id);
    let px = back.layers[0].pixels.as_ref().unwrap();
    assert_eq!(px.pixel(0, 0), [255, 0, 0, 255]);
    assert_eq!(px.pixel(63, 0)[3], 0);
}

#[test]
fn missing_image_is_rejected() {
    let mut doc = Document::new(64, 32);
    doc.layers.push(Layer::with_pixels("Red", red_left_raster(), Point { x: 0.0, y: 0.0 }));
    let mut pkg = save_package(&doc).unwrap();
    pkg.images.clear();
    assert_eq!(open_package(&pkg).unwrap_err(), ProjectError::MissingImage);
}

#[test]
fn corrupt_future_and_unsafe_manifests_are_rejected() {
    let mut doc = Document::new(100, 80);
    doc.layers.push(Layer::blank("Layer 1", doc.size()));
    let pkg = save_package(&doc).unwrap();
    let mut future = pkg.clone();
    future.manifest_json = pkg.manifest_json.replace("\"version\": 7", "\"version\": 42");
    assert_eq!(open_package(&future).unwrap_err(), ProjectError::Version(42));
    let mut unsafe_pkg = pkg.clone();
    unsafe_pkg.manifest_json = pkg.manifest_json.replace("\"name\": \"Layer 1\"", "\"imageFile\": \"../../outside.png\", \"name\": \"Layer 1\"");
    assert_eq!(open_package(&unsafe_pkg).unwrap_err(), ProjectError::Invalid);
    let mut corrupt = pkg.clone();
    corrupt.manifest_json = "not json".into();
    assert_eq!(open_package(&corrupt).unwrap_err(), ProjectError::Invalid);
}

#[test]
fn masks_and_later_phase_fields_survive_round_trip() {
    let mut doc = Document::new(64, 32);
    let mut layer = Layer::with_pixels("Red", red_left_raster(), Point { x: 0.0, y: 0.0 });
    layer.mask = Some(Mask { pixels: GrayRaster::from_bytes(1, 1, vec![255]), enabled: false, placement: None, linked: Some(true) });
    layer.extra.shape = Some(serde_json::json!({"kind": "rectangle", "red": 1.0, "green": 0.0, "blue": 0.0, "cornerRadius": 0.0}));
    layer.opacity = 0.5;
    layer.blend_mode = BlendMode::Multiply;
    doc.layers.push(layer);
    let pkg = save_package(&doc).unwrap();
    assert!(pkg.images.iter().any(|(n, _)| n.ends_with(".mask.png")));
    let back = open_package(&pkg).unwrap();
    let l = &back.layers[0];
    assert_eq!(l.mask.as_ref().unwrap().pixels.is_uniform(), Some(255));
    assert!(!l.mask.as_ref().unwrap().enabled);
    assert_eq!(l.extra.shape, doc.layers[0].extra.shape);
    assert_eq!((l.opacity, l.blend_mode), (0.5, BlendMode::Multiply));
}

#[test]
fn unsupported_save_state_fails_before_encoding() {
    let mut doc = Document::new(100, 80);
    doc.layers.push(Layer::blank("", doc.size()));
    assert_eq!(save_package(&doc).unwrap_err(), ProjectError::Invalid);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p compositor-engine --test package`
Expected: compile error.

- [ ] **Step 3: Implement the document model**

`engine/src/document.rs`:
```rust
use crate::{BlendMode, GrayRaster, LayerRecord, LayerTransform, Manifest, Point, Raster, Size, DEFAULT_RESOLUTION};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq)]
pub struct Mask {
    pub pixels: GrayRaster,
    pub enabled: bool,
    pub placement: Option<LayerTransform>,
    pub linked: Option<bool>,
}

/// Fields owned by later phases, carried through untouched.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct LayerExtra {
    pub adjustment: Option<serde_json::Value>,
    pub shape: Option<serde_json::Value>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    pub id: Uuid,
    pub name: String,
    pub visible: bool,
    pub transform: LayerTransform,
    pub pixels: Option<Raster>,
    /// Bumped whenever `pixels` is replaced, so a renderer knows to re-upload.
    pub pixels_revision: u64,
    pub parent_id: Option<Uuid>,
    pub is_group: bool,
    pub opacity: f64,
    pub blend_mode: BlendMode,
    pub mask: Option<Mask>,
    pub mask_source_id: Option<Uuid>,
    pub extra: LayerExtra,
}

impl Layer {
    fn base(name: &str, transform: LayerTransform, pixels: Option<Raster>) -> Layer {
        Layer { id: Uuid::new_v4(), name: name.to_string(), visible: true, transform, pixels, pixels_revision: 1,
            parent_id: None, is_group: false, opacity: 1.0, blend_mode: BlendMode::Normal, mask: None,
            mask_source_id: None, extra: LayerExtra::default() }
    }
    /// A blank layer covers the canvas and owns no pixels until painted.
    pub fn blank(name: &str, canvas: Size) -> Layer {
        Layer::base(name, LayerTransform::axis_aligned(Point { x: 0.0, y: 0.0 }, canvas), None)
    }
    pub fn with_pixels(name: &str, raster: Raster, origin: Point) -> Layer {
        let size = Size { width: raster.width as f64, height: raster.height as f64 };
        Layer::base(name, LayerTransform::axis_aligned(origin, size), Some(raster))
    }
    pub fn set_pixels(&mut self, pixels: Option<Raster>) {
        self.pixels = pixels;
        self.pixels_revision += 1;
    }
    pub fn record(&self) -> LayerRecord {
        let mut r = LayerRecord::new(self.id, &self.name, self.transform,
            self.pixels.as_ref().map(|_| LayerRecord::image_filename(&self.id)));
        r.is_visible = self.visible;
        r.parent_id = self.parent_id;
        r.is_group = if self.is_group { Some(true) } else { None };
        r.opacity = if self.opacity != 1.0 { Some(self.opacity) } else { None };
        r.blend_mode = if self.blend_mode != BlendMode::Normal { Some(self.blend_mode) } else { None };
        if let Some(mask) = &self.mask {
            r.mask_file = Some(LayerRecord::mask_filename(&self.id));
            r.mask_enabled = Some(mask.enabled);
            r.mask_placement = mask.placement;
            r.mask_linked = mask.linked;
        }
        r.mask_source_id = self.mask_source_id;
        r.adjustment = self.extra.adjustment.clone();
        r.shape = self.extra.shape.clone();
        r
    }
    pub fn from_record(record: &LayerRecord, pixels: Option<Raster>, mask: Option<GrayRaster>) -> Layer {
        Layer {
            id: record.id, name: record.name.clone(), visible: record.is_visible, transform: record.transform,
            pixels, pixels_revision: 1, parent_id: record.parent_id, is_group: record.is_group(),
            opacity: record.opacity.unwrap_or(1.0), blend_mode: record.blend_mode.unwrap_or_default(),
            mask: mask.map(|pixels| Mask { pixels, enabled: record.mask_enabled.unwrap_or(true),
                placement: record.mask_placement, linked: record.mask_linked }),
            mask_source_id: record.mask_source_id,
            extra: LayerExtra { adjustment: record.adjustment.clone(), shape: record.shape.clone() },
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Document {
    pub id: Uuid,
    pub width: u32,
    pub height: u32,
    pub resolution: f64,
    pub layers: Vec<Layer>, // bottom to top
    pub active_layer_id: Option<Uuid>,
}

impl Document {
    pub fn new(width: u32, height: u32) -> Document {
        Document { id: Uuid::new_v4(), width, height, resolution: DEFAULT_RESOLUTION, layers: vec![], active_layer_id: None }
    }
    pub fn size(&self) -> Size { Size { width: self.width as f64, height: self.height as f64 } }
    pub fn manifest(&self) -> Manifest {
        let mut m = Manifest::new(self.id, self.width as i64, self.height as i64, self.active_layer_id,
            self.layers.iter().map(Layer::record).collect());
        m.resolution = Some(self.resolution);
        m
    }
    pub fn used_pixels(&self) -> u64 {
        self.layers.iter().filter_map(|l| l.pixels.as_ref()).map(|r| r.width as u64 * r.height as u64).sum()
    }
    pub fn used_mask_pixels(&self) -> u64 {
        self.layers.iter().filter_map(|l| l.mask.as_ref()).map(|m| m.pixels.width as u64 * m.pixels.height as u64).sum()
    }
    pub fn index_of(&self, id: Uuid) -> Option<usize> { self.layers.iter().position(|l| l.id == id) }
    pub fn layer(&self, id: Uuid) -> Option<&Layer> { self.layers.iter().find(|l| l.id == id) }
    pub fn layer_mut(&mut self, id: Uuid) -> Option<&mut Layer> { self.layers.iter_mut().find(|l| l.id == id) }
}
```

- [ ] **Step 4: Implement package open and save**

`engine/src/package.rs`:
```rust
use crate::*;
use std::collections::HashMap;

/// The bytes of a `.comp` folder: the manifest text and `images/<name>` files.
#[derive(Clone, Debug, PartialEq)]
pub struct Package {
    pub manifest_json: String,
    pub images: Vec<(String, Vec<u8>)>,
}

fn check_budget(width: u32, height: u32, used: &mut u64) -> Result<(), ProjectError> {
    let w = width as i64; let h = height as i64;
    if !(1..=MAX_SIDE).contains(&w) || !(1..=MAX_SIDE).contains(&h) { return Err(ProjectError::TooLarge); }
    let pixels = width as u64 * height as u64;
    if pixels > MAX_PIXELS - *used { return Err(ProjectError::TooLarge); }
    *used += pixels;
    Ok(())
}

pub fn open_package(pkg: &Package) -> Result<Document, ProjectError> {
    let manifest = Manifest::parse(&pkg.manifest_json)?;
    let files: HashMap<&str, &Vec<u8>> = pkg.images.iter().map(|(n, b)| (n.as_str(), b)).collect();
    let mut used = 0u64; let mut used_masks = 0u64;
    let mut layers = Vec::with_capacity(manifest.layers.len());
    for record in &manifest.layers {
        let pixels = match &record.image_file {
            Some(name) => {
                let bytes = files.get(name.as_str()).ok_or(ProjectError::MissingImage)?;
                if bytes.len() as u64 > MAX_ASSET_BYTES { return Err(ProjectError::TooLarge); }
                let raster = decode_package_png(bytes)?;
                check_budget(raster.width, raster.height, &mut used)?;
                Some(raster)
            }
            None => None,
        };
        let mask = match &record.mask_file {
            Some(name) => {
                let bytes = files.get(name.as_str()).ok_or(ProjectError::MissingImage)?;
                if bytes.len() as u64 > MAX_ASSET_BYTES { return Err(ProjectError::TooLarge); }
                let gray = decode_package_mask(bytes)?;
                check_budget(gray.width, gray.height, &mut used_masks)?;
                Some(gray)
            }
            None => None,
        };
        layers.push(Layer::from_record(record, pixels, mask));
    }
    Ok(Document {
        id: manifest.document_id, width: manifest.width as u32, height: manifest.height as u32,
        resolution: manifest.resolution.unwrap_or(DEFAULT_RESOLUTION), layers, active_layer_id: manifest.active_layer_id,
    })
}

pub fn save_package(doc: &Document) -> Result<Package, ProjectError> {
    let manifest = doc.manifest();
    manifest.validate()?;
    let mut used = 0u64; let mut used_masks = 0u64;
    let mut images = Vec::new();
    for layer in &doc.layers {
        if let Some(raster) = &layer.pixels {
            check_budget(raster.width, raster.height, &mut used)?;
            images.push((LayerRecord::image_filename(&layer.id), encode_png(raster, doc.resolution)?));
        }
        if let Some(mask) = &layer.mask {
            check_budget(mask.pixels.width, mask.pixels.height, &mut used_masks)?;
            images.push((LayerRecord::mask_filename(&layer.id), encode_gray_png(&mask.pixels)?));
        }
    }
    Ok(Package { manifest_json: manifest.to_json_pretty()?, images })
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p compositor-engine`
Expected: all tests in geometry, manifest, codec, package pass.

- [ ] **Step 6: Commit**

```
git add engine
git commit -m "feat(engine): document model and .comp package open/save"
```

---

### Task 5: CPU compositor and export

**Files:**
- Create: `engine/src/compositor.rs`
- Modify: `engine/src/lib.rs` (add `pub mod compositor; pub use compositor::*;`)
- Test: `engine/tests/export.rs`

**Interfaces:**
- Produces: `composite(doc: &Document, region: Rect, out_width: u32, out_height: u32) -> Raster` (draws the document rectangle `region` scaled into an `out_width` by `out_height` premultiplied raster; Normal blend with opacity; hidden layers, hidden ancestors and groups skipped; masks ignored in Phase 1); `render_layer(target: &mut [u8], tw, th, region, layer: &Layer)`; `export_png(doc) -> Result<Vec<u8>, ExportError>`; `export_jpeg(doc, quality, matte) -> Result<Vec<u8>, ExportError>`; `sample(raster, x, y, sampling) -> [f32;4]`.

- [ ] **Step 1: Write the failing tests**

`engine/tests/export.rs`:
```rust
mod fixtures;
use compositor_engine::*;

/// 2x2 image: left column red, right column transparent, placed at (1,1) as 4x4 on a 6x6 canvas.
fn doc(rotation: f64, flip: bool) -> Document {
    let mut d = Document::new(6, 6);
    let raster = Raster::from_premultiplied(2, 2, vec![255,0,0,255, 0,0,0,0, 255,0,0,255, 0,0,0,0]);
    let mut layer = Layer::with_pixels("Red", raster, Point { x: 1.0, y: 1.0 });
    layer.transform.size = Size { width: 4.0, height: 4.0 };
    layer.transform.rotation = rotation;
    layer.transform.flip_x = flip;
    layer.transform.sampling = Sampling::Nearest;
    d.active_layer_id = Some(layer.id);
    d.layers.push(layer);
    d
}

#[test]
fn png_preserves_dimensions_alpha_orientation_and_transforms() {
    for (rotation, flip, red, clear) in [(0.0, false, (1, 1), (4, 1)), (0.0, true, (4, 1), (1, 1)), (90.0, false, (1, 1), (1, 4))] {
        let bytes = export_png(&doc(rotation, flip)).unwrap();
        let img = decode_image(&bytes).unwrap().raster;
        assert_eq!((img.width, img.height), (6, 6));
        let r = img.pixel(red.0, red.1);
        assert!(r[0] > 252 && r[3] == 255, "rot {rotation} flip {flip}: {r:?}");
        assert_eq!(img.pixel(clear.0, clear.1)[3], 0, "rot {rotation} flip {flip}");
        assert_eq!(img.pixel(0, 0)[3], 0);
    }
}

#[test]
fn order_and_visibility() {
    let mut d = doc(0.0, false);
    let blue = Raster::from_premultiplied(1, 1, vec![0, 0, 255, 255]);
    let mut top = Layer::with_pixels("Blue", blue, Point { x: -2.0, y: -2.0 });
    top.transform.size = Size { width: 10.0, height: 10.0 };
    for visible in [true, false] {
        top.visible = visible;
        d.layers.truncate(1);
        d.layers.push(top.clone());
        let img = decode_image(&export_png(&d).unwrap()).unwrap().raster;
        let px = img.pixel(1, 1);
        if visible { assert!(px[2] > 252); } else { assert!(px[0] > 252); }
    }
}

#[test]
fn hidden_group_hides_children() {
    let mut d = doc(0.0, false);
    let mut group = Layer::blank("Folder", d.size());
    group.is_group = true;
    group.visible = false;
    d.layers[0].parent_id = Some(group.id);
    d.layers.insert(0, group);
    let img = decode_image(&export_png(&d).unwrap()).unwrap().raster;
    assert_eq!(img.pixel(1, 1)[3], 0);
}

#[test]
fn opacity_scales_alpha() {
    let mut d = doc(0.0, false);
    d.layers[0].opacity = 0.5;
    let out = composite(&d, Rect { x: 0.0, y: 0.0, width: 6.0, height: 6.0 }, 6, 6);
    let px = out.pixel(1, 1);
    assert!((px[3] as i32 - 128).abs() <= 1 && (px[0] as i32 - 128).abs() <= 1);
}

#[test]
fn blank_and_oversized_canvas() {
    let blank = Document::new(2, 2);
    let img = decode_image(&export_png(&blank).unwrap()).unwrap().raster;
    assert_eq!(img.pixel(1, 1)[3], 0);
    let huge = Document::new(30_000, 30_000);
    assert_eq!(export_png(&huge).unwrap_err(), ExportError::TooLarge);
}

#[test]
fn smooth_sampling_blends_at_edges_and_high_prefilters_when_shrinking() {
    let mut d = Document::new(4, 4);
    let checker = fixtures::pattern_raster(64, 64);
    let mut layer = Layer::with_pixels("P", checker, Point { x: 0.0, y: 0.0 });
    layer.transform.size = Size { width: 4.0, height: 4.0 };
    layer.transform.sampling = Sampling::High;
    d.layers.push(layer);
    let high = composite(&d, Rect { x: 0.0, y: 0.0, width: 4.0, height: 4.0 }, 4, 4);
    d.layers[0].transform.sampling = Sampling::Nearest;
    let nearest = composite(&d, Rect { x: 0.0, y: 0.0, width: 4.0, height: 4.0 }, 4, 4);
    // Prefiltered output averages 16x16 source blocks; nearest picks one texel. They differ.
    assert_ne!(high.bytes(), nearest.bytes());
    assert_eq!(high.pixel(0, 0)[3], 255);
}

#[test]
fn jpeg_export_flattens_on_matte() {
    let d = Document::new(20, 12);
    let bytes = export_jpeg(&d, 0.85, [0.0, 0.0, 1.0]).unwrap();
    let img = decode_image(&bytes).unwrap().raster;
    assert_eq!((img.width, img.height), (20, 12));
    assert!(img.pixel(0, 0)[2] > 247 && img.pixel(0, 0)[3] == 255);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p compositor-engine --test export`
Expected: compile error, `composite` missing.

- [ ] **Step 3: Implement the compositor**

`engine/src/compositor.rs`:
```rust
use crate::*;
use std::collections::HashMap;
use uuid::Uuid;

/// Bilinear or nearest sample in premultiplied float RGBA (0..1). Outside the raster is transparent.
pub fn sample(raster: &Raster, x: f64, y: f64, nearest: bool) -> [f32; 4] {
    let w = raster.width as i64; let h = raster.height as i64;
    let fetch = |px: i64, py: i64| -> [f32; 4] {
        if px < 0 || py < 0 || px >= w || py >= h { return [0.0; 4]; }
        let p = raster.pixel(px as u32, py as u32);
        [p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0, p[3] as f32 / 255.0]
    };
    if nearest {
        let px = x.floor() as i64; let py = y.floor() as i64;
        return fetch(px, py);
    }
    let fx = x - 0.5; let fy = y - 0.5;
    let x0 = fx.floor(); let y0 = fy.floor();
    let tx = (fx - x0) as f32; let ty = (fy - y0) as f32;
    let (x0, y0) = (x0 as i64, y0 as i64);
    let a = fetch(x0, y0); let b = fetch(x0 + 1, y0); let c = fetch(x0, y0 + 1); let d = fetch(x0 + 1, y0 + 1);
    let mut out = [0f32; 4];
    for i in 0..4 {
        out[i] = (a[i] * (1.0 - tx) + b[i] * tx) * (1.0 - ty) + (c[i] * (1.0 - tx) + d[i] * tx) * ty;
    }
    out
}

/// Sharp halvings for large reductions: reduce until one output pixel covers at most 2 source pixels.
fn prefiltered(raster: &Raster, pixels_per_output: f64) -> (Raster, f64) {
    let mut current = raster.clone();
    let mut scale = 1.0;
    let mut factor = pixels_per_output;
    while factor > 2.0 && current.width > 1 && current.height > 1 {
        current = current.halved();
        scale *= 0.5;
        factor /= 2.0;
    }
    (current, scale)
}

/// Draws one layer into `target` (premultiplied RGBA8, `tw` x `th`) covering document `region`.
pub fn render_layer(target: &mut [u8], tw: u32, th: u32, region: Rect, layer: &Layer) {
    let Some(raster) = &layer.pixels else { return; };
    let out_per_doc_x = tw as f64 / region.width;
    let out_per_doc_y = th as f64 / region.height;
    let nearest = layer.transform.sampling == Sampling::Nearest;
    // Source pixels per output pixel along the layer's width, for prefiltering.
    let source_per_output = raster.width as f64 / (layer.transform.size.width * out_per_doc_x);
    let (source, scale) = if layer.transform.sampling == Sampling::High && source_per_output > 2.0 {
        prefiltered(raster, source_per_output)
    } else { (raster.clone(), 1.0) };
    let Some(inverse) = layer.transform.pixel_to_document(raster.width, raster.height).invert() else { return; };
    let opacity = layer.opacity.clamp(0.0, 1.0) as f32;
    // Bounding box of the layer in output pixels.
    let b = layer.transform.bounds();
    let x0 = (((b.x - region.x) * out_per_doc_x).floor() as i64 - 1).max(0) as u32;
    let y0 = (((b.y - region.y) * out_per_doc_y).floor() as i64 - 1).max(0) as u32;
    let x1 = (((b.max_x() - region.x) * out_per_doc_x).ceil() as i64 + 1).min(tw as i64).max(0) as u32;
    let y1 = (((b.max_y() - region.y) * out_per_doc_y).ceil() as i64 + 1).min(th as i64).max(0) as u32;
    for oy in y0..y1 {
        for ox in x0..x1 {
            let doc = Point { x: region.x + (ox as f64 + 0.5) / out_per_doc_x, y: region.y + (oy as f64 + 0.5) / out_per_doc_y };
            let p = inverse.apply(doc);
            if nearest && (p.x < 0.0 || p.y < 0.0 || p.x >= raster.width as f64 || p.y >= raster.height as f64) { continue; }
            let s = sample(&source, p.x * scale, p.y * scale, nearest);
            if s[3] <= 0.0 { continue; }
            let i = ((oy * tw + ox) * 4) as usize;
            let src_a = s[3] * opacity;
            for c in 0..3 {
                let dst = target[i + c] as f32 / 255.0;
                target[i + c] = ((s[c] * opacity + dst * (1.0 - src_a)) * 255.0).round().clamp(0.0, 255.0) as u8;
            }
            let dst_a = target[i + 3] as f32 / 255.0;
            target[i + 3] = ((src_a + dst_a * (1.0 - src_a)) * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    }
}

/// Layers to draw, bottom to top: visible, with every ancestor visible, groups excluded.
pub fn render_layers(doc: &Document) -> Vec<&Layer> {
    let by_id: HashMap<Uuid, &Layer> = doc.layers.iter().map(|l| (l.id, l)).collect();
    doc.layers.iter().filter(|layer| {
        if layer.is_group { return false; }
        let mut node = Some(*layer);
        let mut steps = 0;
        while let Some(n) = node {
            if !n.visible || steps > MAX_NESTING { return false; }
            steps += 1;
            node = n.parent_id.and_then(|p| by_id.get(&p).copied());
        }
        true
    }).collect()
}

pub fn composite(doc: &Document, region: Rect, out_width: u32, out_height: u32) -> Raster {
    let mut target = vec![0u8; (out_width as usize) * (out_height as usize) * 4];
    for layer in render_layers(doc) {
        render_layer(&mut target, out_width, out_height, region, layer);
    }
    Raster::from_premultiplied(out_width, out_height, target)
}

fn check_export_size(doc: &Document) -> Result<(), ExportError> {
    let w = doc.width as i64; let h = doc.height as i64;
    if !(1..=MAX_SIDE).contains(&w) || !(1..=MAX_SIDE).contains(&h) || (w * h) as u64 > MAX_PIXELS { return Err(ExportError::TooLarge); }
    Ok(())
}

pub fn render_full(doc: &Document) -> Result<Raster, ExportError> {
    check_export_size(doc)?;
    Ok(composite(doc, Rect { x: 0.0, y: 0.0, width: doc.width as f64, height: doc.height as f64 }, doc.width, doc.height))
}

pub fn export_png(doc: &Document) -> Result<Vec<u8>, ExportError> {
    let raster = render_full(doc)?;
    encode_png(&raster, doc.resolution).map_err(|_| ExportError::Encode)
}

pub fn export_jpeg(doc: &Document, quality: f64, matte: [f64; 3]) -> Result<Vec<u8>, ExportError> {
    let raster = render_full(doc)?;
    encode_jpeg(&raster, quality, matte, doc.resolution)
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p compositor-engine --test export`
Expected: 7 passed. If the rotation-90 case picks the wrong pixel, check `Affine::then` ordering against the `pixel_to_document_and_inverse_round_trip` test in Task 1 (the image center must land on the transform center) before touching the sampler.

- [ ] **Step 5: Commit**

```
git add engine
git commit -m "feat(engine): CPU compositor with nearest/bilinear/prefiltered sampling and PNG/JPEG export"
```

---

### Task 6: History, commands and the Engine facade

**Files:**
- Create: `engine/src/history.rs`, `engine/src/command.rs`, `engine/src/engine.rs`, `engine/src/ops/mod.rs`, `engine/src/ops/layers.rs`
- Modify: `engine/src/lib.rs` (add `pub mod history; pub mod command; pub mod engine; pub mod ops; pub use history::*; pub use command::*; pub use engine::*;`)
- Test: `engine/tests/engine.rs`

**Interfaces:**
- Produces: `History` with `push(before: Document)`, `undo(current) -> Option<Document>`, `redo(current) -> Option<Document>`, `can_undo()`, `can_redo()`, `mark_saved()`, `is_modified()`, `reset()`.
- `Command` enum (serde, `#[serde(tag = "type")]`): `AddBlankLayer`, `RenameLayer { id, name }`, `SetLayerVisible { id, visible }`, `DeleteLayer { id }`, `SetActiveLayer { id: Option<Uuid> }`, `CanvasSize { width, height, anchor, fill: Option<[f64;3]> }`, `Crop { x, y, width, height }`, `ImageSize { width, height, resolution, sampling }`, `FlipCanvas { horizontal }`. Tasks 7 and 8 fill in the last four; Task 6 returns `CommandError::Argument("not implemented")` for them.
- `Dirty { structure: bool, canvas: bool, layers: Vec<Uuid> }` (serde).
- `DocumentState` (serde, camelCase): `id, width, height, resolution, activeLayerId, canUndo, canRedo, isModified, path: Option<String>, layers: Vec<LayerState>` where `LayerState { id, name, visible, isGroup, parentId, opacity, blendMode, transform, pixelsWidth, pixelsHeight, pixelsRevision, hasMask }`.
- `Engine` with `new()`, `new_document(w, h, empty_layer: bool) -> Result<Uuid, CommandError>`, `open_package(pkg: &Package, path: Option<String>) -> Result<Uuid, CommandError>`, `save_package(id) -> Result<Package, CommandError>` (does not mark saved), `mark_saved(id, path: Option<String>)`, `close_document(id)`, `document(id) -> Option<&Document>`, `state(id) -> Result<DocumentState, CommandError>`, `execute(id, Command) -> Result<Dirty, CommandError>`, `undo(id) -> Result<Dirty, CommandError>`, `redo(id)`, `import_image(id: Option<Uuid>, bytes: &[u8], name: &str, at: Option<Point>) -> Result<Uuid, CommandError>` (Task 9 fills the placement; Task 6 declares it), `export_png(id)`, `export_jpeg(id, quality, matte)`, `composite(id, region, w, h)`, `document_ids() -> Vec<Uuid>`.
- `ops::layers`: `add_blank_layer(doc: &mut Document)`, `rename_layer(doc, id, name) -> Result<(), CommandError>`, `set_layer_visible`, `delete_layer`, `set_active_layer`, `next_layer_name(doc) -> String` ("Layer N", first unused N from 1).

- [ ] **Step 1: Write the failing tests**

`engine/tests/engine.rs`:
```rust
mod fixtures;
use compositor_engine::*;

#[test]
fn new_document_with_empty_layer_and_state() {
    let mut e = Engine::new();
    let id = e.new_document(1920, 1080, true).unwrap();
    let s = e.state(id).unwrap();
    assert_eq!((s.width, s.height), (1920, 1080));
    assert_eq!(s.layers.len(), 1);
    assert_eq!(s.layers[0].name, "Layer 1");
    assert_eq!(s.active_layer_id, Some(s.layers[0].id));
    assert!(!s.can_undo && !s.is_modified);
    assert_eq!(e.new_document(0, 10, false).unwrap_err(), CommandError::Argument("width and height must be 1 to 30000".into()));
}

#[test]
fn commands_are_undoable_and_track_modification() {
    let mut e = Engine::new();
    let id = e.new_document(100, 80, true).unwrap();
    let layer = e.state(id).unwrap().layers[0].id;
    let dirty = e.execute(id, Command::RenameLayer { id: layer, name: "Edited".into() }).unwrap();
    assert!(dirty.structure);
    assert_eq!(e.state(id).unwrap().layers[0].name, "Edited");
    assert!(e.state(id).unwrap().is_modified);
    e.undo(id).unwrap();
    assert_eq!(e.state(id).unwrap().layers[0].name, "Layer 1");
    assert!(!e.state(id).unwrap().is_modified);
    e.redo(id).unwrap();
    assert_eq!(e.state(id).unwrap().layers[0].name, "Edited");
    e.mark_saved(id, Some("C:/x/Test.comp".into()));
    assert!(!e.state(id).unwrap().is_modified);
    assert_eq!(e.state(id).unwrap().path.as_deref(), Some("C:/x/Test.comp"));
}

#[test]
fn blank_layers_number_from_first_free_name() {
    let mut e = Engine::new();
    let id = e.new_document(10, 10, true).unwrap();
    e.execute(id, Command::AddBlankLayer).unwrap();
    e.execute(id, Command::AddBlankLayer).unwrap();
    let names: Vec<_> = e.state(id).unwrap().layers.iter().map(|l| l.name.clone()).collect();
    assert_eq!(names, ["Layer 1", "Layer 2", "Layer 3"]);
    let second = e.state(id).unwrap().layers[1].id;
    e.execute(id, Command::DeleteLayer { id: second }).unwrap();
    e.execute(id, Command::AddBlankLayer).unwrap();
    let names: Vec<_> = e.state(id).unwrap().layers.iter().map(|l| l.name.clone()).collect();
    assert_eq!(names, ["Layer 1", "Layer 3", "Layer 2"]);
    // New layer goes just above the active layer.
    let active = e.state(id).unwrap().active_layer_id.unwrap();
    assert_eq!(active, e.state(id).unwrap().layers[2].id);
}

#[test]
fn visibility_delete_and_active_layer() {
    let mut e = Engine::new();
    let id = e.new_document(10, 10, true).unwrap();
    let l1 = e.state(id).unwrap().layers[0].id;
    e.execute(id, Command::SetLayerVisible { id: l1, visible: false }).unwrap();
    assert!(!e.state(id).unwrap().layers[0].visible);
    e.execute(id, Command::DeleteLayer { id: l1 }).unwrap();
    assert!(e.state(id).unwrap().layers.is_empty());
    assert_eq!(e.state(id).unwrap().active_layer_id, None);
    assert_eq!(e.execute(id, Command::DeleteLayer { id: l1 }).unwrap_err(), CommandError::NoLayer);
    assert_eq!(e.execute(uuid::Uuid::new_v4(), Command::AddBlankLayer).unwrap_err(), CommandError::NoDocument);
}

#[test]
fn open_and_save_round_trip_through_engine_resets_history() {
    let mut e = Engine::new();
    let id = e.new_document(64, 32, true).unwrap();
    e.execute(id, Command::AddBlankLayer).unwrap();
    let pkg = e.save_package(id).unwrap();
    let reopened = e.open_package(&pkg, Some("C:/p/A.comp".into())).unwrap();
    let s = e.state(reopened).unwrap();
    assert_eq!(s.layers.len(), 2);
    assert!(!s.can_undo && !s.is_modified);
    assert_eq!(e.document_ids().len(), 2);
    e.close_document(id);
    assert_eq!(e.document_ids(), vec![reopened]);
}

#[test]
fn command_json_shape() {
    let c: Command = serde_json::from_str(r#"{"type":"RenameLayer","id":"E621E1F8-C36C-495A-93FC-0C247A3E6E5F","name":"X"}"#).unwrap();
    assert!(matches!(c, Command::RenameLayer { .. }));
    let s = serde_json::to_string(&Command::AddBlankLayer).unwrap();
    assert_eq!(s, r#"{"type":"AddBlankLayer"}"#);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p compositor-engine --test engine`
Expected: compile error.

- [ ] **Step 3: Implement history**

`engine/src/history.rs`:
```rust
use crate::Document;

/// Whole-document snapshots. Rasters are shared, so a snapshot costs only the layer metadata.
#[derive(Debug, Default)]
pub struct History {
    undo: Vec<Document>,
    redo: Vec<Document>,
    saved_depth: usize,
}

impl History {
    pub fn push(&mut self, before: Document) {
        self.undo.push(before);
        self.redo.clear();
        // The saved state can no longer be reached by redo once the future is discarded.
        if self.saved_depth > self.undo.len() { self.saved_depth = usize::MAX; }
    }
    pub fn undo(&mut self, current: &Document) -> Option<Document> {
        let before = self.undo.pop()?;
        self.redo.push(current.clone());
        Some(before)
    }
    pub fn redo(&mut self, current: &Document) -> Option<Document> {
        let after = self.redo.pop()?;
        self.undo.push(current.clone());
        Some(after)
    }
    pub fn can_undo(&self) -> bool { !self.undo.is_empty() }
    pub fn can_redo(&self) -> bool { !self.redo.is_empty() }
    pub fn mark_saved(&mut self) { self.saved_depth = self.undo.len(); }
    pub fn is_modified(&self) -> bool { self.saved_depth != self.undo.len() }
    pub fn reset(&mut self) { self.undo.clear(); self.redo.clear(); self.saved_depth = 0; }
}
```

- [ ] **Step 4: Implement commands and layer ops**

`engine/src/command.rs`:
```rust
use crate::{ids, Sampling};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Command {
    AddBlankLayer,
    RenameLayer { #[serde(with = "ids::upper")] id: Uuid, name: String },
    SetLayerVisible { #[serde(with = "ids::upper")] id: Uuid, visible: bool },
    DeleteLayer { #[serde(with = "ids::upper")] id: Uuid },
    SetActiveLayer { #[serde(with = "ids::upper_opt")] id: Option<Uuid> },
    CanvasSize { width: u32, height: u32, anchor: u8, fill: Option<[f64; 3]> },
    Crop { x: f64, y: f64, width: f64, height: f64 },
    ImageSize { width: u32, height: u32, resolution: f64, sampling: Sampling },
    FlipCanvas { horizontal: bool },
}

impl Command {
    pub fn action_name(&self) -> &'static str {
        match self {
            Command::AddBlankLayer => "New Layer",
            Command::RenameLayer { .. } => "Rename Layer",
            Command::SetLayerVisible { .. } => "Layer Visibility",
            Command::DeleteLayer { .. } => "Delete Layer",
            Command::SetActiveLayer { .. } => "Select Layer",
            Command::CanvasSize { .. } => "Canvas Size",
            Command::Crop { .. } => "Crop",
            Command::ImageSize { .. } => "Image Size",
            Command::FlipCanvas { horizontal: true } => "Flip Canvas Horizontal",
            Command::FlipCanvas { horizontal: false } => "Flip Canvas Vertical",
        }
    }
}

/// What a command changed, so the renderer re-syncs only what it must.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Dirty {
    /// Layer list, names, order, visibility or transforms changed.
    pub structure: bool,
    /// Canvas size or resolution changed.
    pub canvas: bool,
    /// Layers whose pixels were replaced.
    #[serde(serialize_with = "serialize_ids", deserialize_with = "deserialize_ids")]
    pub layers: Vec<Uuid>,
}

fn serialize_ids<S: serde::Serializer>(ids: &[Uuid], s: S) -> Result<S::Ok, S::Error> {
    use serde::Serialize;
    ids.iter().map(ids::upper_string).collect::<Vec<_>>().serialize(s)
}
fn deserialize_ids<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<Uuid>, D::Error> {
    use serde::Deserialize;
    let v: Vec<String> = Vec::deserialize(d)?;
    v.iter().map(|t| Uuid::parse_str(t).map_err(serde::de::Error::custom)).collect()
}

impl Dirty {
    pub fn everything() -> Dirty { Dirty { structure: true, canvas: true, layers: vec![] } }
    pub fn structure() -> Dirty { Dirty { structure: true, ..Default::default() } }
}
```

`engine/src/ops/mod.rs`:
```rust
pub mod layers;
```

`engine/src/ops/layers.rs`:
```rust
use crate::{CommandError, Document, Layer, MAX_LAYERS, MAX_NESTING};
use uuid::Uuid;

pub fn next_layer_name(doc: &Document) -> String {
    let mut n = 1;
    while doc.layers.iter().any(|l| l.name == format!("Layer {n}")) { n += 1; }
    format!("Layer {n}")
}

fn is_inside(doc: &Document, id: Uuid, folder: Uuid) -> bool {
    let mut parent = doc.layer(id).and_then(|l| l.parent_id);
    let mut steps = 0;
    while let Some(p) = parent {
        if p == folder { return true; }
        steps += 1;
        if steps > MAX_NESTING { return false; }
        parent = doc.layer(p).and_then(|l| l.parent_id);
    }
    false
}

/// Inserts a blank layer above the active layer (or at the top), inside the active folder if one is active.
pub fn add_blank_layer(doc: &mut Document) -> Result<Uuid, CommandError> {
    if doc.layers.len() >= MAX_LAYERS { return Err(CommandError::Argument("too many layers".into())); }
    let mut layer = Layer::blank(&next_layer_name(doc), doc.size());
    let active = doc.active_layer_id.and_then(|id| doc.layer(id).cloned());
    layer.parent_id = match &active { Some(a) if a.is_group => Some(a.id), Some(a) => a.parent_id, None => None };
    let mut insertion = doc.active_layer_id.and_then(|id| doc.index_of(id)).map(|i| i + 1).unwrap_or(doc.layers.len());
    if let Some(a) = &active {
        if a.is_group {
            if let Some(top) = doc.layers.iter().rposition(|l| is_inside(doc, l.id, a.id)) { insertion = insertion.max(top + 1); }
        }
    }
    let id = layer.id;
    doc.layers.insert(insertion, layer);
    doc.active_layer_id = Some(id);
    Ok(id)
}

pub fn rename_layer(doc: &mut Document, id: Uuid, name: &str) -> Result<(), CommandError> {
    if name.trim().is_empty() || name.len() > 16_384 { return Err(CommandError::Argument("layer name is empty or too long".into())); }
    doc.layer_mut(id).ok_or(CommandError::NoLayer)?.name = name.to_string();
    Ok(())
}

pub fn set_layer_visible(doc: &mut Document, id: Uuid, visible: bool) -> Result<(), CommandError> {
    doc.layer_mut(id).ok_or(CommandError::NoLayer)?.visible = visible;
    Ok(())
}

/// Removes the layer and, for a folder, everything inside it. The active layer moves to the layer below.
pub fn delete_layer(doc: &mut Document, id: Uuid) -> Result<(), CommandError> {
    let index = doc.index_of(id).ok_or(CommandError::NoLayer)?;
    let removed: Vec<Uuid> = doc.layers.iter().filter(|l| l.id == id || is_inside(doc, l.id, id)).map(|l| l.id).collect();
    doc.layers.retain(|l| !removed.contains(&l.id));
    for layer in &mut doc.layers {
        if layer.mask_source_id.map_or(false, |s| removed.contains(&s)) { layer.mask_source_id = None; }
    }
    if doc.active_layer_id.map_or(false, |a| removed.contains(&a)) {
        doc.active_layer_id = if doc.layers.is_empty() { None } else { Some(doc.layers[index.min(doc.layers.len() - 1).saturating_sub(if index >= doc.layers.len() { 0 } else { 0 })].id) };
        if index > 0 && index <= doc.layers.len() { doc.active_layer_id = Some(doc.layers[index - 1].id); }
    }
    Ok(())
}

pub fn set_active_layer(doc: &mut Document, id: Option<Uuid>) -> Result<(), CommandError> {
    if let Some(id) = id { doc.layer(id).ok_or(CommandError::NoLayer)?; }
    doc.active_layer_id = id;
    Ok(())
}
```

- [ ] **Step 5: Implement the Engine facade**

`engine/src/engine.rs`:
```rust
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

pub struct Session {
    pub document: Document,
    pub history: History,
    pub path: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayerState {
    #[serde(with = "ids::upper")] pub id: Uuid,
    pub name: String,
    pub visible: bool,
    pub is_group: bool,
    #[serde(with = "ids::upper_opt")] pub parent_id: Option<Uuid>,
    pub opacity: f64,
    pub blend_mode: BlendMode,
    pub transform: LayerTransform,
    pub pixels_width: u32,
    pub pixels_height: u32,
    pub pixels_revision: u64,
    pub has_mask: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentState {
    #[serde(with = "ids::upper")] pub id: Uuid,
    pub width: u32,
    pub height: u32,
    pub resolution: f64,
    #[serde(with = "ids::upper_opt")] pub active_layer_id: Option<Uuid>,
    pub can_undo: bool,
    pub can_redo: bool,
    pub is_modified: bool,
    pub path: Option<String>,
    pub layers: Vec<LayerState>,
}

#[derive(Default)]
pub struct Engine { sessions: HashMap<Uuid, Session>, order: Vec<Uuid> }

fn check_dimensions(width: u32, height: u32) -> Result<(), CommandError> {
    if !(1..=MAX_SIDE as u32).contains(&width) || !(1..=MAX_SIDE as u32).contains(&height) {
        return Err(CommandError::Argument("width and height must be 1 to 30000".into()));
    }
    Ok(())
}

impl Engine {
    pub fn new() -> Engine { Engine::default() }
    pub fn version() -> &'static str { env!("CARGO_PKG_VERSION") }

    fn insert(&mut self, document: Document, path: Option<String>) -> Uuid {
        let id = document.id;
        self.sessions.insert(id, Session { document, history: History::default(), path });
        self.order.push(id);
        id
    }
    fn session(&self, id: Uuid) -> Result<&Session, CommandError> { self.sessions.get(&id).ok_or(CommandError::NoDocument) }
    fn session_mut(&mut self, id: Uuid) -> Result<&mut Session, CommandError> { self.sessions.get_mut(&id).ok_or(CommandError::NoDocument) }

    pub fn document_ids(&self) -> Vec<Uuid> { self.order.clone() }
    pub fn document(&self, id: Uuid) -> Option<&Document> { self.sessions.get(&id).map(|s| &s.document) }

    pub fn new_document(&mut self, width: u32, height: u32, empty_layer: bool) -> Result<Uuid, CommandError> {
        check_dimensions(width, height)?;
        let mut doc = Document::new(width, height);
        if empty_layer {
            let layer = Layer::blank("Layer 1", doc.size());
            doc.active_layer_id = Some(layer.id);
            doc.layers.push(layer);
        }
        Ok(self.insert(doc, None))
    }

    pub fn open_package(&mut self, pkg: &Package, path: Option<String>) -> Result<Uuid, CommandError> {
        let doc = package::open_package(pkg)?;
        // Same package opened twice gets a fresh session id? No: keep the document id from the file, but do not
        // allow two sessions with one id.
        if self.sessions.contains_key(&doc.id) { self.close_document(doc.id); }
        Ok(self.insert(doc, path))
    }

    pub fn save_package(&self, id: Uuid) -> Result<Package, CommandError> {
        Ok(package::save_package(&self.session(id)?.document)?)
    }
    pub fn mark_saved(&mut self, id: Uuid, path: Option<String>) {
        if let Some(s) = self.sessions.get_mut(&id) { s.history.mark_saved(); if path.is_some() { s.path = path; } }
    }
    pub fn close_document(&mut self, id: Uuid) {
        self.sessions.remove(&id);
        self.order.retain(|d| *d != id);
    }

    pub fn state(&self, id: Uuid) -> Result<DocumentState, CommandError> {
        let s = self.session(id)?;
        let d = &s.document;
        Ok(DocumentState {
            id: d.id, width: d.width, height: d.height, resolution: d.resolution, active_layer_id: d.active_layer_id,
            can_undo: s.history.can_undo(), can_redo: s.history.can_redo(), is_modified: s.history.is_modified(),
            path: s.path.clone(),
            layers: d.layers.iter().map(|l| LayerState {
                id: l.id, name: l.name.clone(), visible: l.visible, is_group: l.is_group, parent_id: l.parent_id,
                opacity: l.opacity, blend_mode: l.blend_mode, transform: l.transform,
                pixels_width: l.pixels.as_ref().map_or(0, |p| p.width), pixels_height: l.pixels.as_ref().map_or(0, |p| p.height),
                pixels_revision: l.pixels_revision, has_mask: l.mask.is_some(),
            }).collect(),
        })
    }

    /// Runs `f` on a copy of the document; on success the copy replaces it and the original goes to history.
    fn edit<F>(&mut self, id: Uuid, f: F) -> Result<Dirty, CommandError>
    where F: FnOnce(&mut Document) -> Result<Dirty, CommandError> {
        let s = self.session_mut(id)?;
        let mut next = s.document.clone();
        let dirty = f(&mut next)?;
        if next != s.document {
            let before = std::mem::replace(&mut s.document, next);
            s.history.push(before);
        }
        Ok(dirty)
    }

    pub fn execute(&mut self, id: Uuid, command: Command) -> Result<Dirty, CommandError> {
        self.edit(id, |doc| match command {
            Command::AddBlankLayer => { ops::layers::add_blank_layer(doc)?; Ok(Dirty::structure()) }
            Command::RenameLayer { id, name } => { ops::layers::rename_layer(doc, id, &name)?; Ok(Dirty::structure()) }
            Command::SetLayerVisible { id, visible } => { ops::layers::set_layer_visible(doc, id, visible)?; Ok(Dirty::structure()) }
            Command::DeleteLayer { id } => { ops::layers::delete_layer(doc, id)?; Ok(Dirty::structure()) }
            Command::SetActiveLayer { id } => { ops::layers::set_active_layer(doc, id)?; Ok(Dirty::structure()) }
            Command::CanvasSize { .. } | Command::Crop { .. } | Command::ImageSize { .. } | Command::FlipCanvas { .. } =>
                Err(CommandError::Argument("not implemented".into())),
        })
    }

    pub fn undo(&mut self, id: Uuid) -> Result<Dirty, CommandError> {
        let s = self.session_mut(id)?;
        if let Some(before) = s.history.undo(&s.document) { s.document = before; }
        Ok(Dirty::everything())
    }
    pub fn redo(&mut self, id: Uuid) -> Result<Dirty, CommandError> {
        let s = self.session_mut(id)?;
        if let Some(after) = s.history.redo(&s.document) { s.document = after; }
        Ok(Dirty::everything())
    }

    pub fn import_image(&mut self, _id: Option<Uuid>, _bytes: &[u8], _name: &str, _at: Option<Point>) -> Result<Uuid, CommandError> {
        Err(CommandError::Argument("not implemented".into())) // Task 9
    }

    pub fn export_png(&self, id: Uuid) -> Result<Vec<u8>, CommandError> { Ok(compositor::export_png(&self.session(id)?.document)?) }
    pub fn export_jpeg(&self, id: Uuid, quality: f64, matte: [f64; 3]) -> Result<Vec<u8>, CommandError> {
        Ok(compositor::export_jpeg(&self.session(id)?.document, quality, matte)?)
    }
    pub fn composite(&self, id: Uuid, region: Rect, width: u32, height: u32) -> Result<Raster, CommandError> {
        Ok(compositor::composite(&self.session(id)?.document, region, width, height))
    }
}
```

Note for `delete_layer`: the active layer after deletion is the layer that was directly below the removed one (index − 1), or the new layer at the same index when the bottom layer was removed, or none. Simplify the two lines that set `active_layer_id` to exactly that rule before running the tests; the sketch above is deliberately verbose and should be replaced by:

```rust
    if doc.active_layer_id.map_or(false, |a| removed.contains(&a)) {
        doc.active_layer_id = if doc.layers.is_empty() { None }
            else if index > 0 { Some(doc.layers[(index - 1).min(doc.layers.len() - 1)].id) }
            else { Some(doc.layers[0].id) };
    }
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test -p compositor-engine`
Expected: all pass, including the 6 in `engine.rs`.

- [ ] **Step 7: Commit**

```
git add engine
git commit -m "feat(engine): history, command enum, layer ops and Engine facade"
```

---

### Task 7: Canvas Size, Crop and Flip Canvas

**Files:**
- Create: `engine/src/ops/canvas_size.rs`, `engine/src/ops/flip.rs`
- Modify: `engine/src/ops/mod.rs` (add `pub mod canvas_size; pub mod flip;`), `engine/src/engine.rs` (wire `CanvasSize`, `Crop`, `FlipCanvas`)
- Test: `engine/tests/canvas_size.rs`

**Interfaces:**
- Produces: `CanvasSizeOptions { width: u32, height: u32, anchor: u8 (0..=8, row-major, 4 = center), fill: Option<[f64;3]>, content_offset: Option<Point> }` with `offset(from_width, from_height) -> Point`; `canvas_size(doc: &Document, options) -> Result<Document, ProjectError>` (pure: returns the new document; layers keep their rasters; a fill adds a bottom layer named "Canvas Extension" with the old canvas rectangle cleared); `flip_canvas(doc: &mut Document, horizontal: bool)`.
- Engine: `Command::CanvasSize` maps to `canvas_size` with `content_offset: None`; `Command::Crop { x, y, width, height }` maps to `canvas_size` with `width`, `height` and `content_offset = (-x, -y)`; both return `Dirty::everything()`; `Command::FlipCanvas` returns `Dirty::structure()`.

- [ ] **Step 1: Write the failing tests**

`engine/tests/canvas_size.rs`:
```rust
mod fixtures;
use compositor_engine::*;
use compositor_engine::ops::canvas_size::*;
use compositor_engine::ops::flip::flip_canvas;
use fixtures::*;

fn imported() -> Document {
    let mut d = Document::new(64, 32);
    let mut layer = Layer::with_pixels("Red", red_left_raster(), Point { x: 0.0, y: 0.0 });
    layer.transform.rotation = 37.0;
    layer.transform.flip_x = true;
    d.active_layer_id = Some(layer.id);
    d.layers.push(layer);
    d
}

#[test]
fn every_anchor_preserves_source_and_transform() {
    let source = imported();
    let transform = source.layers[0].transform;
    for delta in [5i32, -5] {
        for anchor in 0..9u8 {
            let out = canvas_size(&source, CanvasSizeOptions { width: (64 + delta) as u32, height: (32 + delta) as u32, anchor, fill: None, content_offset: None }).unwrap();
            let layer = &out.layers[0];
            let expected = [0, if delta == 5 { 2 } else { -3 }, delta];
            assert_eq!(layer.transform.origin.x, transform.origin.x + expected[(anchor % 3) as usize] as f64, "anchor {anchor} delta {delta}");
            assert_eq!(layer.transform.origin.y, transform.origin.y + expected[(anchor / 3) as usize] as f64);
            assert_eq!(layer.transform.size, transform.size);
            assert_eq!(layer.transform.rotation, 37.0);
            assert!(layer.transform.flip_x);
            assert_eq!(layer.id, source.layers[0].id);
            assert!(layer.pixels.as_ref().unwrap().same_pixels(source.layers[0].pixels.as_ref().unwrap()));
        }
    }
}

#[test]
fn colored_extension_keeps_old_area_transparent() {
    let mut d = Document::new(4, 4);
    d.layers.push(Layer::blank("Layer 1", d.size()));
    d.active_layer_id = Some(d.layers[0].id);
    let out = canvas_size(&d, CanvasSizeOptions { width: 8, height: 2, anchor: 4, fill: Some([1.0, 0.0, 0.0]), content_offset: None }).unwrap();
    assert_eq!(out.layers.len(), 2);
    assert_eq!(out.layers[0].name, "Canvas Extension");
    assert_eq!(out.active_layer_id, d.active_layer_id);
    let png = export_png(&out).unwrap();
    let img = decode_image(&png).unwrap().raster;
    assert_eq!(img.pixel(0, 0)[3], 255);
    assert!(img.pixel(0, 0)[0] > 252);
    assert_eq!(img.pixel(3, 0)[3], 0);
    assert_eq!(img.pixel(7, 1)[3], 255);
}

#[test]
fn transparent_resize_allocates_nothing_and_shrink_adds_no_fill() {
    let d = Document::new(4, 4);
    let large = canvas_size(&d, CanvasSizeOptions { width: 30_000, height: 30_000, anchor: 4, fill: None, content_offset: None }).unwrap();
    assert!(large.layers.is_empty());
    let small = canvas_size(&d, CanvasSizeOptions { width: 2, height: 2, anchor: 4, fill: Some([1.0; 3]), content_offset: None }).unwrap();
    assert!(small.layers.is_empty());
    let err = canvas_size(&d, CanvasSizeOptions { width: 30_000, height: 30_000, anchor: 4, fill: Some([1.0; 3]), content_offset: None }).unwrap_err();
    assert_eq!(err, ProjectError::TooLarge);
}

#[test]
fn crop_translates_without_resampling_and_undo_restores() {
    let mut e = Engine::new();
    let id = e.new_document(64, 32, false).unwrap();
    let raster = red_left_raster();
    let doc = e.document(id).unwrap().clone();
    // Seed a layer directly through the engine's import path substitute: use canvas_size on a hand-built doc.
    let mut seeded = doc.clone();
    seeded.layers.push(Layer::with_pixels("Red", raster.clone(), Point { x: 0.0, y: 0.0 }));
    let pkg = save_package(&seeded).unwrap();
    let id = e.open_package(&pkg, None).unwrap();
    e.execute(id, Command::Crop { x: 8.0, y: 4.0, width: 32.0, height: 16.0 }).unwrap();
    let s = e.state(id).unwrap();
    assert_eq!((s.width, s.height), (32, 16));
    assert_eq!(s.layers[0].transform.origin, Point { x: -8.0, y: -4.0 });
    assert_eq!(s.layers[0].pixels_revision, 1, "pixels untouched");
    e.undo(id).unwrap();
    let s = e.state(id).unwrap();
    assert_eq!((s.width, s.height), (64, 32));
    assert_eq!(s.layers[0].transform.origin, Point { x: 0.0, y: 0.0 });
}

#[test]
fn same_size_offset_crop_and_expansion_use_exact_bounds() {
    let mut e = Engine::new();
    let id = e.new_document(100, 50, true).unwrap();
    e.execute(id, Command::Crop { x: -20.0, y: 10.0, width: 100.0, height: 50.0 }).unwrap();
    assert_eq!(e.state(id).unwrap().layers[0].transform.origin, Point { x: 20.0, y: -10.0 });
    e.execute(id, Command::Crop { x: -10.0, y: -10.0, width: 140.0, height: 80.0 }).unwrap();
    let s = e.state(id).unwrap();
    assert_eq!((s.width, s.height), (140, 80));
    assert_eq!(s.layers[0].transform.origin, Point { x: 30.0, y: 0.0 });
    let img = decode_image(&e.export_png(id).unwrap()).unwrap().raster;
    assert_eq!((img.width, img.height), (140, 80));
    assert_eq!(img.pixel(0, 0)[3], 0);
}

#[test]
fn flip_canvas_mirrors_every_layer() {
    let mut d = imported();
    d.layers.push(Layer::blank("Layer 2", d.size()));
    let before = d.layers[0].transform;
    flip_canvas(&mut d, true);
    let l = &d.layers[0].transform;
    assert_eq!(l.flip_x, !before.flip_x);
    assert_eq!(l.rotation, -before.rotation);
    // Center x 32 stays at 32 on a 64-wide canvas; origin unchanged.
    assert_eq!(l.origin.x, before.origin.x);
    let blank = &d.layers[1].transform;
    assert!(blank.flip_x && blank.origin.x == 0.0);
    flip_canvas(&mut d, false);
    assert!(d.layers[0].transform.flip_y);
    assert_eq!(d.layers[0].transform.rotation, before.rotation);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p compositor-engine --test canvas_size`
Expected: compile error, `ops::canvas_size` missing.

- [ ] **Step 3: Implement canvas size and flip**

`engine/src/ops/canvas_size.rs`:
```rust
use crate::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasSizeOptions {
    pub width: u32,
    pub height: u32,
    /// Row-major, top-left (0) through bottom-right (8); 4 is the center.
    pub anchor: u8,
    pub fill: Option<[f64; 3]>,
    /// Crop supplies an explicit document-space translation instead of an anchor.
    pub content_offset: Option<Point>,
}

impl CanvasSizeOptions {
    pub fn offset(&self, from_width: u32, from_height: u32) -> Point {
        if let Some(o) = self.content_offset { return o; }
        // Floor puts the extra pixel on the right/bottom when expanding and removes it from the left/top when shrinking.
        Point {
            x: ((self.width as f64 - from_width as f64) * (self.anchor % 3) as f64 / 2.0).floor(),
            y: ((self.height as f64 - from_height as f64) * (self.anchor / 3) as f64 / 2.0).floor(),
        }
    }
}

pub fn canvas_size(doc: &Document, options: CanvasSizeOptions) -> Result<Document, ProjectError> {
    let w = options.width as i64; let h = options.height as i64;
    if !(1..=MAX_SIDE).contains(&w) || !(1..=MAX_SIDE).contains(&h) || options.anchor > 8 { return Err(ProjectError::TooLarge); }
    let offset = options.offset(doc.width, doc.height);
    if !offset.x.is_finite() || !offset.y.is_finite() || offset.x.abs() > 1_000_000.0 || offset.y.abs() > 1_000_000.0 {
        return Err(ProjectError::Invalid);
    }
    if options.width == doc.width && options.height == doc.height && offset == (Point { x: 0.0, y: 0.0 }) {
        return Ok(doc.clone());
    }
    let mut out = doc.clone();
    out.width = options.width;
    out.height = options.height;
    for layer in &mut out.layers {
        layer.transform.origin.x += offset.x;
        layer.transform.origin.y += offset.y;
        if !layer.transform.is_valid() { return Err(ProjectError::TooLarge); }
        if let Some(mask) = &mut layer.mask {
            if let Some(p) = &mut mask.placement { p.origin.x += offset.x; p.origin.y += offset.y; }
        }
    }
    // A colored extension is separate bottom-layer content; the old canvas area stays transparent.
    if let Some(color) = options.fill {
        if options.width > doc.width || options.height > doc.height {
            let pixels = options.width as u64 * options.height as u64;
            if pixels > MAX_PIXELS - doc.used_pixels() || out.layers.len() >= MAX_LAYERS { return Err(ProjectError::TooLarge); }
            if color.iter().any(|c| !c.is_finite() || !(0.0..=1.0).contains(c)) { return Err(ProjectError::Invalid); }
            let rgb = color.map(|c| (c * 255.0).round() as u8);
            let mut data = vec![0u8; (pixels * 4) as usize];
            let hole_x0 = offset.x.max(0.0) as i64; let hole_y0 = offset.y.max(0.0) as i64;
            let hole_x1 = (offset.x + doc.width as f64).min(options.width as f64) as i64;
            let hole_y1 = (offset.y + doc.height as f64).min(options.height as f64) as i64;
            for y in 0..options.height as i64 {
                for x in 0..options.width as i64 {
                    if x >= hole_x0 && x < hole_x1 && y >= hole_y0 && y < hole_y1 { continue; }
                    let i = ((y * options.width as i64 + x) * 4) as usize;
                    data[i] = rgb[0]; data[i + 1] = rgb[1]; data[i + 2] = rgb[2]; data[i + 3] = 255;
                }
            }
            let raster = Raster::from_premultiplied(options.width, options.height, data);
            out.layers.insert(0, Layer::with_pixels("Canvas Extension", raster, Point { x: 0.0, y: 0.0 }));
        }
    }
    Ok(out)
}
```

`engine/src/ops/flip.rs`:
```rust
use crate::Document;

/// Mirrors every layer, folder and placed mask across the canvas middle.
pub fn flip_canvas(doc: &mut Document, horizontal: bool) {
    let axis = if horizontal { doc.width as f64 / 2.0 } else { doc.height as f64 / 2.0 };
    for layer in &mut doc.layers {
        layer.transform = layer.transform.mirrored(horizontal, axis);
        if let Some(mask) = &mut layer.mask {
            if let Some(p) = mask.placement { mask.placement = Some(p.mirrored(horizontal, axis)); }
        }
    }
}
```

Wire the commands in `engine/src/engine.rs` `execute`:
```rust
            Command::CanvasSize { width, height, anchor, fill } => {
                *doc = ops::canvas_size::canvas_size(doc, ops::canvas_size::CanvasSizeOptions { width, height, anchor, fill, content_offset: None })?;
                Ok(Dirty::everything())
            }
            Command::Crop { x, y, width, height } => {
                if ![x, y, width, height].iter().all(|v| v.is_finite()) || width < 1.0 || height < 1.0 || width > MAX_SIDE as f64 || height > MAX_SIDE as f64 {
                    return Err(CommandError::Argument("crop rectangle out of range".into()));
                }
                *doc = ops::canvas_size::canvas_size(doc, ops::canvas_size::CanvasSizeOptions {
                    width: width.round() as u32, height: height.round() as u32, anchor: 4, fill: None,
                    content_offset: Some(Point { x: -x.round(), y: -y.round() }) })?;
                Ok(Dirty::everything())
            }
            Command::FlipCanvas { horizontal } => { ops::flip::flip_canvas(doc, horizontal); Ok(Dirty::structure()) }
```
and keep `Command::ImageSize { .. } => Err(CommandError::Argument("not implemented".into()))` until Task 8.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p compositor-engine`
Expected: all pass, including 6 in `canvas_size.rs`.

- [ ] **Step 5: Commit**

```
git add engine
git commit -m "feat(engine): canvas size, crop and flip canvas commands"
```

---

### Task 8: Image Size

**Files:**
- Create: `engine/src/ops/image_size.rs`
- Modify: `engine/src/ops/mod.rs` (add `pub mod image_size;`), `engine/src/engine.rs` (wire `ImageSize`)
- Test: `engine/tests/image_size.rs`

**Interfaces:**
- Produces: `ImageSizeOptions { width: u32, height: u32, resolution: f64, sampling: Sampling }`; `image_size(doc: &Document, options) -> Result<Document, ProjectError>`. Resolution-only changes keep every layer and raster untouched. Otherwise each layer is rasterized independently into its scaled, upright bounding box (rotation becomes 0, flips baked in) with the chosen sampling; blank layers only get their transform scaled; uniform 1x1 masks and masks with a placement are kept, other masks are resampled as coverage. Returns `Dirty::everything()` and bumps `pixels_revision` on resampled layers.

- [ ] **Step 1: Write the failing tests**

`engine/tests/image_size.rs`:
```rust
mod fixtures;
use compositor_engine::*;
use compositor_engine::ops::image_size::*;
use fixtures::*;

#[test]
fn resize_resamples_pixels_and_keeps_identity() {
    let mut d = Document::new(64, 32);
    let layer = Layer::with_pixels("Red", red_left_raster(), Point { x: 0.0, y: 0.0 });
    let id = layer.id;
    d.active_layer_id = Some(id);
    d.layers.push(layer);
    let out = image_size(&d, ImageSizeOptions { width: 128, height: 96, resolution: 300.0, sampling: Sampling::Nearest }).unwrap();
    assert_eq!((out.width, out.height, out.resolution), (128, 96, 300.0));
    assert_eq!(out.active_layer_id, Some(id));
    let px = out.layers[0].pixels.as_ref().unwrap();
    assert_eq!((px.width, px.height), (128, 96));
    assert!(px.pixel(0, 0)[0] > 242);
    assert_eq!(px.pixel(127, 0)[3], 0);
    assert_eq!(out.layers[0].pixels_revision, 2);
}

#[test]
fn resolution_only_keeps_pixels_and_transforms() {
    let mut d = Document::new(32, 16);
    d.layers.push(Layer::blank("Layer 1", d.size()));
    d.layers.push(Layer::with_pixels("Red", red_left_raster(), Point { x: -10.0, y: 3.0 }));
    let out = image_size(&d, ImageSizeOptions { width: 32, height: 16, resolution: 300.0, sampling: Sampling::High }).unwrap();
    assert_eq!(out.layers[0].transform, d.layers[0].transform);
    assert!(out.layers[1].pixels.as_ref().unwrap().same_pixels(d.layers[1].pixels.as_ref().unwrap()));
    assert_eq!(out.resolution, 300.0);
    let pkg = save_package(&out).unwrap();
    assert!(pkg.manifest_json.contains("\"resolution\": 300"));
    assert!((png_dpi(&export_png(&out).unwrap()).unwrap() - 300.0).abs() < 1.0);
}

#[test]
fn rotated_hidden_layer_scales_in_document_axes_and_invalid_size_is_rejected() {
    let mut d = Document::new(64, 32);
    let mut layer = Layer::with_pixels("Red", red_left_raster(), Point { x: -16.0, y: 4.0 });
    layer.transform.rotation = 90.0;
    layer.visible = false;
    d.layers.push(layer);
    let out = image_size(&d, ImageSizeOptions { width: 128, height: 96, resolution: 72.0, sampling: Sampling::Nearest }).unwrap();
    let t = out.layers[0].transform;
    assert!(!out.layers[0].visible);
    assert_eq!(t.rotation, 0.0);
    // A 90-degree 64x32 layer becomes 32x64, then scales 2x horizontally and 3x vertically.
    assert!((t.size.width - 64.0).abs() <= 1.0);
    assert!((t.size.height - 192.0).abs() <= 1.0);
    assert!(t.origin.y < 0.0);
    let err = image_size(&d, ImageSizeOptions { width: 30_000, height: 30_000, resolution: 72.0, sampling: Sampling::High }).unwrap_err();
    assert_eq!(err, ProjectError::TooLarge);
}

#[test]
fn image_size_command_is_undoable() {
    let mut e = Engine::new();
    let id = e.new_document(40, 20, true).unwrap();
    e.execute(id, Command::ImageSize { width: 80, height: 40, resolution: 72.0, sampling: Sampling::High }).unwrap();
    assert_eq!(e.state(id).unwrap().width, 80);
    assert_eq!(e.state(id).unwrap().layers[0].transform.size, Size { width: 80.0, height: 40.0 });
    e.undo(id).unwrap();
    assert_eq!(e.state(id).unwrap().width, 40);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p compositor-engine --test image_size`
Expected: compile error.

- [ ] **Step 3: Implement image size**

`engine/src/ops/image_size.rs`:
```rust
use crate::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImageSizeOptions { pub width: u32, pub height: u32, pub resolution: f64, pub sampling: Sampling }

/// Draws `layer` (with `sampling`) into an upright raster covering document rect `left,top,w,h` after the canvas
/// is scaled by `sx, sy`.
fn rasterize(layer: &Layer, raster: &Raster, sampling: Sampling, sx: f64, sy: f64, left: f64, top: f64, w: u32, h: u32) -> Raster {
    let mut probe = layer.clone();
    probe.transform.sampling = sampling;
    probe.opacity = 1.0;
    probe.visible = true;
    probe.parent_id = None;
    probe.set_pixels(Some(raster.clone()));
    // Output pixel (x, y) sits at document (left + x, top + y) in the new canvas, i.e. ((left + x) / sx, (top + y) / sy) in the old.
    let region = Rect { x: left / sx, y: top / sy, width: w as f64 / sx, height: h as f64 / sy };
    let mut target = vec![0u8; (w as usize) * (h as usize) * 4];
    compositor::render_layer(&mut target, w, h, region, &probe);
    Raster::from_premultiplied(w, h, target)
}

pub fn image_size(doc: &Document, options: ImageSizeOptions) -> Result<Document, ProjectError> {
    let w = options.width as i64; let h = options.height as i64;
    if !(1..=MAX_SIDE).contains(&w) || !(1..=MAX_SIDE).contains(&h) || !options.resolution.is_finite() || !(1.0..=9600.0).contains(&options.resolution) {
        return Err(ProjectError::TooLarge);
    }
    let mut out = doc.clone();
    out.resolution = options.resolution;
    if options.width == doc.width && options.height == doc.height { return Ok(out); }
    if (w * h) as u64 > MAX_PIXELS { return Err(ProjectError::TooLarge); }
    out.width = options.width;
    out.height = options.height;
    let sx = options.width as f64 / doc.width as f64;
    let sy = options.height as f64 / doc.height as f64;
    let mut used = 0u64; let mut used_masks = 0u64;
    for layer in &mut out.layers {
        let corners = layer.transform.corners().map(|p| Point { x: p.x * sx, y: p.y * sy });
        let left = corners.iter().map(|p| p.x).fold(f64::INFINITY, f64::min).floor();
        let top = corners.iter().map(|p| p.y).fold(f64::INFINITY, f64::min).floor();
        let right = corners.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max).ceil();
        let bottom = corners.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max).ceil();
        let width = (right - left).max(1.0) as u32;
        let height = (bottom - top).max(1.0) as u32;
        let mut transform = LayerTransform::axis_aligned(Point { x: left, y: top }, Size { width: width as f64, height: height as f64 });
        transform.sampling = options.sampling;
        if !transform.is_valid() { return Err(ProjectError::TooLarge); }
        if let Some(raster) = layer.pixels.clone() {
            let pixels = width as u64 * height as u64;
            if width as i64 > MAX_SIDE || height as i64 > MAX_SIDE || pixels > MAX_PIXELS - used { return Err(ProjectError::TooLarge); }
            used += pixels;
            let resampled = rasterize(layer, &raster, options.sampling, sx, sy, left, top, width, height);
            layer.set_pixels(Some(resampled));
        }
        if let Some(mask) = &mut layer.mask {
            let keep = (mask.pixels.width == 1 && mask.pixels.height == 1) || mask.placement.is_some();
            if !keep {
                let pixels = width as u64 * height as u64;
                if pixels > MAX_PIXELS - used_masks { return Err(ProjectError::TooLarge); }
                used_masks += pixels;
                // Coverage resamples like an opaque grey image: expand to RGBA, draw, take the red channel.
                let gray = &mask.pixels;
                let mut rgba = Vec::with_capacity(gray.bytes().len() * 4);
                for &v in gray.bytes() { rgba.extend_from_slice(&[v, v, v, 255]); }
                let as_raster = Raster::from_premultiplied(gray.width, gray.height, rgba);
                let drawn = rasterize(layer, &as_raster, options.sampling, sx, sy, left, top, width, height);
                let coverage: Vec<u8> = drawn.bytes().chunks_exact(4).map(|p| p[0]).collect();
                mask.pixels = GrayRaster::from_bytes(width, height, coverage);
            }
            if let Some(p) = &mut mask.placement {
                let c = p.corners().map(|q| Point { x: q.x * sx, y: q.y * sy });
                let l = c.iter().map(|q| q.x).fold(f64::INFINITY, f64::min);
                let t = c.iter().map(|q| q.y).fold(f64::INFINITY, f64::min);
                let r = c.iter().map(|q| q.x).fold(f64::NEG_INFINITY, f64::max);
                let b = c.iter().map(|q| q.y).fold(f64::NEG_INFINITY, f64::max);
                p.origin = Point { x: l, y: t };
                p.size = Size { width: (r - l).max(1.0), height: (b - t).max(1.0) };
            }
        }
        layer.transform = transform;
    }
    Ok(out)
}
```

Wire in `engine/src/engine.rs` `execute`:
```rust
            Command::ImageSize { width, height, resolution, sampling } => {
                *doc = ops::image_size::image_size(doc, ops::image_size::ImageSizeOptions { width, height, resolution, sampling })?;
                Ok(Dirty::everything())
            }
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p compositor-engine`
Expected: all pass, including 4 in `image_size.rs`.

- [ ] **Step 5: Commit**

```
git add engine
git commit -m "feat(engine): image size resampling with resolution"
```

---

### Task 9: Import images into a document

**Files:**
- Modify: `engine/src/ops/layers.rs` (add `import_raster`), `engine/src/engine.rs` (implement `import_image`)
- Test: `engine/tests/import.rs`

**Interfaces:**
- Produces: `ops::layers::import_raster(doc: &mut Document, raster: Raster, name: &str, at: Option<Point>) -> Result<Uuid, CommandError>` (placement: origin = floor(center − size/2) where center is `at` or the canvas center; parent follows the active layer's folder; the new layer becomes active; budget: raster pixels must fit in `MAX_PIXELS − used_pixels`). `Engine::import_image(id: Option<Uuid>, bytes, name, at)`: with no document id, decodes and creates a new document the size of the image, then imports at the center; with an id, decodes then imports as one undo step. Returns the document id.

- [ ] **Step 1: Write the failing tests**

`engine/tests/import.rs`:
```rust
mod fixtures;
use compositor_engine::*;
use fixtures::*;

#[test]
fn first_import_creates_a_document_of_the_image_size() {
    let mut e = Engine::new();
    let id = e.import_image(None, &red_left_png(), "photo", None).unwrap();
    let s = e.state(id).unwrap();
    assert_eq!((s.width, s.height), (64, 32));
    assert_eq!(s.layers.len(), 1);
    assert_eq!(s.layers[0].name, "photo");
    assert_eq!(s.layers[0].transform.origin, Point { x: 0.0, y: 0.0 });
    assert_eq!(s.active_layer_id, Some(s.layers[0].id));
    assert!(s.is_modified, "an imported document has unsaved content");
}

#[test]
fn import_into_document_centers_and_is_undoable() {
    let mut e = Engine::new();
    let id = e.new_document(128, 128, false).unwrap();
    e.import_image(Some(id), &red_left_png(), "photo", None).unwrap();
    let s = e.state(id).unwrap();
    assert_eq!(s.layers[0].transform.origin, Point { x: 32.0, y: 48.0 });
    e.import_image(Some(id), &red_left_png(), "photo", Some(Point { x: 300.0, y: 250.0 })).unwrap();
    assert_eq!(e.state(id).unwrap().layers[1].transform.origin, Point { x: 268.0, y: 234.0 });
    e.undo(id).unwrap();
    assert_eq!(e.state(id).unwrap().layers.len(), 1);
}

#[test]
fn unsupported_and_over_budget_imports_fail_without_changing_the_document() {
    let mut e = Engine::new();
    let id = e.new_document(10, 10, true).unwrap();
    let err = e.import_image(Some(id), b"not an image", "x", None).unwrap_err();
    assert_eq!(err, CommandError::Import(ImportError::Unreadable));
    assert_eq!(e.state(id).unwrap().layers.len(), 1);
    assert!(!e.state(id).unwrap().is_modified);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p compositor-engine --test import`
Expected: `not implemented` argument errors.

- [ ] **Step 3: Implement import**

Add to `engine/src/ops/layers.rs`:
```rust
pub fn import_raster(doc: &mut Document, raster: crate::Raster, name: &str, at: Option<crate::Point>) -> Result<Uuid, CommandError> {
    let pixels = raster.width as u64 * raster.height as u64;
    if raster.width as i64 > crate::MAX_SIDE || raster.height as i64 > crate::MAX_SIDE || pixels > crate::MAX_PIXELS - doc.used_pixels() {
        return Err(CommandError::Import(crate::ImportError::TooLarge));
    }
    if doc.layers.len() >= MAX_LAYERS { return Err(CommandError::Argument("too many layers".into())); }
    let center = at.unwrap_or(crate::Point { x: doc.width as f64 / 2.0, y: doc.height as f64 / 2.0 });
    let origin = crate::Point { x: (center.x - raster.width as f64 / 2.0).floor(), y: (center.y - raster.height as f64 / 2.0).floor() };
    let mut layer = Layer::with_pixels(name, raster, origin);
    let active = doc.active_layer_id.and_then(|id| doc.layer(id).cloned());
    layer.parent_id = match &active { Some(a) if a.is_group => Some(a.id), Some(a) => a.parent_id, None => None };
    let id = layer.id;
    doc.layers.push(layer);
    doc.active_layer_id = Some(id);
    Ok(id)
}
```

Replace `Engine::import_image` in `engine/src/engine.rs`:
```rust
    pub fn import_image(&mut self, id: Option<Uuid>, bytes: &[u8], name: &str, at: Option<Point>) -> Result<Uuid, CommandError> {
        let raster = decode_image(bytes)?.raster;
        match id {
            Some(id) => {
                self.edit(id, |doc| { ops::layers::import_raster(doc, raster, name, at)?; Ok(Dirty::structure()) })?;
                Ok(id)
            }
            None => {
                let mut doc = Document::new(raster.width, raster.height);
                ops::layers::import_raster(&mut doc, raster, name, None)?;
                let id = self.insert(doc, None);
                // A fresh import has content that is not on disk: record one history step so it reads as modified.
                let s = self.session_mut(id)?;
                s.history.push(Document::new(s.document.width, s.document.height));
                Ok(id)
            }
        }
    }
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p compositor-engine`
Expected: all pass.

- [ ] **Step 5: Commit**

```
git add engine
git commit -m "feat(engine): import images as layers or as a new document"
```

---

### Task 10: wasm wrapper, app skeleton and engine client

**Files:**
- Create: `engine-wasm/Cargo.toml`, `engine-wasm/src/lib.rs`, `package.json`, `vite.config.ts`, `vitest.config.ts`, `tsconfig.json`, `app/index.html`, `app/src/main.tsx`, `app/src/App.tsx`, `app/src/build-info.ts`, `app/src/engine/types.ts`, `app/src/engine/client.ts`, `app/src/test-api.ts`, `playwright.config.ts`, `app/tests/e2e/smoke.spec.ts`, `scripts/write-build-info.ps1`
- Modify: `Cargo.toml` (members add `engine-wasm`), `.gitignore` already ignores `app/src/engine/pkg/`

**Interfaces:**
- `WasmEngine` (wasm-bindgen class): `new()`, `version(): string`, `new_document(width, height, empty_layer): string` (uppercase uuid), `open_package(manifest: string, names: string[], blobs: Uint8Array[], path: string | null): string`, `save_package_manifest(doc): string`, `save_package_image_names(doc): string[]`, `save_package_image(doc, name): Uint8Array` (call `prepare_save(doc)` first; it caches the encoded package until `finish_save(doc)`), `mark_saved(doc, path: string | null)`, `close_document(doc)`, `document_ids(): string[]`, `state(doc): string` (DocumentState JSON), `execute(doc, command_json): string` (Dirty JSON), `undo(doc)`, `redo(doc)`, `import_image(doc: string | null, bytes: Uint8Array, name, x: number | null, y: number | null): string`, `export_png(doc): Uint8Array`, `export_jpeg(doc, quality, r, g, b): Uint8Array`, `composite(doc, x, y, w, h, out_w, out_h): Uint8Array`, `layer_pixels_ptr(doc, layer): number`, `layer_pixels_len(doc, layer): number`. Every failure throws a JS `Error` whose message is the engine error's Display text.
- `EngineClient` (TypeScript): same operations with typed `DocumentState`, `Command`, `Dirty`; `layerPixels(doc, layer): Uint8Array` returns a view on wasm memory (valid until the next engine call).
- `window.__compositor` test API (only when `import.meta.env.DEV` or `VITE_TEST_API=1`): `{ engine: EngineClient, store }`.

- [ ] **Step 1: Toolchain**

Run once:
```
rustup target add wasm32-unknown-unknown
cargo install wasm-pack --locked
```
If `cargo install` cannot reach GitHub for wasm-bindgen downloads, wasm-pack still works because it fetches `wasm-bindgen-cli` from crates.io; if the proxy blocks crates.io, set `CARGO_HTTP_PROXY` to the corporate proxy first.

- [ ] **Step 2: wasm crate**

`engine-wasm/Cargo.toml`:
```toml
[package]
name = "compositor-engine-wasm"
version.workspace = true
edition.workspace = true

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
compositor-engine = { path = "../engine" }
wasm-bindgen = "0.2"
js-sys = "0.3"
serde_json = "1"
uuid = { version = "1", features = ["js"] }
console_error_panic_hook = "0.1"
```

`engine-wasm/src/lib.rs`:
```rust
use compositor_engine::*;
use js_sys::{Array, Uint8Array};
use std::collections::HashMap;
use uuid::Uuid;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmEngine { engine: Engine, pending_saves: HashMap<Uuid, Package> }

fn js_err<E: std::fmt::Display>(e: E) -> JsError { JsError::new(&e.to_string()) }
fn parse_id(text: &str) -> Result<Uuid, JsError> { Uuid::parse_str(text).map_err(js_err) }

#[wasm_bindgen]
impl WasmEngine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmEngine {
        console_error_panic_hook::set_once();
        WasmEngine { engine: Engine::new(), pending_saves: HashMap::new() }
    }
    pub fn version(&self) -> String { Engine::version().to_string() }

    pub fn new_document(&mut self, width: u32, height: u32, empty_layer: bool) -> Result<String, JsError> {
        self.engine.new_document(width, height, empty_layer).map(|id| ids::upper_string(&id)).map_err(js_err)
    }
    pub fn open_package(&mut self, manifest: String, names: Vec<String>, blobs: Array, path: Option<String>) -> Result<String, JsError> {
        let images = names.into_iter().enumerate().map(|(i, name)| {
            let arr = Uint8Array::new(&blobs.get(i as u32));
            (name, arr.to_vec())
        }).collect();
        let pkg = Package { manifest_json: manifest, images };
        self.engine.open_package(&pkg, path).map(|id| ids::upper_string(&id)).map_err(js_err)
    }
    pub fn prepare_save(&mut self, doc: &str) -> Result<(), JsError> {
        let id = parse_id(doc)?;
        let pkg = self.engine.save_package(id).map_err(js_err)?;
        self.pending_saves.insert(id, pkg);
        Ok(())
    }
    pub fn save_package_manifest(&self, doc: &str) -> Result<String, JsError> {
        let id = parse_id(doc)?;
        Ok(self.pending_saves.get(&id).ok_or_else(|| JsError::new("prepare_save first"))?.manifest_json.clone())
    }
    pub fn save_package_image_names(&self, doc: &str) -> Result<Vec<String>, JsError> {
        let id = parse_id(doc)?;
        Ok(self.pending_saves.get(&id).ok_or_else(|| JsError::new("prepare_save first"))?.images.iter().map(|(n, _)| n.clone()).collect())
    }
    pub fn save_package_image(&self, doc: &str, name: &str) -> Result<Uint8Array, JsError> {
        let id = parse_id(doc)?;
        let pkg = self.pending_saves.get(&id).ok_or_else(|| JsError::new("prepare_save first"))?;
        let (_, bytes) = pkg.images.iter().find(|(n, _)| n == name).ok_or_else(|| JsError::new("no such image"))?;
        Ok(Uint8Array::from(bytes.as_slice()))
    }
    pub fn finish_save(&mut self, doc: &str) -> Result<(), JsError> { self.pending_saves.remove(&parse_id(doc)?); Ok(()) }
    pub fn mark_saved(&mut self, doc: &str, path: Option<String>) -> Result<(), JsError> { self.engine.mark_saved(parse_id(doc)?, path); Ok(()) }
    pub fn close_document(&mut self, doc: &str) -> Result<(), JsError> { let id = parse_id(doc)?; self.engine.close_document(id); self.pending_saves.remove(&id); Ok(()) }
    pub fn document_ids(&self) -> Vec<String> { self.engine.document_ids().iter().map(ids::upper_string).collect() }

    pub fn state(&self, doc: &str) -> Result<String, JsError> {
        let s = self.engine.state(parse_id(doc)?).map_err(js_err)?;
        serde_json::to_string(&s).map_err(js_err)
    }
    pub fn execute(&mut self, doc: &str, command_json: &str) -> Result<String, JsError> {
        let command: Command = serde_json::from_str(command_json).map_err(js_err)?;
        let dirty = self.engine.execute(parse_id(doc)?, command).map_err(js_err)?;
        serde_json::to_string(&dirty).map_err(js_err)
    }
    pub fn undo(&mut self, doc: &str) -> Result<String, JsError> {
        serde_json::to_string(&self.engine.undo(parse_id(doc)?).map_err(js_err)?).map_err(js_err)
    }
    pub fn redo(&mut self, doc: &str) -> Result<String, JsError> {
        serde_json::to_string(&self.engine.redo(parse_id(doc)?).map_err(js_err)?).map_err(js_err)
    }
    pub fn import_image(&mut self, doc: Option<String>, bytes: &[u8], name: &str, x: Option<f64>, y: Option<f64>) -> Result<String, JsError> {
        let id = match doc { Some(d) => Some(parse_id(&d)?), None => None };
        let at = match (x, y) { (Some(x), Some(y)) => Some(Point { x, y }), _ => None };
        self.engine.import_image(id, bytes, name, at).map(|id| ids::upper_string(&id)).map_err(js_err)
    }
    pub fn export_png(&self, doc: &str) -> Result<Uint8Array, JsError> {
        Ok(Uint8Array::from(self.engine.export_png(parse_id(doc)?).map_err(js_err)?.as_slice()))
    }
    pub fn export_jpeg(&self, doc: &str, quality: f64, r: f64, g: f64, b: f64) -> Result<Uint8Array, JsError> {
        Ok(Uint8Array::from(self.engine.export_jpeg(parse_id(doc)?, quality, [r, g, b]).map_err(js_err)?.as_slice()))
    }
    pub fn composite(&self, doc: &str, x: f64, y: f64, w: f64, h: f64, out_w: u32, out_h: u32) -> Result<Uint8Array, JsError> {
        let raster = self.engine.composite(parse_id(doc)?, Rect { x, y, width: w, height: h }, out_w, out_h).map_err(js_err)?;
        Ok(Uint8Array::from(raster.bytes()))
    }
    pub fn layer_pixels_ptr(&self, doc: &str, layer: &str) -> Result<*const u8, JsError> {
        let d = self.engine.document(parse_id(doc)?).ok_or_else(|| JsError::new("no document"))?;
        let l = d.layer(parse_id(layer)?).ok_or_else(|| JsError::new("no layer"))?;
        Ok(l.pixels.as_ref().map_or(std::ptr::null(), |p| p.bytes().as_ptr()))
    }
    pub fn layer_pixels_len(&self, doc: &str, layer: &str) -> Result<usize, JsError> {
        let d = self.engine.document(parse_id(doc)?).ok_or_else(|| JsError::new("no document"))?;
        let l = d.layer(parse_id(layer)?).ok_or_else(|| JsError::new("no layer"))?;
        Ok(l.pixels.as_ref().map_or(0, |p| p.bytes().len()))
    }
}
```

Add `"engine-wasm"` to the workspace members. Run `cargo check -p compositor-engine-wasm --target wasm32-unknown-unknown` and fix any signature the `wasm-bindgen` version rejects (for example `Vec<String>` arguments need `js-sys` 0.3.64 or newer).

- [ ] **Step 3: Node project**

`package.json`:
```json
{
  "name": "compositor-windows",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "wasm": "wasm-pack build engine-wasm --target web --release --out-dir ../app/src/engine/pkg --out-name compositor_engine",
    "wasm:dev": "wasm-pack build engine-wasm --target web --dev --out-dir ../app/src/engine/pkg --out-name compositor_engine",
    "build-info": "powershell -ExecutionPolicy Bypass -File scripts/write-build-info.ps1",
    "dev": "vite",
    "build": "tsc --noEmit && vite build",
    "test": "vitest run",
    "e2e": "playwright test",
    "tauri": "tauri"
  },
  "dependencies": {
    "@tauri-apps/api": "^2.3.0",
    "@tauri-apps/plugin-dialog": "^2.2.1",
    "react": "^18.3.1",
    "react-dom": "^18.3.1",
    "zustand": "^5.0.2"
  },
  "devDependencies": {
    "@playwright/test": "^1.49.0",
    "@tauri-apps/cli": "^2.3.1",
    "@types/react": "^18.3.18",
    "@types/react-dom": "^18.3.5",
    "@vitejs/plugin-react": "^4.3.4",
    "typescript": "^5.7.2",
    "vite": "^6.0.7",
    "vitest": "^3.0.4",
    "jsdom": "^25.0.1"
  }
}
```

`vite.config.ts`:
```ts
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  root: "app",
  plugins: [react()],
  clearScreen: false,
  server: { port: 1420, strictPort: true, fs: { allow: [".."] } },
  build: { outDir: "dist", emptyOutDir: true, target: "es2022" },
  define: { __APP_VERSION__: JSON.stringify(process.env.npm_package_version ?? "0.0.0") },
});
```

`vitest.config.ts`:
```ts
import { defineConfig } from "vitest/config";
export default defineConfig({ test: { include: ["app/tests/unit/**/*.test.ts"], environment: "node" } });
```

`tsconfig.json`:
```json
{
  "compilerOptions": {
    "target": "ES2022", "lib": ["ES2022", "DOM", "DOM.Iterable"], "module": "ESNext", "moduleResolution": "bundler",
    "jsx": "react-jsx", "strict": true, "noUnusedLocals": true, "skipLibCheck": true, "isolatedModules": true,
    "types": ["vite/client"]
  },
  "include": ["app/src", "app/tests"]
}
```

`app/index.html`:
```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Compositor</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

`app/src/build-info.ts` (committed default; the build script rewrites it):
```ts
export const BUILD_MARKER = "COMPOSITOR_BUILD_dev";
```

`scripts/write-build-info.ps1`:
```powershell
param([string]$Version = "")
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
if (-not $Version) { $Version = (Get-Content (Join-Path $root 'package.json') -Raw | ConvertFrom-Json).version }
$stamp = Get-Date -Format 'yyyyMMdd-HHmm'
$marker = "COMPOSITOR_BUILD_${Version}_$stamp"
Set-Content -Path (Join-Path $root 'app\src\build-info.ts') -Value "export const BUILD_MARKER = `"$marker`";" -Encoding ascii
Write-Output $marker
```

`app/src/engine/types.ts`:
```ts
export type Sampling = "Nearest" | "Smooth" | "High quality";
export type BlendMode = "Normal" | "Multiply" | "Screen" | "Overlay" | "Darken" | "Lighten" | "Difference"
  | "Color Dodge" | "Color Burn" | "Hue" | "Saturation" | "Color" | "Luminosity";

export interface LayerTransform { origin: [number, number]; size: [number, number]; rotation: number; flipX: boolean; flipY: boolean; sampling: Sampling; }

export interface LayerState {
  id: string; name: string; visible: boolean; isGroup: boolean; parentId: string | null; opacity: number;
  blendMode: BlendMode; transform: LayerTransform; pixelsWidth: number; pixelsHeight: number; pixelsRevision: number; hasMask: boolean;
}

export interface DocumentState {
  id: string; width: number; height: number; resolution: number; activeLayerId: string | null;
  canUndo: boolean; canRedo: boolean; isModified: boolean; path: string | null; layers: LayerState[];
}

export type Command =
  | { type: "AddBlankLayer" }
  | { type: "RenameLayer"; id: string; name: string }
  | { type: "SetLayerVisible"; id: string; visible: boolean }
  | { type: "DeleteLayer"; id: string }
  | { type: "SetActiveLayer"; id: string | null }
  | { type: "CanvasSize"; width: number; height: number; anchor: number; fill: [number, number, number] | null }
  | { type: "Crop"; x: number; y: number; width: number; height: number }
  | { type: "ImageSize"; width: number; height: number; resolution: number; sampling: Sampling }
  | { type: "FlipCanvas"; horizontal: boolean };

export interface Dirty { structure: boolean; canvas: boolean; layers: string[]; }

export interface PackageFiles { manifest: string; images: { name: string; bytes: Uint8Array }[]; }
```

`app/src/engine/client.ts`:
```ts
import init, { WasmEngine } from "./pkg/compositor_engine.js";
import type { Command, Dirty, DocumentState, PackageFiles } from "./types";

export class EngineClient {
  private constructor(private readonly wasm: WasmEngine, private readonly memory: WebAssembly.Memory) {}

  static async load(): Promise<EngineClient> {
    const exports = await init();
    return new EngineClient(new WasmEngine(), exports.memory);
  }

  version(): string { return this.wasm.version(); }
  newDocument(width: number, height: number, emptyLayer: boolean): string { return this.wasm.new_document(width, height, emptyLayer); }
  openPackage(files: PackageFiles, path: string | null): string {
    return this.wasm.open_package(files.manifest, files.images.map((i) => i.name), files.images.map((i) => i.bytes), path ?? undefined);
  }
  savePackage(doc: string): PackageFiles {
    this.wasm.prepare_save(doc);
    try {
      const manifest = this.wasm.save_package_manifest(doc);
      const images = this.wasm.save_package_image_names(doc).map((name) => ({ name, bytes: this.wasm.save_package_image(doc, name) }));
      return { manifest, images };
    } finally { this.wasm.finish_save(doc); }
  }
  markSaved(doc: string, path: string | null): void { this.wasm.mark_saved(doc, path ?? undefined); }
  closeDocument(doc: string): void { this.wasm.close_document(doc); }
  documentIds(): string[] { return this.wasm.document_ids(); }
  state(doc: string): DocumentState { return JSON.parse(this.wasm.state(doc)) as DocumentState; }
  execute(doc: string, command: Command): Dirty { return JSON.parse(this.wasm.execute(doc, JSON.stringify(command))) as Dirty; }
  undo(doc: string): Dirty { return JSON.parse(this.wasm.undo(doc)) as Dirty; }
  redo(doc: string): Dirty { return JSON.parse(this.wasm.redo(doc)) as Dirty; }
  importImage(doc: string | null, bytes: Uint8Array, name: string, at: { x: number; y: number } | null): string {
    return this.wasm.import_image(doc ?? undefined, bytes, name, at?.x, at?.y);
  }
  exportPng(doc: string): Uint8Array { return this.wasm.export_png(doc); }
  exportJpeg(doc: string, quality: number, matte: [number, number, number]): Uint8Array { return this.wasm.export_jpeg(doc, quality, ...matte); }
  composite(doc: string, region: { x: number; y: number; width: number; height: number }, outWidth: number, outHeight: number): Uint8Array {
    return this.wasm.composite(doc, region.x, region.y, region.width, region.height, outWidth, outHeight);
  }
  /** A view on wasm memory; valid only until the next engine call. */
  layerPixels(doc: string, layer: string): Uint8Array | null {
    const len = this.wasm.layer_pixels_len(doc, layer);
    if (len === 0) return null;
    const ptr = this.wasm.layer_pixels_ptr(doc, layer);
    return new Uint8Array(this.memory.buffer, ptr, len);
  }
}
```

`app/src/main.tsx`:
```tsx
import React from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";

createRoot(document.getElementById("root")!).render(<React.StrictMode><App /></React.StrictMode>);
```

`app/src/App.tsx` (skeleton; later tasks replace the body):
```tsx
import { useEffect, useState } from "react";
import { EngineClient } from "./engine/client";
import { BUILD_MARKER } from "./build-info";
import { installTestApi } from "./test-api";

export function App() {
  const [engine, setEngine] = useState<EngineClient | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    EngineClient.load().then((e) => { setEngine(e); installTestApi({ engine: e }); }).catch((e) => setError(String(e)));
  }, []);
  if (error) return <div data-testid="engine-error">Engine failed to load: {error}</div>;
  if (!engine) return <div data-testid="engine-loading">Loading engine...</div>;
  return <div data-testid="engine-ready">Compositor engine {engine.version()} ({BUILD_MARKER})</div>;
}
```

`app/src/test-api.ts`:
```ts
export function installTestApi(api: Record<string, unknown>): void {
  if (import.meta.env.DEV || import.meta.env.VITE_TEST_API === "1") {
    (window as unknown as { __compositor: Record<string, unknown> }).__compositor = { ...((window as unknown as { __compositor?: Record<string, unknown> }).__compositor ?? {}), ...api };
  }
}
```

`playwright.config.ts`:
```ts
import { defineConfig } from "@playwright/test";
export default defineConfig({
  testDir: "app/tests/e2e",
  timeout: 30_000,
  use: { baseURL: "http://localhost:1420", headless: true },
  webServer: { command: "pnpm dev", url: "http://localhost:1420", reuseExistingServer: true, env: { VITE_BRIDGE: "mock", VITE_TEST_API: "1" } },
});
```

`app/tests/e2e/smoke.spec.ts`:
```ts
import { test, expect } from "@playwright/test";

test("engine loads in the browser", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toContainText("Compositor engine 0.1.0");
  const ids = await page.evaluate(() => {
    const api = (window as unknown as { __compositor: { engine: { newDocument(w: number, h: number, e: boolean): string; documentIds(): string[] } } }).__compositor;
    api.engine.newDocument(10, 10, true);
    return api.engine.documentIds();
  });
  expect(ids).toHaveLength(1);
});
```

- [ ] **Step 4: Build and run**

```
pnpm install
pnpm wasm:dev
pnpm build
pnpm exec playwright install chromium
pnpm e2e
```
Expected: `pnpm build` succeeds; the smoke test passes. If Playwright's Chromium download is blocked by the proxy, set `PLAYWRIGHT_DOWNLOAD_HOST` to a reachable mirror or use `channel: "msedge"` in `playwright.config.ts` (system Edge is present on this machine, see LL-011).

- [ ] **Step 5: Commit**

```
git add Cargo.toml engine-wasm package.json pnpm-lock.yaml vite.config.ts vitest.config.ts tsconfig.json playwright.config.ts app scripts
git commit -m "feat(app): wasm engine wrapper, Vite/React skeleton and typed engine client"
```

---

### Task 11: Viewport, crop geometry and canvas size draft (TypeScript ports)

**Files:**
- Create: `app/src/canvas/viewport.ts`, `app/src/tools/crop-geometry.ts`, `app/src/tools/canvas-size-draft.ts`
- Test: `app/tests/unit/viewport.test.ts`, `app/tests/unit/crop-geometry.test.ts`, `app/tests/unit/canvas-size-draft.test.ts`

**Interfaces:**
- `Viewport` class: fields `viewSize {width,height}`, `backingScale`, `zoom` (read-only), `pan {width,height}`, `followsFit`; `pointsPerPixel`, `center`, `documentRect(size)`, `documentPoint(viewPoint, size)`, `viewPoint(docPoint, size)`, `fit(size)`, `resize(size, backingScale, docSize?)`, `setZoom(value, anchor, size)`, `translate(delta)`; `ZOOM_RANGE = [0.001, 32]`. Document coordinates are pixels, top-left origin; view coordinates are CSS pixels; zoom 1 means one document pixel per device pixel.
- `crop-geometry.ts`: `type Rect = {x,y,width,height}`; `snapped(rect)`, `isValid(rect)`, `create(start, end, ratio, symmetric)`, `HANDLES` (8 unit points in the macOS order), `cropDrag(mode, start, original).updated(point, ratio, symmetric)` where `mode` is `{kind:"create"} | {kind:"move"} | {kind:"resize", index}`; `resizeTransform(original: Rect, index, start, point, lockRatio, symmetric): Rect` (port of `TransformDrag.updated` for the resize case only); `CropSnap` class with `apply(rect, drag, point, ratio, symmetric)`.
- `canvas-size-draft.ts`: `CanvasSizeDraft` with `originalWidth`, `originalHeight`, `resolution`, `width`, `height`, `relative`, `locked`, `unit: "Pixels" | "Percent" | "Inches" | "Centimeters"`, `valid`, `displayed(widthAxis)`, `set(value, widthAxis)`.

- [ ] **Step 1: Write the failing tests**

`app/tests/unit/viewport.test.ts`:
```ts
import { describe, expect, it } from "vitest";
import { Viewport } from "../../src/canvas/viewport";

describe("Viewport", () => {
  it("maps document and view points both ways at any zoom", () => {
    const size = { width: 1000, height: 500 };
    for (const scale of [1, 2]) {
      const v = new Viewport();
      v.resize({ width: 800, height: 600 }, scale, size);
      v.translate({ width: 37, height: -19 });
      const point = { x: -20, y: 135 };
      const view = v.viewPoint(point, size);
      const back = v.documentPoint(view, size);
      expect(Math.abs(back.x - point.x)).toBeLessThan(0.001);
      expect(Math.abs(back.y - point.y)).toBeLessThan(0.001);
    }
  });

  it("fit leaves a 96 point margin and clamps zoom", () => {
    const v = new Viewport();
    v.resize({ width: 800, height: 600 }, 2, { width: 1000, height: 500 });
    // zoom = min((800-96)/1000, (600-96)/500) * 2 = min(0.704, 1.008) * 2 = 1.408
    expect(v.zoom).toBeCloseTo(1.408, 6);
    expect(v.followsFit).toBe(true);
    v.setZoom(1000, { x: 0, y: 0 }, { width: 1000, height: 500 });
    expect(v.zoom).toBe(32);
    expect(v.followsFit).toBe(false);
  });

  it("zoom anchors the pointer", () => {
    const size = { width: 1000, height: 800 };
    const v = new Viewport();
    v.resize({ width: 700, height: 500 }, 2, size);
    const anchor = { x: 100, y: 120 };
    const before = v.documentPoint(anchor, size);
    v.setZoom(2.5, anchor, size);
    const after = v.documentPoint(anchor, size);
    expect(Math.abs(after.x - before.x)).toBeLessThan(1e-6);
    expect(Math.abs(after.y - before.y)).toBeLessThan(1e-6);
  });

  it("resize keeps the pan proportional when not following fit", () => {
    const v = new Viewport();
    v.resize({ width: 800, height: 600 }, 1, { width: 100, height: 100 });
    v.translate({ width: 10, height: 20 });
    v.resize({ width: 800, height: 600 }, 2, { width: 100, height: 100 });
    expect(v.pan.width).toBeCloseTo(5, 6);
    expect(v.pan.height).toBeCloseTo(10, 6);
  });
});
```

`app/tests/unit/crop-geometry.test.ts`:
```ts
import { describe, expect, it } from "vitest";
import { create, cropDrag, CropSnap, HANDLES, isValid } from "../../src/tools/crop-geometry";

describe("crop geometry", () => {
  it("supports reverse drags, ratios, moves and every handle", () => {
    const rect = create({ x: 100, y: 100 }, { x: 20, y: 60 }, 2, false);
    expect(rect).toEqual({ x: 20, y: 60, width: 80, height: 40 });
    const move = cropDrag({ kind: "move" }, { x: 40, y: 70 }, rect);
    expect(move.updated({ x: 30, y: 40 }, null, false)).toEqual({ x: 10, y: 30, width: 80, height: 40 });
    for (let index = 0; index < 8; index++) {
      const unit = HANDLES[index];
      const start = { x: rect.x + unit.x * rect.width, y: rect.y + unit.y * rect.height };
      const drag = cropDrag({ kind: "resize", index }, start, rect);
      const next = drag.updated({ x: start.x + (unit.x * 2 - 1) * 20, y: start.y + (unit.y * 2 - 1) * 10 }, 2, false);
      expect(isValid(next)).toBe(true);
      expect(Math.abs(next.width / next.height - 2)).toBeLessThan(0.05);
      expect(next).not.toEqual(rect);
    }
  });

  it("snaps edges to nearby targets", () => {
    const snap = new CropSnap([0, 200, 50, 150], [0, 100, 20, 80], 6);
    const rect = { x: 10, y: 10, width: 60, height: 40 };
    const move = cropDrag({ kind: "move" }, { x: 30, y: 30 }, rect);
    const movedTo = { x: 26, y: 34 };
    expect(snap.apply(move.updated(movedTo, null, false), move, movedTo, null, false)).toEqual({ x: 0, y: 20, width: 60, height: 40 });
    const cornerIndex = HANDLES.findIndex((h) => h.x === 1 && h.y === 1);
    const resize = cropDrag({ kind: "resize", index: cornerIndex }, { x: 70, y: 50 }, rect);
    const near = { x: 146, y: 83 };
    expect(snap.apply(resize.updated(near, null, false), resize, near, null, false)).toEqual({ x: 10, y: 10, width: 140, height: 70 });
    const far = { x: 120, y: 60 };
    expect(snap.apply(resize.updated(far, null, false), resize, far, null, false)).toEqual({ x: 10, y: 10, width: 110, height: 50 });
    const ratioRect = { x: 10, y: 10, width: 138, height: 69 };
    expect(snap.apply(ratioRect, resize, { x: 148, y: 79 }, 2, false)).toEqual(ratioRect);
    const createDrag = cropDrag({ kind: "create" }, { x: 52, y: 18 }, { x: 0, y: 0, width: 0, height: 0 });
    const dragged = { x: 147, y: 77 };
    expect(snap.apply(createDrag.updated(dragged, null, false), createDrag, dragged, null, false)).toEqual({ x: 52, y: 18, width: 98, height: 62 });
  });

  it("symmetric creation grows from the start point", () => {
    expect(create({ x: 50, y: 50 }, { x: 60, y: 70 }, null, true)).toEqual({ x: 40, y: 30, width: 20, height: 40 });
  });
});
```

`app/tests/unit/canvas-size-draft.test.ts`:
```ts
import { describe, expect, it } from "vitest";
import { CanvasSizeDraft } from "../../src/tools/canvas-size-draft";

describe("CanvasSizeDraft", () => {
  it("relative, ratio and units use final dimensions", () => {
    const d = new CanvasSizeDraft(1000, 500, 100);
    d.relative = true; d.locked = true;
    d.set(200, true);
    expect([d.width, d.height]).toEqual([1200, 600]);
    expect(d.displayed(false)).toBe(100);
    d.set(-250, false);
    expect([d.width, d.height]).toEqual([500, 250]);
    d.relative = false; d.unit = "Inches";
    d.set(10, true);
    expect([d.width, d.height]).toEqual([1000, 500]);
    d.unit = "Percent";
    d.set(50, true);
    expect([d.width, d.height]).toEqual([500, 250]);
    d.unit = "Centimeters";
    expect(Math.abs(d.displayed(true) - 12.7)).toBeLessThan(0.001);
    d.unit = "Pixels";
    d.set(0, true);
    expect(d.valid).toBe(false);
  });
});
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `pnpm test`
Expected: module not found errors for the three source files.

- [ ] **Step 3: Implement the viewport**

`app/src/canvas/viewport.ts`:
```ts
export interface SizeLike { width: number; height: number; }
export interface PointLike { x: number; y: number; }
export interface RectLike { x: number; y: number; width: number; height: number; }

export const ZOOM_RANGE: [number, number] = [0.001, 32];
const FIT_MARGIN = 96;

/** Document: pixels, top-left origin. View: CSS pixels. Zoom 1 draws one document pixel per device pixel. */
export class Viewport {
  viewSize: SizeLike = { width: 0, height: 0 };
  backingScale = 1;
  private _zoom = 1;
  pan: SizeLike = { width: 0, height: 0 };
  private _followsFit = true;

  get zoom(): number { return this._zoom; }
  get followsFit(): boolean { return this._followsFit; }
  get pointsPerPixel(): number { return this._zoom / this.backingScale; }
  get center(): PointLike { return { x: this.viewSize.width / 2, y: this.viewSize.height / 2 }; }

  documentRect(size: SizeLike): RectLike {
    const w = size.width * this.pointsPerPixel, h = size.height * this.pointsPerPixel;
    const c = this.center;
    return { x: c.x - w / 2 + this.pan.width, y: c.y - h / 2 + this.pan.height, width: w, height: h };
  }
  documentPoint(point: PointLike, size: SizeLike): PointLike {
    const o = this.documentRect(size);
    return { x: (point.x - o.x) / this.pointsPerPixel, y: (point.y - o.y) / this.pointsPerPixel };
  }
  viewPoint(point: PointLike, size: SizeLike): PointLike {
    const o = this.documentRect(size);
    return { x: o.x + point.x * this.pointsPerPixel, y: o.y + point.y * this.pointsPerPixel };
  }
  fit(size: SizeLike): void {
    if (this.viewSize.width <= 0 || this.viewSize.height <= 0) { this._followsFit = true; return; }
    const zoom = Math.min(Math.max(1, this.viewSize.width - FIT_MARGIN) / size.width,
      Math.max(1, this.viewSize.height - FIT_MARGIN) / size.height) * this.backingScale;
    this._zoom = clamp(zoom);
    this.pan = { width: 0, height: 0 };
    this._followsFit = true;
  }
  resize(size: SizeLike, backingScale: number, docSize?: SizeLike): void {
    const oldScale = this.pointsPerPixel;
    this.viewSize = { ...size };
    this.backingScale = Math.max(1, backingScale);
    if (this._followsFit && docSize) { this.fit(docSize); }
    else {
      const ratio = this.pointsPerPixel / oldScale;
      this.pan = { width: this.pan.width * ratio, height: this.pan.height * ratio };
    }
  }
  setZoom(value: number, anchor: PointLike, size: SizeLike): void {
    if (!Number.isFinite(value)) return;
    const pixel = this.documentPoint(anchor, size);
    this._zoom = clamp(value);
    const moved = this.viewPoint(pixel, size);
    this.pan = { width: this.pan.width + anchor.x - moved.x, height: this.pan.height + anchor.y - moved.y };
    this._followsFit = false;
  }
  translate(delta: SizeLike): void {
    this.pan = { width: this.pan.width + delta.width, height: this.pan.height + delta.height };
    this._followsFit = false;
  }
}

function clamp(value: number): number { return Math.min(ZOOM_RANGE[1], Math.max(ZOOM_RANGE[0], value)); }
```

- [ ] **Step 4: Implement crop geometry**

`app/src/tools/crop-geometry.ts`:
```ts
import type { PointLike, RectLike } from "../canvas/viewport";
export type Rect = RectLike;
export type Point = PointLike;

export const HANDLES: Point[] = [
  { x: 0, y: 0 }, { x: 0.5, y: 0 }, { x: 1, y: 0 }, { x: 1, y: 0.5 }, { x: 1, y: 1 }, { x: 0.5, y: 1 }, { x: 0, y: 1 }, { x: 0, y: 0.5 },
];

function standardized(r: Rect): Rect {
  const x = Math.min(r.x, r.x + r.width), y = Math.min(r.y, r.y + r.height);
  return { x, y, width: Math.abs(r.width), height: Math.abs(r.height) };
}
/** Whole document pixels, at least 1 wide and high (matches CropGeometry.snapped). */
export function snapped(rect: Rect): Rect {
  const r = standardized(rect);
  const x = Math.round(r.x), y = Math.round(r.y);
  return { x, y, width: Math.max(1, Math.round(r.x + r.width) - x), height: Math.max(1, Math.round(r.y + r.height) - y) };
}
export function isValid(r: Rect): boolean {
  return [r.x, r.y, r.width, r.height].every(Number.isFinite) && r.width >= 1 && r.width <= 30_000 && r.height >= 1 && r.height <= 30_000
    && Math.abs(r.x) <= 1_000_000 && Math.abs(r.y) <= 1_000_000;
}
/** A frame dragged from start to end; with a ratio the longer axis wins; symmetric grows out from start. */
export function create(start: Point, end: Point, ratio: number | null, symmetric: boolean): Rect {
  let dx = end.x - start.x, dy = end.y - start.y;
  if (ratio !== null) {
    if (Math.abs(dx) > Math.abs(dy) * ratio) dy = (dy < 0 ? -1 : 1) * Math.abs(dx) / ratio;
    else dx = (dx < 0 ? -1 : 1) * Math.abs(dy) * ratio;
  }
  if (symmetric) return snapped({ x: start.x - Math.abs(dx), y: start.y - Math.abs(dy), width: Math.abs(dx) * 2, height: Math.abs(dy) * 2 });
  return snapped({ x: Math.min(start.x, start.x + dx), y: Math.min(start.y, start.y + dy), width: Math.abs(dx), height: Math.abs(dy) });
}

/** Port of TransformDrag.updated for the resize case, rotation 0. */
export function resizeRect(original: Rect, index: number, start: Point, point: Point, lockRatio: boolean, symmetric: boolean): Rect {
  const handle = HANDLES[index];
  const anchorUnit = symmetric ? { x: 0.5, y: 0.5 } : { x: 1 - handle.x, y: 1 - handle.y };
  const unitPoint = (u: Point) => ({ x: original.x + u.x * original.width, y: original.y + u.y * original.height });
  const anchor = unitPoint(anchorUnit);
  const initial = unitPoint(handle);
  const dx = initial.x + point.x - start.x - anchor.x;
  const dy = initial.y + point.y - start.y - anchor.y;
  const span = symmetric ? 2 : 1;
  const localX = dx * span, localY = dy * span;
  const sx = handle.x * 2 - 1, sy = handle.y * 2 - 1;
  let width = sx === 0 ? original.width : Math.max(1, localX * sx);
  let height = sy === 0 ? original.height : Math.max(1, localY * sy);
  if (lockRatio) {
    let factor: number;
    if (sx === 0) factor = height / original.height;
    else if (sy === 0) factor = width / original.width;
    else factor = Math.max(1 / Math.min(original.width, original.height),
      (localX * sx * original.width + localY * sy * original.height) / (original.width * original.width + original.height * original.height));
    width = original.width * factor; height = original.height * factor;
  }
  const cx = anchor.x + (0.5 - anchorUnit.x) * width, cy = anchor.y + (0.5 - anchorUnit.y) * height;
  const result = { x: cx - width / 2, y: cy - height / 2, width, height };
  return isValid(result) ? result : original;
}

export type CropDragMode = { kind: "create" } | { kind: "move" } | { kind: "resize"; index: number };
export interface CropDrag {
  mode: CropDragMode; start: Point; original: Rect;
  updated(point: Point, ratio: number | null, symmetric: boolean): Rect;
}
export function cropDrag(mode: CropDragMode, start: Point, original: Rect): CropDrag {
  return {
    mode, start, original,
    updated(point, ratio, symmetric) {
      switch (mode.kind) {
        case "create": return create(start, point, ratio, symmetric);
        case "move": return snapped({ ...original, x: original.x + point.x - start.x, y: original.y + point.y - start.y });
        case "resize": return snapped(resizeRect(original, mode.index, start, point, ratio !== null, symmetric));
      }
    },
  };
}

/** Crop edges snap to nearby layer and canvas edges while dragging. */
export class CropSnap {
  constructor(readonly xs: number[], readonly ys: number[], readonly tolerance: number) {}
  private nearest(value: number, targets: number[]): number | null {
    let best: number | null = null;
    for (const t of targets) {
      if (Math.abs(t - value) > this.tolerance) continue;
      if (best !== null && Math.abs(best - value) <= Math.abs(t - value)) continue;
      best = t;
    }
    return best;
  }
  apply(rect: Rect, drag: CropDrag, point: Point, ratio: number | null, symmetric: boolean): Rect {
    if (this.tolerance <= 0) return rect;
    let horizontal: boolean, vertical: boolean;
    switch (drag.mode.kind) {
      case "move": {
        const shift = (edges: number[], targets: number[]) => {
          const moves = edges.map((e) => { const n = this.nearest(e, targets); return n === null ? null : n - e; }).filter((m): m is number => m !== null);
          return moves.length ? moves.reduce((a, b) => (Math.abs(a) < Math.abs(b) ? a : b)) : 0;
        };
        const dx = shift([rect.x, rect.x + rect.width], this.xs), dy = shift([rect.y, rect.y + rect.height], this.ys);
        return { ...rect, x: rect.x + dx, y: rect.y + dy };
      }
      case "create": if (ratio !== null) return rect; horizontal = true; vertical = true; break;
      case "resize": { if (ratio !== null) return rect; const h = HANDLES[drag.mode.index]; horizontal = h.x !== 0.5; vertical = h.y !== 0.5; }
    }
    let r = { ...rect };
    if (horizontal) {
      if (Math.abs(point.x - r.x) <= Math.abs(point.x - (r.x + r.width))) {
        const x = this.nearest(r.x, this.xs);
        if (x !== null && x < r.x + r.width) r = { x, y: r.y, width: r.x + r.width - x, height: r.height };
      } else {
        const x = this.nearest(r.x + r.width, this.xs);
        if (x !== null && x > r.x) r.width = x - r.x;
      }
    }
    if (vertical) {
      if (Math.abs(point.y - r.y) <= Math.abs(point.y - (r.y + r.height))) {
        const y = this.nearest(r.y, this.ys);
        if (y !== null && y < r.y + r.height) r = { x: r.x, y, width: r.width, height: r.y + r.height - y };
      } else {
        const y = this.nearest(r.y + r.height, this.ys);
        if (y !== null && y > r.y) r.height = y - r.y;
      }
    }
    if (symmetric) {
      let center = { x: drag.original.x + drag.original.width / 2, y: drag.original.y + drag.original.height / 2 };
      if (drag.mode.kind === "create") center = drag.start;
      if (horizontal) {
        const half = point.x >= center.x ? r.x + r.width - center.x : center.x - r.x;
        if (half >= 0.5) { r.x = center.x - half; r.width = half * 2; }
      }
      if (vertical) {
        const half = point.y >= center.y ? r.y + r.height - center.y : center.y - r.y;
        if (half >= 0.5) { r.y = center.y - half; r.height = half * 2; }
      }
    }
    return r;
  }
}
```

- [ ] **Step 5: Implement the canvas size draft**

`app/src/tools/canvas-size-draft.ts`:
```ts
export type CanvasUnit = "Pixels" | "Percent" | "Inches" | "Centimeters";
export const CANVAS_UNITS: CanvasUnit[] = ["Pixels", "Percent", "Inches", "Centimeters"];

export class CanvasSizeDraft {
  width: number; height: number;
  relative = false; locked = false; unit: CanvasUnit = "Pixels";
  constructor(readonly originalWidth: number, readonly originalHeight: number, readonly resolution: number) {
    this.width = originalWidth; this.height = originalHeight;
  }
  get valid(): boolean {
    const w = Math.round(this.width), h = Math.round(this.height);
    return Number.isFinite(this.width) && Number.isFinite(this.height) && w >= 1 && w <= 30_000 && h >= 1 && h <= 30_000;
  }
  displayed(widthAxis: boolean): number {
    const original = widthAxis ? this.originalWidth : this.originalHeight;
    const pixels = (widthAxis ? this.width : this.height) - (this.relative ? original : 0);
    switch (this.unit) {
      case "Pixels": return pixels;
      case "Percent": return pixels / original * 100;
      case "Inches": return pixels / this.resolution;
      case "Centimeters": return pixels / this.resolution * 2.54;
    }
  }
  set(value: number, widthAxis: boolean): void {
    const original = widthAxis ? this.originalWidth : this.originalHeight;
    let pixels: number;
    switch (this.unit) {
      case "Pixels": pixels = value; break;
      case "Percent": pixels = value / 100 * original; break;
      case "Inches": pixels = value * this.resolution; break;
      case "Centimeters": pixels = value / 2.54 * this.resolution; break;
    }
    const final = pixels + (this.relative ? original : 0);
    if (widthAxis) { this.width = final; if (this.locked) this.height = final * this.originalHeight / this.originalWidth; }
    else { this.height = final; if (this.locked) this.width = final * this.originalWidth / this.originalHeight; }
  }
}
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `pnpm test`
Expected: 8 tests pass across the three files.

- [ ] **Step 7: Commit**

```
git add app
git commit -m "feat(app): viewport, crop geometry and canvas size draft ported with tests"
```

---

### Task 12: WebGL2 renderer, CPU fallback and the canvas view

**Files:**
- Create: `app/src/canvas/layer-textures.ts`, `app/src/canvas/gl-renderer.ts`, `app/src/canvas/cpu-renderer.ts`, `app/src/canvas/renderer.ts`, `app/src/canvas/overlay.ts`, `app/src/canvas/CanvasView.tsx`, `app/src/state/store.ts`
- Modify: `app/src/App.tsx` (mount `CanvasView` with a test document), `app/src/test-api.ts`
- Test: `app/tests/e2e/render.spec.ts`

**Interfaces:**
- `Renderer` interface: `sync(engine, state: DocumentState)` (uploads new or changed layer pixels, keyed by `pixelsRevision`), `render(state, viewport, dpr, options: { checkerboard: boolean })`, `readPixels(): Uint8Array` (RGBA, premultiplied, bottom-up flipped to top-down), `dispose()`. `createRenderer(canvas: HTMLCanvasElement, engine): Renderer` picks WebGL2 or the CPU fallback and exposes `kind: "gl" | "cpu"`.
- `LayerTextures`: per layer, a grid of chunk textures at most `CHUNK = 2048` px each, uploaded from the contiguous pixel buffer with `UNPACK_ROW_LENGTH`, `UNPACK_SKIP_PIXELS`, `UNPACK_SKIP_ROWS`; `LINEAR_MIPMAP_LINEAR` with `generateMipmap` for Smooth/High, `NEAREST` for Nearest.
- Store (`zustand`): `documents: Record<id, DocumentState>`, `order: string[]`, `activeId`, `viewports: Record<id, Viewport>`, `tool: "move" | "hand" | "zoom" | "crop"`, `cropRect: Rect | null`, `cropRatio: "None" | "Original" | "1:1" | "4:3" | "16:9"`, `sheet: null | { kind: "new" } | { kind: "canvasSize" } | { kind: "imageSize" } | { kind: "jpeg" }`, `error: string | null`, `rendererKind`; actions `refresh(id)`, `run(command)`, `undo()`, `redo()`, `setTool`, `setActive(id)`, `openSheet`, `closeSheet`, `setError`.
- Overlay (2D canvas above GL): pixel grid when `viewport.zoom >= 8`, crop frame (dimmed outside, 8 handles, rule-of-thirds lines), snap guide lines.
- Test API additions: `store`, `renderer`, `setZoom(z)`, `readPixels()`.

- [ ] **Step 1: Write the failing e2e test**

`app/tests/e2e/render.spec.ts`:
```ts
import { test, expect } from "@playwright/test";

/** A 4x4 document with a red 2x2 layer at (1,1) rendered at zoom 1 must match the engine's CPU composite. */
test("gl renderer matches the CPU compositor", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const result = await page.evaluate(async () => {
    const api = (window as unknown as { __compositor: any }).__compositor;
    const doc = api.engine.newDocument(4, 4, false);
    // Import a 2x2 red PNG via a data URL decoded by the browser.
    const png = Uint8Array.from(atob("iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAYAAABytg0kAAAAEUlEQVQIW2P8z8DwnwEIGGEMAD1sBAOiD07mAAAAAElFTkSuQmCC"), (c) => c.charCodeAt(0));
    api.engine.importImage(doc, png, "red", { x: 2, y: 2 });
    api.store.getState().openDocument(doc);
    await api.setZoom(1);
    api.setCheckerboard(false);
    await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
    const gl = Array.from(api.readDocumentPixels()) as number[];
    const cpu = Array.from(api.engine.composite(doc, { x: 0, y: 0, width: 4, height: 4 }, 4, 4)) as number[];
    return { gl, cpu, kind: api.store.getState().rendererKind };
  });
  expect(result.gl.length).toBe(64);
  for (let i = 0; i < 64; i++) expect(Math.abs(result.gl[i] - result.cpu[i]), `byte ${i} (${result.kind})`).toBeLessThanOrEqual(2);
});

test("cpu fallback renders when WebGL2 is unavailable", async ({ page }) => {
  await page.addInitScript(() => {
    const original = HTMLCanvasElement.prototype.getContext;
    HTMLCanvasElement.prototype.getContext = function (type: string, ...rest: unknown[]) {
      if (type === "webgl2") return null;
      return (original as any).call(this, type, ...rest);
    };
  });
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const kind = await page.evaluate(async () => {
    const api = (window as unknown as { __compositor: any }).__compositor;
    const doc = api.engine.newDocument(4, 4, true);
    api.store.getState().openDocument(doc);
    await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
    return api.store.getState().rendererKind;
  });
  expect(kind).toBe("cpu");
});
```

`readDocumentPixels()` reads the 4 by 4 document area from the rendered canvas (the viewport's `documentRect` scaled by the device pixel ratio), so the assertion is independent of where the document sits in the view.

- [ ] **Step 2: Run the test to verify it fails**

Run: `pnpm e2e --grep renderer`
Expected: fails, `api.store` undefined.

- [ ] **Step 3: Implement the store**

`app/src/state/store.ts`:
```ts
import { create } from "zustand";
import type { Command, DocumentState } from "../engine/types";
import type { EngineClient } from "../engine/client";
import { Viewport } from "../canvas/viewport";
import type { Rect } from "../tools/crop-geometry";

export type Tool = "move" | "hand" | "zoom" | "crop";
export type CropRatio = "None" | "Original" | "1:1" | "4:3" | "16:9";
export type Sheet = null | { kind: "new" } | { kind: "canvasSize" } | { kind: "imageSize" } | { kind: "jpeg" };

export interface EditorStore {
  engine: EngineClient | null;
  documents: Record<string, DocumentState>;
  order: string[];
  activeId: string | null;
  viewports: Record<string, Viewport>;
  tool: Tool;
  cropRect: Rect | null;
  cropRatio: CropRatio;
  sheet: Sheet;
  error: string | null;
  rendererKind: "gl" | "cpu" | null;
  renderTick: number;
  setEngine(engine: EngineClient): void;
  openDocument(id: string): void;
  closeDocument(id: string): void;
  setActive(id: string): void;
  refresh(id?: string): void;
  run(command: Command): void;
  undo(): void;
  redo(): void;
  setTool(tool: Tool): void;
  setCropRect(rect: Rect | null): void;
  setCropRatio(ratio: CropRatio): void;
  openSheet(sheet: Sheet): void;
  closeSheet(): void;
  setError(error: string | null): void;
  setRendererKind(kind: "gl" | "cpu"): void;
  invalidate(): void;
}

export const useEditor = create<EditorStore>((set, get) => ({
  engine: null, documents: {}, order: [], activeId: null, viewports: {}, tool: "move", cropRect: null, cropRatio: "None",
  sheet: null, error: null, rendererKind: null, renderTick: 0,
  setEngine: (engine) => set({ engine }),
  openDocument: (id) => {
    const engine = get().engine!;
    const state = engine.state(id);
    const viewport = new Viewport();
    set((s) => ({ documents: { ...s.documents, [id]: state }, order: s.order.includes(id) ? s.order : [...s.order, id],
      viewports: { ...s.viewports, [id]: viewport }, activeId: id, cropRect: null }));
  },
  closeDocument: (id) => {
    get().engine!.closeDocument(id);
    set((s) => {
      const { [id]: _d, ...documents } = s.documents;
      const { [id]: _v, ...viewports } = s.viewports;
      const order = s.order.filter((o) => o !== id);
      const activeId = s.activeId === id ? order[order.length - 1] ?? null : s.activeId;
      return { documents, viewports, order, activeId, cropRect: null };
    });
  },
  setActive: (id) => set({ activeId: id, cropRect: null }),
  refresh: (id) => {
    const target = id ?? get().activeId;
    if (!target) return;
    const state = get().engine!.state(target);
    set((s) => ({ documents: { ...s.documents, [target]: state }, renderTick: s.renderTick + 1 }));
  },
  run: (command) => {
    const { engine, activeId } = get();
    if (!engine || !activeId) return;
    try { engine.execute(activeId, command); get().refresh(activeId); }
    catch (e) { set({ error: String(e instanceof Error ? e.message : e) }); }
  },
  undo: () => { const { engine, activeId } = get(); if (engine && activeId) { engine.undo(activeId); get().refresh(activeId); } },
  redo: () => { const { engine, activeId } = get(); if (engine && activeId) { engine.redo(activeId); get().refresh(activeId); } },
  setTool: (tool) => set({ tool, cropRect: tool === "crop" ? get().cropRect : null }),
  setCropRect: (cropRect) => set({ cropRect }),
  setCropRatio: (cropRatio) => set({ cropRatio }),
  openSheet: (sheet) => set({ sheet }),
  closeSheet: () => set({ sheet: null }),
  setError: (error) => set({ error }),
  setRendererKind: (rendererKind) => set({ rendererKind }),
  invalidate: () => set((s) => ({ renderTick: s.renderTick + 1 })),
}));
```

- [ ] **Step 4: Implement layer textures and the GL renderer**

`app/src/canvas/layer-textures.ts`:
```ts
import type { LayerState } from "../engine/types";

export const CHUNK = 2048;

export interface Chunk { texture: WebGLTexture; x: number; y: number; width: number; height: number; }
export interface LayerTexture { revision: number; width: number; height: number; nearest: boolean; chunks: Chunk[]; }

export class LayerTextures {
  private layers = new Map<string, LayerTexture>();
  constructor(private readonly gl: WebGL2RenderingContext) {}

  get(id: string): LayerTexture | undefined { return this.layers.get(id); }

  /** Uploads when the revision or sampling changed. `pixels` is the layer's contiguous premultiplied RGBA buffer. */
  sync(layer: LayerState, pixels: Uint8Array | null): void {
    const nearest = layer.transform.sampling === "Nearest";
    const existing = this.layers.get(layer.id);
    if (!pixels || layer.pixelsWidth === 0) { if (existing) this.remove(layer.id); return; }
    if (existing && existing.revision === layer.pixelsRevision && existing.nearest === nearest) return;
    if (existing) this.remove(layer.id);
    const gl = this.gl;
    const chunks: Chunk[] = [];
    gl.pixelStorei(gl.UNPACK_ROW_LENGTH, layer.pixelsWidth);
    gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, false);
    for (let y = 0; y < layer.pixelsHeight; y += CHUNK) {
      for (let x = 0; x < layer.pixelsWidth; x += CHUNK) {
        const width = Math.min(CHUNK, layer.pixelsWidth - x), height = Math.min(CHUNK, layer.pixelsHeight - y);
        const texture = gl.createTexture()!;
        gl.bindTexture(gl.TEXTURE_2D, texture);
        gl.pixelStorei(gl.UNPACK_SKIP_PIXELS, x);
        gl.pixelStorei(gl.UNPACK_SKIP_ROWS, y);
        gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, width, height, 0, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, nearest ? gl.NEAREST : gl.LINEAR);
        if (nearest) { gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST); }
        else { gl.generateMipmap(gl.TEXTURE_2D); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR_MIPMAP_LINEAR); }
        chunks.push({ texture, x, y, width, height });
      }
    }
    gl.pixelStorei(gl.UNPACK_SKIP_PIXELS, 0);
    gl.pixelStorei(gl.UNPACK_SKIP_ROWS, 0);
    gl.pixelStorei(gl.UNPACK_ROW_LENGTH, 0);
    this.layers.set(layer.id, { revision: layer.pixelsRevision, width: layer.pixelsWidth, height: layer.pixelsHeight, nearest, chunks });
  }
  remove(id: string): void {
    const t = this.layers.get(id);
    if (!t) return;
    for (const c of t.chunks) this.gl.deleteTexture(c.texture);
    this.layers.delete(id);
  }
  retainOnly(ids: Set<string>): void { for (const id of [...this.layers.keys()]) if (!ids.has(id)) this.remove(id); }
  dispose(): void { this.retainOnly(new Set()); }
}
```

`app/src/canvas/renderer.ts`:
```ts
import type { DocumentState } from "../engine/types";
import type { EngineClient } from "../engine/client";
import type { Viewport } from "./viewport";
import { GlRenderer } from "./gl-renderer";
import { CpuRenderer } from "./cpu-renderer";

export interface RenderOptions { checkerboard: boolean; }
export interface Renderer {
  readonly kind: "gl" | "cpu";
  sync(engine: EngineClient, state: DocumentState): void;
  render(state: DocumentState, viewport: Viewport, dpr: number, options: RenderOptions): void;
  /** RGBA, top-down, the whole canvas element. */
  readPixels(): Uint8Array;
  dispose(): void;
}

export function createRenderer(canvas: HTMLCanvasElement, engine: EngineClient): Renderer {
  const gl = canvas.getContext("webgl2", { premultipliedAlpha: true, preserveDrawingBuffer: true, antialias: false });
  if (gl) return new GlRenderer(canvas, gl);
  return new CpuRenderer(canvas, engine);
}

/** Layers to draw bottom to top: visible with visible ancestors, groups excluded. */
export function renderOrder(state: DocumentState): DocumentState["layers"] {
  const byId = new Map(state.layers.map((l) => [l.id, l]));
  return state.layers.filter((layer) => {
    if (layer.isGroup) return false;
    let node: typeof layer | undefined = layer;
    let steps = 0;
    while (node) { if (!node.visible || steps++ > 64) return false; node = node.parentId ? byId.get(node.parentId) : undefined; }
    return true;
  });
}
```

`app/src/canvas/gl-renderer.ts`:
```ts
import type { DocumentState, LayerState } from "../engine/types";
import type { EngineClient } from "../engine/client";
import type { Viewport } from "./viewport";
import { LayerTextures } from "./layer-textures";
import { renderOrder, type RenderOptions, type Renderer } from "./renderer";

const VERT = `#version 300 es
in vec2 unit;
uniform mat3 unitToClip;
uniform vec4 uvRect; // x, y, w, h within the layer in 0..1
out vec2 uv;
void main() {
  vec3 clip = unitToClip * vec3(unit, 1.0);
  gl_Position = vec4(clip.xy, 0.0, 1.0);
  uv = unit;
}`;
const FRAG = `#version 300 es
precision highp float;
in vec2 uv;
uniform sampler2D tex;
uniform float opacity;
out vec4 color;
void main() { color = texture(tex, uv) * opacity; }`;
const CHECKER_FRAG = `#version 300 es
precision highp float;
in vec2 uv;
uniform vec2 sizePx; // document rect size in device pixels
uniform float cell;
out vec4 color;
void main() {
  vec2 p = floor(uv * sizePx / cell);
  float c = mod(p.x + p.y, 2.0) < 1.0 ? 0.80 : 0.95;
  color = vec4(c, c, c, 1.0);
}`;

function compile(gl: WebGL2RenderingContext, vert: string, frag: string): WebGLProgram {
  const make = (type: number, src: string) => { const s = gl.createShader(type)!; gl.shaderSource(s, src); gl.compileShader(s);
    if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(s) ?? "shader"); return s; };
  const p = gl.createProgram()!;
  gl.attachShader(p, make(gl.VERTEX_SHADER, vert)); gl.attachShader(p, make(gl.FRAGMENT_SHADER, frag)); gl.linkProgram(p);
  if (!gl.getProgramParameter(p, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(p) ?? "link");
  return p;
}

/** Column-major mat3 mapping unit square -> clip space for a rectangle placed by an affine map in view CSS pixels. */
function unitToClip(a: number, b: number, c: number, d: number, tx: number, ty: number, viewW: number, viewH: number): Float32Array {
  // view -> clip: x' = 2x/W - 1, y' = 1 - 2y/H
  const sx = 2 / viewW, sy = -2 / viewH;
  return new Float32Array([a * sx, b * sy, 0, c * sx, d * sy, 0, tx * sx - 1, ty * sy + 1, 1]);
}

export class GlRenderer implements Renderer {
  readonly kind = "gl" as const;
  private textures: LayerTextures;
  private layerProgram: WebGLProgram;
  private checkerProgram: WebGLProgram;
  private vao: WebGLVertexArrayObject;
  constructor(private readonly canvas: HTMLCanvasElement, private readonly gl: WebGL2RenderingContext) {
    this.textures = new LayerTextures(gl);
    this.layerProgram = compile(gl, VERT, FRAG);
    this.checkerProgram = compile(gl, VERT, CHECKER_FRAG);
    this.vao = gl.createVertexArray()!;
    gl.bindVertexArray(this.vao);
    const buf = gl.createBuffer()!;
    gl.bindBuffer(gl.ARRAY_BUFFER, buf);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([0, 0, 1, 0, 0, 1, 1, 1]), gl.STATIC_DRAW);
    for (const p of [this.layerProgram, this.checkerProgram]) {
      const loc = gl.getAttribLocation(p, "unit");
      gl.enableVertexAttribArray(loc);
      gl.vertexAttribPointer(loc, 2, gl.FLOAT, false, 0, 0);
    }
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
  }

  sync(engine: EngineClient, state: DocumentState): void {
    const keep = new Set<string>();
    for (const layer of state.layers) {
      keep.add(layer.id);
      const pixels = layer.pixelsWidth > 0 ? engine.layerPixels(state.id, layer.id) : null;
      this.textures.sync(layer, pixels);
    }
    this.textures.retainOnly(keep);
  }

  /** Affine placing layer pixel (px, py) in view CSS pixels: doc = T(px), view = rect.origin + doc * ppp. */
  private layerToView(layer: LayerState, viewport: Viewport, state: DocumentState) {
    const [ox, oy] = layer.transform.origin; const [w, h] = layer.transform.size;
    const cx = ox + w / 2, cy = oy + h / 2;
    const rad = (layer.transform.rotation % 360) * Math.PI / 180;
    const cos = Math.cos(rad), sin = Math.sin(rad);
    const fx = layer.transform.flipX ? -1 : 1, fy = layer.transform.flipY ? -1 : 1;
    // unit (0..1) -> doc: center + R * ((u - 0.5) * size * flip)
    const a = cos * w * fx, b = sin * w * fx, c = -sin * h * fy, d = cos * h * fy;
    const tx = cx - (a + c) / 2, ty = cy - (b + d) / 2;
    const ppp = viewport.pointsPerPixel; const rect = viewport.documentRect({ width: state.width, height: state.height });
    return { a: a * ppp, b: b * ppp, c: c * ppp, d: d * ppp, tx: rect.x + tx * ppp, ty: rect.y + ty * ppp };
  }

  render(state: DocumentState, viewport: Viewport, dpr: number, options: RenderOptions): void {
    const gl = this.gl;
    const W = Math.max(1, Math.round(viewport.viewSize.width * dpr)), H = Math.max(1, Math.round(viewport.viewSize.height * dpr));
    if (this.canvas.width !== W || this.canvas.height !== H) { this.canvas.width = W; this.canvas.height = H; }
    gl.viewport(0, 0, W, H);
    gl.clearColor(0.16, 0.16, 0.16, 1); gl.clear(gl.COLOR_BUFFER_BIT);
    gl.bindVertexArray(this.vao);
    const rect = viewport.documentRect({ width: state.width, height: state.height });
    const vw = viewport.viewSize.width, vh = viewport.viewSize.height;
    // Document background: checkerboard or transparent black.
    gl.useProgram(this.checkerProgram);
    gl.uniformMatrix3fv(gl.getUniformLocation(this.checkerProgram, "unitToClip"), false, unitToClip(rect.width, 0, 0, rect.height, rect.x, rect.y, vw, vh));
    gl.uniform2f(gl.getUniformLocation(this.checkerProgram, "sizePx"), rect.width * dpr, rect.height * dpr);
    gl.uniform1f(gl.getUniformLocation(this.checkerProgram, "cell"), options.checkerboard ? 8 * dpr : 1e9);
    if (options.checkerboard) gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
    else { gl.enable(gl.SCISSOR_TEST); gl.scissor(Math.round(rect.x * dpr), Math.round(H - (rect.y + rect.height) * dpr), Math.round(rect.width * dpr), Math.round(rect.height * dpr)); gl.clearColor(0, 0, 0, 0); gl.clear(gl.COLOR_BUFFER_BIT); gl.disable(gl.SCISSOR_TEST); }
    // Clip layers to the document.
    gl.enable(gl.SCISSOR_TEST);
    gl.scissor(Math.round(rect.x * dpr), Math.round(H - (rect.y + rect.height) * dpr), Math.round(rect.width * dpr), Math.round(rect.height * dpr));
    gl.useProgram(this.layerProgram);
    const uMat = gl.getUniformLocation(this.layerProgram, "unitToClip");
    const uOpacity = gl.getUniformLocation(this.layerProgram, "opacity");
    gl.uniform1i(gl.getUniformLocation(this.layerProgram, "tex"), 0);
    for (const layer of renderOrder(state)) {
      const t = this.textures.get(layer.id);
      if (!t) continue;
      const m = this.layerToView(layer, viewport, state);
      gl.uniform1f(uOpacity, layer.opacity);
      for (const chunk of t.chunks) {
        // Sub-rectangle of the unit square this chunk covers.
        const u0 = chunk.x / t.width, v0 = chunk.y / t.height, uw = chunk.width / t.width, vh2 = chunk.height / t.height;
        const a = m.a * uw, b = m.b * uw, c = m.c * vh2, d = m.d * vh2;
        const tx = m.tx + m.a * u0 + m.c * v0, ty = m.ty + m.b * u0 + m.d * v0;
        gl.uniformMatrix3fv(uMat, false, unitToClip(a, b, c, d, tx, ty, vw, vh));
        gl.bindTexture(gl.TEXTURE_2D, chunk.texture);
        gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
      }
    }
    gl.disable(gl.SCISSOR_TEST);
  }

  readPixels(): Uint8Array {
    const gl = this.gl; const W = this.canvas.width, H = this.canvas.height;
    const out = new Uint8Array(W * H * 4);
    gl.readPixels(0, 0, W, H, gl.RGBA, gl.UNSIGNED_BYTE, out);
    // Flip bottom-up to top-down.
    const row = W * 4; const flipped = new Uint8Array(W * H * 4);
    for (let y = 0; y < H; y++) flipped.set(out.subarray(y * row, (y + 1) * row), (H - 1 - y) * row);
    return flipped;
  }
  dispose(): void { this.textures.dispose(); }
}
```

`app/src/canvas/cpu-renderer.ts`:
```ts
import type { DocumentState } from "../engine/types";
import type { EngineClient } from "../engine/client";
import type { Viewport } from "./viewport";
import type { RenderOptions, Renderer } from "./renderer";

/** Asks the engine for the visible document region at the current zoom and blits it. */
export class CpuRenderer implements Renderer {
  readonly kind = "cpu" as const;
  private ctx: CanvasRenderingContext2D;
  constructor(private readonly canvas: HTMLCanvasElement, private readonly engine: EngineClient) {
    this.ctx = canvas.getContext("2d")!;
  }
  sync(): void {}
  render(state: DocumentState, viewport: Viewport, dpr: number, options: RenderOptions): void {
    const W = Math.max(1, Math.round(viewport.viewSize.width * dpr)), H = Math.max(1, Math.round(viewport.viewSize.height * dpr));
    if (this.canvas.width !== W || this.canvas.height !== H) { this.canvas.width = W; this.canvas.height = H; }
    const ctx = this.ctx;
    ctx.fillStyle = "#292929"; ctx.fillRect(0, 0, W, H);
    const rect = viewport.documentRect({ width: state.width, height: state.height });
    const x = Math.round(rect.x * dpr), y = Math.round(rect.y * dpr);
    const w = Math.max(1, Math.round(rect.width * dpr)), h = Math.max(1, Math.round(rect.height * dpr));
    // Only the part of the document inside the canvas element is composited.
    const vx0 = Math.max(0, x), vy0 = Math.max(0, y), vx1 = Math.min(W, x + w), vy1 = Math.min(H, y + h);
    if (vx1 <= vx0 || vy1 <= vy0) return;
    const docPerPx = state.width / w;
    const region = { x: (vx0 - x) * docPerPx, y: (vy0 - y) * docPerPx, width: (vx1 - vx0) * docPerPx, height: (vy1 - vy0) * docPerPx };
    const outW = vx1 - vx0, outH = vy1 - vy0;
    if (options.checkerboard) {
      for (let cy = 0; cy < outH; cy += 8 * dpr) for (let cx = 0; cx < outW; cx += 8 * dpr) {
        ctx.fillStyle = ((Math.floor(cx / (8 * dpr)) + Math.floor(cy / (8 * dpr))) % 2 === 0) ? "#cccccc" : "#f2f2f2";
        ctx.fillRect(vx0 + cx, vy0 + cy, 8 * dpr, 8 * dpr);
      }
    } else { ctx.clearRect(vx0, vy0, outW, outH); }
    const premultiplied = this.engine.composite(state.id, region, outW, outH);
    const straight = new Uint8ClampedArray(premultiplied.length);
    for (let i = 0; i < premultiplied.length; i += 4) {
      const a = premultiplied[i + 3];
      straight[i + 3] = a;
      for (let c = 0; c < 3; c++) straight[i + c] = a === 0 ? 0 : Math.min(255, Math.round(premultiplied[i + c] * 255 / a));
    }
    const image = new ImageData(straight, outW, outH);
    const scratch = document.createElement("canvas"); scratch.width = outW; scratch.height = outH;
    scratch.getContext("2d")!.putImageData(image, 0, 0);
    ctx.drawImage(scratch, vx0, vy0);
  }
  readPixels(): Uint8Array {
    const img = this.ctx.getImageData(0, 0, this.canvas.width, this.canvas.height).data;
    const out = new Uint8Array(img.length);
    for (let i = 0; i < img.length; i += 4) { const a = img[i + 3]; out[i + 3] = a; for (let c = 0; c < 3; c++) out[i + c] = Math.round(img[i + c] * a / 255); }
    return out;
  }
  dispose(): void {}
}
```

- [ ] **Step 5: Implement the overlay and the CanvasView**

`app/src/canvas/overlay.ts`:
```ts
import type { Viewport } from "./viewport";
import type { Rect } from "../tools/crop-geometry";
import { HANDLES } from "../tools/crop-geometry";

export interface OverlayState { docWidth: number; docHeight: number; cropRect: Rect | null; guides: { xs: number[]; ys: number[] }; }

export function drawOverlay(ctx: CanvasRenderingContext2D, viewport: Viewport, dpr: number, state: OverlayState): void {
  const W = ctx.canvas.width, H = ctx.canvas.height;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, W, H);
  const size = { width: state.docWidth, height: state.docHeight };
  const rect = viewport.documentRect(size);
  // Pixel grid once a document pixel is 8 device pixels or larger.
  if (viewport.zoom >= 8) {
    ctx.strokeStyle = "rgba(0,0,0,0.25)"; ctx.lineWidth = 1 / dpr;
    ctx.beginPath();
    const step = viewport.pointsPerPixel;
    const x0 = Math.max(0, Math.floor(-rect.x / step)), x1 = Math.min(state.docWidth, Math.ceil((viewport.viewSize.width - rect.x) / step));
    const y0 = Math.max(0, Math.floor(-rect.y / step)), y1 = Math.min(state.docHeight, Math.ceil((viewport.viewSize.height - rect.y) / step));
    for (let x = x0; x <= x1; x++) { const vx = rect.x + x * step; ctx.moveTo(vx, rect.y); ctx.lineTo(vx, rect.y + rect.height); }
    for (let y = y0; y <= y1; y++) { const vy = rect.y + y * step; ctx.moveTo(rect.x, vy); ctx.lineTo(rect.x + rect.width, vy); }
    ctx.stroke();
  }
  if (state.cropRect) {
    const c = state.cropRect;
    const tl = viewport.viewPoint({ x: c.x, y: c.y }, size), br = viewport.viewPoint({ x: c.x + c.width, y: c.y + c.height }, size);
    ctx.fillStyle = "rgba(0,0,0,0.5)";
    ctx.beginPath();
    ctx.rect(0, 0, viewport.viewSize.width, viewport.viewSize.height);
    ctx.rect(tl.x, tl.y, br.x - tl.x, br.y - tl.y);
    ctx.fill("evenodd");
    ctx.strokeStyle = "white"; ctx.lineWidth = 1;
    ctx.strokeRect(tl.x + 0.5, tl.y + 0.5, br.x - tl.x, br.y - tl.y);
    ctx.strokeStyle = "rgba(255,255,255,0.4)";
    for (const f of [1 / 3, 2 / 3]) {
      ctx.beginPath(); ctx.moveTo(tl.x + (br.x - tl.x) * f, tl.y); ctx.lineTo(tl.x + (br.x - tl.x) * f, br.y); ctx.stroke();
      ctx.beginPath(); ctx.moveTo(tl.x, tl.y + (br.y - tl.y) * f); ctx.lineTo(br.x, tl.y + (br.y - tl.y) * f); ctx.stroke();
    }
    ctx.fillStyle = "white";
    for (const h of HANDLES) { const hx = tl.x + (br.x - tl.x) * h.x, hy = tl.y + (br.y - tl.y) * h.y; ctx.fillRect(hx - 4, hy - 4, 8, 8); }
  }
  ctx.strokeStyle = "#ff40ff"; ctx.lineWidth = 1;
  for (const x of state.guides.xs) { const v = viewport.viewPoint({ x, y: 0 }, size).x; ctx.beginPath(); ctx.moveTo(v + 0.5, 0); ctx.lineTo(v + 0.5, viewport.viewSize.height); ctx.stroke(); }
  for (const y of state.guides.ys) { const v = viewport.viewPoint({ x: 0, y }, size).y; ctx.beginPath(); ctx.moveTo(0, v + 0.5); ctx.lineTo(viewport.viewSize.width, v + 0.5); ctx.stroke(); }
}
```

`app/src/canvas/CanvasView.tsx` (pan, zoom, fit; crop pointer handling is Task 15):
```tsx
import { useEffect, useRef } from "react";
import { useEditor } from "../state/store";
import { createRenderer, type Renderer } from "./renderer";
import { drawOverlay } from "./overlay";
import { installTestApi } from "../test-api";

export const HIT_HANDLE_PX = 6;

export function CanvasView() {
  const glRef = useRef<HTMLCanvasElement>(null);
  const overlayRef = useRef<HTMLCanvasElement>(null);
  const rendererRef = useRef<Renderer | null>(null);
  const checkerboardRef = useRef(true);
  const guidesRef = useRef<{ xs: number[]; ys: number[] }>({ xs: [], ys: [] });
  const engine = useEditor((s) => s.engine);
  const activeId = useEditor((s) => s.activeId);
  const state = useEditor((s) => (s.activeId ? s.documents[s.activeId] : null));
  const viewport = useEditor((s) => (s.activeId ? s.viewports[s.activeId] : null));
  const cropRect = useEditor((s) => s.cropRect);
  const tool = useEditor((s) => s.tool);
  const renderTick = useEditor((s) => s.renderTick);

  // Renderer lifetime follows the canvas element.
  useEffect(() => {
    if (!engine || !glRef.current) return;
    const renderer = createRenderer(glRef.current, engine);
    rendererRef.current = renderer;
    useEditor.getState().setRendererKind(renderer.kind);
    installTestApi({
      renderer,
      setCheckerboard: (on: boolean) => { checkerboardRef.current = on; useEditor.getState().invalidate(); },
      setZoom: async (z: number) => { const s = useEditor.getState(); const vp = s.viewports[s.activeId!]; const d = s.documents[s.activeId!];
        vp.setZoom(z * (window.devicePixelRatio || 1), vp.center, { width: d.width, height: d.height }); s.invalidate();
        await new Promise((r) => requestAnimationFrame(r)); },
      readDocumentPixels: () => {
        const s = useEditor.getState(); const vp = s.viewports[s.activeId!]; const d = s.documents[s.activeId!];
        const dpr = window.devicePixelRatio || 1; const all = renderer.readPixels();
        const rect = vp.documentRect({ width: d.width, height: d.height });
        const W = glRef.current!.width; const x0 = Math.round(rect.x * dpr), y0 = Math.round(rect.y * dpr);
        const w = Math.round(rect.width * dpr), h = Math.round(rect.height * dpr);
        const out = new Uint8Array(w * h * 4);
        for (let y = 0; y < h; y++) out.set(all.subarray(((y0 + y) * W + x0) * 4, ((y0 + y) * W + x0 + w) * 4), y * w * 4);
        return out;
      },
    });
    return () => { renderer.dispose(); rendererRef.current = null; };
  }, [engine]);

  // Size the viewport to the element.
  useEffect(() => {
    const el = glRef.current?.parentElement; if (!el) return;
    const observer = new ResizeObserver(() => {
      const s = useEditor.getState();
      for (const id of s.order) { const d = s.documents[id]; s.viewports[id].resize({ width: el.clientWidth, height: el.clientHeight }, window.devicePixelRatio || 1, { width: d.width, height: d.height }); }
      s.invalidate();
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, [activeId]);

  // Draw on every store change that affects the picture.
  useEffect(() => {
    const renderer = rendererRef.current; const gl = glRef.current; const overlay = overlayRef.current;
    if (!renderer || !gl || !overlay || !state || !viewport || !engine) return;
    const dpr = window.devicePixelRatio || 1;
    renderer.sync(engine, state);
    renderer.render(state, viewport, dpr, { checkerboard: checkerboardRef.current });
    overlay.width = gl.width; overlay.height = gl.height;
    drawOverlay(overlay.getContext("2d")!, viewport, dpr, { docWidth: state.width, docHeight: state.height, cropRect: tool === "crop" ? (cropRect ?? { x: 0, y: 0, width: state.width, height: state.height }) : null, guides: guidesRef.current });
  }, [state, viewport, cropRect, tool, renderTick, engine]);

  // Wheel: zoom with Ctrl, otherwise pan.
  useEffect(() => {
    const el = glRef.current?.parentElement; if (!el) return;
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      const s = useEditor.getState(); if (!s.activeId) return;
      const vp = s.viewports[s.activeId]; const d = s.documents[s.activeId];
      const r = el.getBoundingClientRect(); const anchor = { x: e.clientX - r.left, y: e.clientY - r.top };
      if (e.ctrlKey || e.metaKey) vp.setZoom(vp.zoom * Math.exp(-e.deltaY * 0.002), anchor, { width: d.width, height: d.height });
      else vp.translate({ width: -e.deltaX, height: -e.deltaY });
      s.invalidate();
    };
    el.addEventListener("wheel", onWheel, { passive: false });
    return () => el.removeEventListener("wheel", onWheel);
  }, []);

  // Drag to pan with the hand tool or the space bar.
  useEffect(() => {
    const el = glRef.current?.parentElement; if (!el) return;
    let last: { x: number; y: number } | null = null; let space = false;
    const down = (e: PointerEvent) => { const s = useEditor.getState(); if (s.tool === "hand" || space || e.button === 1) { last = { x: e.clientX, y: e.clientY }; el.setPointerCapture(e.pointerId); } };
    const move = (e: PointerEvent) => { if (!last) return; const s = useEditor.getState(); if (!s.activeId) return; s.viewports[s.activeId].translate({ width: e.clientX - last.x, height: e.clientY - last.y }); last = { x: e.clientX, y: e.clientY }; s.invalidate(); };
    const up = () => { last = null; };
    const key = (e: KeyboardEvent) => { if (e.code === "Space") space = e.type === "keydown"; };
    el.addEventListener("pointerdown", down); el.addEventListener("pointermove", move); el.addEventListener("pointerup", up);
    window.addEventListener("keydown", key); window.addEventListener("keyup", key);
    return () => { el.removeEventListener("pointerdown", down); el.removeEventListener("pointermove", move); el.removeEventListener("pointerup", up); window.removeEventListener("keydown", key); window.removeEventListener("keyup", key); };
  }, []);

  return (
    <div data-testid="canvas-view" style={{ position: "relative", flex: 1, minWidth: 0, minHeight: 0, overflow: "hidden", background: "#292929" }}>
      <canvas ref={glRef} style={{ position: "absolute", inset: 0, width: "100%", height: "100%" }} />
      <canvas ref={overlayRef} data-testid="overlay" style={{ position: "absolute", inset: 0, width: "100%", height: "100%", pointerEvents: "none" }} />
      {!state && <div style={{ position: "absolute", inset: 0, display: "grid", placeItems: "center", color: "#999" }}>Open an image or create a new canvas</div>}
    </div>
  );
}
```

Update `app/src/App.tsx` so it stores the engine in the store (`useEditor.getState().setEngine(e)`), installs `store: useEditor` on the test API, and renders `<CanvasView />` inside a full-height flex column. Keep the `engine-ready` test id on a small status element so existing tests pass.

- [ ] **Step 6: Run the tests to verify they pass**

Run: `pnpm build && pnpm e2e`
Expected: 3 e2e tests pass (smoke, gl match, cpu fallback).

- [ ] **Step 7: Commit**

```
git add app
git commit -m "feat(app): WebGL2 renderer with CPU fallback, overlay and canvas view"
```

---

### Task 13: Shell bridge and Tauri commands

**Files:**
- Create: `src-tauri/Cargo.toml`, `src-tauri/build.rs`, `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`, `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/atomic.rs`, `src-tauri/src/commands/mod.rs`, `src-tauri/src/commands/package.rs`, `src-tauri/src/commands/files.rs`, `src-tauri/src/commands/recent.rs`, `app/src/shell/bridge.ts`, `app/src/shell/tauri-bridge.ts`, `app/src/shell/mock-bridge.ts`
- Modify: `Cargo.toml` (members add `src-tauri`), `package.json` (scripts `tauri:dev`, `tauri:build`)
- Test: `src-tauri/src/atomic.rs` unit tests, `src-tauri/src/commands/package.rs` unit tests, `app/tests/unit/mock-bridge.test.ts`

**Interfaces:**
- `ShellBridge` (TypeScript):
  ```ts
  interface ShellBridge {
    pickOpenPackage(): Promise<string | null>;               // folder path of a .comp
    pickSavePackage(suggested: string): Promise<string | null>;
    pickImportImages(): Promise<string[]>;
    pickExportFile(suggested: string, ext: "png" | "jpg"): Promise<string | null>;
    readPackage(path: string): Promise<PackageFiles>;
    writePackage(path: string, files: PackageFiles): Promise<void>;
    readFile(path: string): Promise<Uint8Array>;
    writeFile(path: string, bytes: Uint8Array): Promise<void>;
    recentPackages(): Promise<string[]>;
    addRecentPackage(path: string): Promise<void>;
    onFileDrop(handler: (paths: string[], position: { x: number; y: number } | null) => void): () => void;
    baseName(path: string): string;
  }
  ```
  `getBridge()` returns the Tauri bridge when `window.__TAURI_INTERNALS__` exists and `VITE_BRIDGE !== "mock"`, else the mock. The mock keeps an in-memory file system (`Map<string, Uint8Array>` and package folders) and exposes `mock.seedFile(path, bytes)`, `mock.setNextPick(path)` for tests.
- Tauri commands (Rust): `read_package_manifest(path) -> { manifest: String, image_names: Vec<String> }`, `read_package_image(path, name) -> tauri::ipc::Response` (raw bytes), `write_package_begin(path) -> token`, `write_package_manifest(token, manifest)`, `write_package_image(request: tauri::ipc::Request)` with headers `token` and `name` and a raw body, `write_package_commit(token) -> ()`, `write_package_abort(token)`, `read_file(path) -> Response`, `write_file(request)` with header `path` and raw body, `recent_packages() -> Vec<String>`, `add_recent_package(path)`. Image names must match `^[0-9A-Fa-f]{8}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{12}(\.mask)?\.png$`. The manifest is capped at 4 MiB and images at 512 MiB before reading.
- `atomic::stage_dir(final_path) -> PathBuf` creates `<final>.tmp-<uuid>`; `atomic::commit(stage, final_path)` renames the existing folder to `<final>.bak-<uuid>`, renames the stage into place, deletes the backup; on failure after the first rename it restores the backup.

- [ ] **Step 1: Write the failing Rust tests**

In `src-tauri/src/atomic.rs` (tests module at the bottom of the file):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("compositor-atomic-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn commit_replaces_existing_package_and_removes_backup() {
        let root = temp();
        let final_path = root.join("A.comp");
        fs::create_dir_all(final_path.join("images")).unwrap();
        fs::write(final_path.join("manifest.json"), b"old").unwrap();
        fs::write(final_path.join("images").join("x.png"), b"x").unwrap();
        let stage = stage_dir(&final_path).unwrap();
        fs::create_dir_all(stage.join("images")).unwrap();
        fs::write(stage.join("manifest.json"), b"new").unwrap();
        commit(&stage, &final_path).unwrap();
        assert_eq!(fs::read(final_path.join("manifest.json")).unwrap(), b"new");
        assert!(!final_path.join("images").join("x.png").exists(), "removed assets are dropped");
        let leftovers: Vec<_> = fs::read_dir(&root).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().to_string()).collect();
        assert_eq!(leftovers, vec!["A.comp".to_string()]);
    }

    #[test]
    fn commit_into_a_file_path_fails_and_keeps_the_original() {
        let root = temp();
        let blocker = root.join("not-a-directory");
        fs::write(&blocker, b"1").unwrap();
        let final_path = blocker.join("CannotSave.comp");
        assert!(stage_dir(&final_path).is_err());
        assert_eq!(fs::read(&blocker).unwrap(), b"1");
    }
}
```

In `src-tauri/src/commands/package.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn image_names_are_uuid_pngs_only() {
        assert!(valid_image_name("E621E1F8-C36C-495A-93FC-0C247A3E6E5F.png"));
        assert!(valid_image_name("e621e1f8-c36c-495a-93fc-0c247a3e6e5f.mask.png"));
        assert!(!valid_image_name("../../outside.png"));
        assert!(!valid_image_name("E621E1F8-C36C-495A-93FC-0C247A3E6E5F.jpg"));
        assert!(!valid_image_name("images/E621E1F8-C36C-495A-93FC-0C247A3E6E5F.png"));
    }
}
```

`app/tests/unit/mock-bridge.test.ts`:
```ts
import { describe, expect, it } from "vitest";
import { MockBridge } from "../../src/shell/mock-bridge";

describe("MockBridge", () => {
  it("round-trips packages and files in memory", async () => {
    const b = new MockBridge();
    await b.writePackage("C:/p/A.comp", { manifest: "{}", images: [{ name: "X.png", bytes: new Uint8Array([1, 2]) }] });
    const pkg = await b.readPackage("C:/p/A.comp");
    expect(pkg.manifest).toBe("{}");
    expect(Array.from(pkg.images[0].bytes)).toEqual([1, 2]);
    await b.writeFile("C:/p/out.png", new Uint8Array([9]));
    expect(Array.from(await b.readFile("C:/p/out.png"))).toEqual([9]);
    b.setNextPick("C:/p/A.comp");
    expect(await b.pickOpenPackage()).toBe("C:/p/A.comp");
    expect(await b.pickOpenPackage()).toBeNull();
    expect(b.baseName("C:/p/A.comp")).toBe("A");
  });
});
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p compositor-shell` and `pnpm test`
Expected: crate not found; module not found.

- [ ] **Step 3: Create the Tauri crate**

`src-tauri/Cargo.toml`:
```toml
[package]
name = "compositor-shell"
version.workspace = true
edition.workspace = true

[lib]
name = "compositor_shell_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-dialog = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4"] }
regex = "1"
```

`src-tauri/build.rs`:
```rust
fn main() { tauri_build::build() }
```

`src-tauri/src/main.rs`:
```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
fn main() { compositor_shell_lib::run() }
```

`src-tauri/src/lib.rs`:
```rust
mod atomic;
mod commands;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::package::read_package_manifest, commands::package::read_package_image,
            commands::package::write_package_begin, commands::package::write_package_manifest,
            commands::package::write_package_image, commands::package::write_package_commit, commands::package::write_package_abort,
            commands::files::read_file, commands::files::write_file,
            commands::recent::recent_packages, commands::recent::add_recent_package,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Compositor");
}
```

`src-tauri/src/atomic.rs`:
```rust
use std::fs;
use std::path::{Path, PathBuf};

fn sibling(final_path: &Path, tag: &str) -> PathBuf {
    let name = final_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    final_path.with_file_name(format!("{name}.{tag}-{}", uuid::Uuid::new_v4()))
}

/// A fresh sibling directory to write the new package into.
pub fn stage_dir(final_path: &Path) -> std::io::Result<PathBuf> {
    let parent = final_path.parent().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "no parent folder"))?;
    if !parent.is_dir() { return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "the destination folder does not exist")); }
    let stage = sibling(final_path, "tmp");
    fs::create_dir(&stage)?;
    Ok(stage)
}

/// Swaps the staged directory into place. Either the old package or the new one survives a failure.
pub fn commit(stage: &Path, final_path: &Path) -> std::io::Result<()> {
    let backup = if final_path.exists() {
        let b = sibling(final_path, "bak");
        fs::rename(final_path, &b)?;
        Some(b)
    } else { None };
    if let Err(e) = fs::rename(stage, final_path) {
        if let Some(b) = &backup { let _ = fs::rename(b, final_path); }
        let _ = fs::remove_dir_all(stage);
        return Err(e);
    }
    if let Some(b) = backup { fs::remove_dir_all(b)?; }
    Ok(())
}

pub fn abort(stage: &Path) { let _ = fs::remove_dir_all(stage); }
```

`src-tauri/src/commands/mod.rs`:
```rust
pub mod files;
pub mod package;
pub mod recent;
```

`src-tauri/src/commands/package.rs`:
```rust
use crate::atomic;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::ipc::{Request, Response};
use tauri::State;

const MAX_MANIFEST: u64 = 4 * 1024 * 1024;
const MAX_ASSET: u64 = 512 * 1024 * 1024;

pub fn valid_image_name(name: &str) -> bool {
    let re = regex::Regex::new(r"^[0-9A-Fa-f]{8}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{12}(\.mask)?\.png$").unwrap();
    re.is_match(name)
}

#[derive(Serialize)]
pub struct PackageHeader { pub manifest: String, pub image_names: Vec<String> }

fn io(e: std::io::Error, what: &str, path: &Path) -> String { format!("{what} {}: {e}", path.display()) }

#[tauri::command]
pub fn read_package_manifest(path: String) -> Result<PackageHeader, String> {
    let root = PathBuf::from(&path);
    if !root.is_dir() { return Err(format!("{} is not a Compositor project folder.", root.display())); }
    let manifest_path = root.join("manifest.json");
    let meta = fs::metadata(&manifest_path).map_err(|e| io(e, "Cannot read", &manifest_path))?;
    if !meta.is_file() || meta.len() > MAX_MANIFEST { return Err("This project exceeds the supported manifest size.".into()); }
    let manifest = fs::read_to_string(&manifest_path).map_err(|e| io(e, "Cannot read", &manifest_path))?;
    let mut image_names = Vec::new();
    if let Ok(entries) = fs::read_dir(root.join("images")) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if valid_image_name(&name) && entry.file_type().map(|t| t.is_file()).unwrap_or(false) { image_names.push(name); }
        }
    }
    image_names.sort();
    Ok(PackageHeader { manifest, image_names })
}

#[tauri::command]
pub fn read_package_image(path: String, name: String) -> Result<Response, String> {
    if !valid_image_name(&name) { return Err("Invalid image name.".into()); }
    let file = PathBuf::from(&path).join("images").join(&name);
    let meta = fs::metadata(&file).map_err(|e| io(e, "Cannot read", &file))?;
    if !meta.is_file() || meta.len() > MAX_ASSET { return Err("This project exceeds the supported asset size.".into()); }
    Ok(Response::new(fs::read(&file).map_err(|e| io(e, "Cannot read", &file))?))
}

#[derive(Default)]
pub struct PendingWrites(pub Mutex<HashMap<String, (PathBuf, PathBuf)>>); // token -> (stage, final)

#[tauri::command]
pub fn write_package_begin(path: String, pending: State<PendingWrites>) -> Result<String, String> {
    let final_path = PathBuf::from(&path);
    let stage = atomic::stage_dir(&final_path).map_err(|e| io(e, "Cannot save to", &final_path))?;
    fs::create_dir(stage.join("images")).map_err(|e| io(e, "Cannot create", &stage))?;
    let token = uuid::Uuid::new_v4().to_string();
    pending.0.lock().unwrap().insert(token.clone(), (stage, final_path));
    Ok(token)
}

fn stage_for(token: &str, pending: &State<PendingWrites>) -> Result<(PathBuf, PathBuf), String> {
    pending.0.lock().unwrap().get(token).cloned().ok_or_else(|| "Unknown save token.".to_string())
}

#[tauri::command]
pub fn write_package_manifest(token: String, manifest: String, pending: State<PendingWrites>) -> Result<(), String> {
    let (stage, _) = stage_for(&token, &pending)?;
    if manifest.len() as u64 > MAX_MANIFEST { return Err("Manifest too large.".into()); }
    let p = stage.join("manifest.json");
    fs::write(&p, manifest).map_err(|e| io(e, "Cannot write", &p))
}

#[tauri::command]
pub fn write_package_image(request: Request<'_>, pending: State<PendingWrites>) -> Result<(), String> {
    let header = |k: &str| request.headers().get(k).and_then(|v| v.to_str().ok()).map(|s| s.to_string()).ok_or_else(|| format!("missing header {k}"));
    let token = header("token")?; let name = header("name")?;
    if !valid_image_name(&name) { return Err("Invalid image name.".into()); }
    let (stage, _) = stage_for(&token, &pending)?;
    let bytes = match request.body() { tauri::ipc::InvokeBody::Raw(b) => b.clone(), _ => return Err("expected raw body".into()) };
    if bytes.len() as u64 > MAX_ASSET { return Err("Asset too large.".into()); }
    let p = stage.join("images").join(&name);
    fs::write(&p, bytes).map_err(|e| io(e, "Cannot write", &p))
}

#[tauri::command]
pub fn write_package_commit(token: String, pending: State<PendingWrites>) -> Result<(), String> {
    let (stage, final_path) = pending.0.lock().unwrap().remove(&token).ok_or_else(|| "Unknown save token.".to_string())?;
    atomic::commit(&stage, &final_path).map_err(|e| io(e, "Cannot replace", &final_path))
}

#[tauri::command]
pub fn write_package_abort(token: String, pending: State<PendingWrites>) {
    if let Some((stage, _)) = pending.0.lock().unwrap().remove(&token) { atomic::abort(&stage); }
}
```

Register the state in `lib.rs` with `.manage(commands::package::PendingWrites::default())` before `.invoke_handler`.

`src-tauri/src/commands/files.rs`:
```rust
use std::fs;
use std::path::PathBuf;
use tauri::ipc::{InvokeBody, Request, Response};

const MAX_FILE: u64 = 512 * 1024 * 1024;

#[tauri::command]
pub fn read_file(path: String) -> Result<Response, String> {
    let p = PathBuf::from(&path);
    let meta = fs::metadata(&p).map_err(|e| format!("Cannot read {}: {e}", p.display()))?;
    if !meta.is_file() || meta.len() > MAX_FILE { return Err(format!("{} is not a readable file under 512 MiB.", p.display())); }
    Ok(Response::new(fs::read(&p).map_err(|e| format!("Cannot read {}: {e}", p.display()))?))
}

/// Header `path`, raw body. Writes to a sibling temp file then renames over the target.
#[tauri::command]
pub fn write_file(request: Request<'_>) -> Result<(), String> {
    let path = request.headers().get("path").and_then(|v| v.to_str().ok()).ok_or("missing header path")?;
    let path = PathBuf::from(percent_decode(path));
    let bytes = match request.body() { InvokeBody::Raw(b) => b, _ => return Err("expected raw body".into()) };
    let tmp = path.with_extension(format!("tmp-{}", uuid::Uuid::new_v4()));
    fs::write(&tmp, bytes).map_err(|e| format!("Cannot write {}: {e}", tmp.display()))?;
    if let Err(e) = fs::rename(&tmp, &path) { let _ = fs::remove_file(&tmp); return Err(format!("Cannot replace {}: {e}", path.display())); }
    Ok(())
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes(); let mut out = Vec::with_capacity(bytes.len()); let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() + 0 && i + 2 <= bytes.len() - 1 {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) { out.push(v); i += 3; continue; }
        }
        out.push(bytes[i]); i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}
```
Headers are ASCII only, so the TypeScript side percent-encodes paths (`encodeURIComponent`) before putting them in the `path` and `name` headers, and Rust decodes them with `percent_decode`. Apply the same decoding to `name` and `token` in `write_package_image`.

`src-tauri/src/commands/recent.rs`:
```rust
use std::fs;
use tauri::{AppHandle, Manager};

fn store_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("recent.json"))
}

#[tauri::command]
pub fn recent_packages(app: AppHandle) -> Result<Vec<String>, String> {
    let p = store_path(&app)?;
    if !p.exists() { return Ok(vec![]); }
    let text = fs::read_to_string(&p).map_err(|e| e.to_string())?;
    let list: Vec<String> = serde_json::from_str(&text).unwrap_or_default();
    Ok(list.into_iter().filter(|s| std::path::Path::new(s).is_dir()).collect())
}

#[tauri::command]
pub fn add_recent_package(app: AppHandle, path: String) -> Result<(), String> {
    let mut list = recent_packages(app.clone())?;
    list.retain(|p| p != &path);
    list.insert(0, path);
    list.truncate(10);
    fs::write(store_path(&app)?, serde_json::to_string(&list).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
```

`src-tauri/tauri.conf.json`:
```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Compositor",
  "version": "0.1.0",
  "identifier": "com.compositor.windows",
  "build": {
    "frontendDist": "../app/dist",
    "devUrl": "http://localhost:1420",
    "beforeDevCommand": "pnpm dev",
    "beforeBuildCommand": "pnpm build"
  },
  "app": {
    "windows": [{ "title": "Compositor", "width": 1440, "height": 900, "minWidth": 960, "minHeight": 600, "dragDropEnabled": true }],
    "security": {
      "csp": "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; connect-src 'self' ipc: http://ipc.localhost"
    }
  },
  "bundle": { "active": false, "icon": ["icons/32x32.png", "icons/128x128.png", "icons/128x128@2x.png", "icons/icon.ico"] }
}
```

`src-tauri/capabilities/default.json`:
```json
{
  "identifier": "default",
  "windows": ["main"],
  "permissions": ["core:default", "core:window:allow-set-title", "dialog:default"]
}
```

Generate icons from the macOS app icon: copy `../Compositor/Compositor/Assets.xcassets/AppIcon.appiconset/app-icon-1024.png` to `src-tauri/app-icon.png` and run `pnpm tauri icon src-tauri/app-icon.png`.

Add `"src-tauri"` to the workspace members and the scripts `"tauri:dev": "tauri dev"`, `"tauri:build": "tauri build --no-bundle"` to `package.json`.

- [ ] **Step 4: Implement the bridges**

`app/src/shell/bridge.ts`:
```ts
import type { PackageFiles } from "../engine/types";

export interface ShellBridge {
  pickOpenPackage(): Promise<string | null>;
  pickSavePackage(suggested: string): Promise<string | null>;
  pickImportImages(): Promise<string[]>;
  pickExportFile(suggested: string, ext: "png" | "jpg"): Promise<string | null>;
  readPackage(path: string): Promise<PackageFiles>;
  writePackage(path: string, files: PackageFiles): Promise<void>;
  readFile(path: string): Promise<Uint8Array>;
  writeFile(path: string, bytes: Uint8Array): Promise<void>;
  recentPackages(): Promise<string[]>;
  addRecentPackage(path: string): Promise<void>;
  onFileDrop(handler: (paths: string[], position: { x: number; y: number } | null) => void): () => void;
  baseName(path: string): string;
}

export function baseName(path: string): string {
  const last = path.split(/[\\/]/).filter(Boolean).pop() ?? "";
  return last.replace(/\.[^.]+$/, "");
}

let bridge: ShellBridge | null = null;
export async function getBridge(): Promise<ShellBridge> {
  if (bridge) return bridge;
  const isTauri = "__TAURI_INTERNALS__" in window && import.meta.env.VITE_BRIDGE !== "mock";
  bridge = isTauri ? new (await import("./tauri-bridge")).TauriBridge() : new (await import("./mock-bridge")).MockBridge();
  return bridge;
}
```

`app/src/shell/tauri-bridge.ts`:
```ts
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open, save } from "@tauri-apps/plugin-dialog";
import type { PackageFiles } from "../engine/types";
import { baseName, type ShellBridge } from "./bridge";

export class TauriBridge implements ShellBridge {
  async pickOpenPackage(): Promise<string | null> {
    const picked = await open({ directory: true, multiple: false, title: "Open Compositor Project (.comp folder)" });
    return typeof picked === "string" ? picked : null;
  }
  async pickSavePackage(suggested: string): Promise<string | null> {
    const picked = await save({ defaultPath: suggested.endsWith(".comp") ? suggested : `${suggested}.comp`, filters: [{ name: "Compositor Project", extensions: ["comp"] }] });
    return picked ? (picked.endsWith(".comp") ? picked : `${picked}.comp`) : null;
  }
  async pickImportImages(): Promise<string[]> {
    const picked = await open({ multiple: true, filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "tif", "tiff", "webp", "bmp"] }] });
    return Array.isArray(picked) ? picked : picked ? [picked] : [];
  }
  async pickExportFile(suggested: string, ext: "png" | "jpg"): Promise<string | null> {
    return (await save({ defaultPath: `${suggested}.${ext}`, filters: [{ name: ext.toUpperCase(), extensions: ext === "jpg" ? ["jpg", "jpeg"] : ["png"] }] })) ?? null;
  }
  async readPackage(path: string): Promise<PackageFiles> {
    const header = await invoke<{ manifest: string; image_names: string[] }>("read_package_manifest", { path });
    const images = [];
    for (const name of header.image_names) {
      const bytes = await invoke<ArrayBuffer>("read_package_image", { path, name });
      images.push({ name, bytes: new Uint8Array(bytes) });
    }
    return { manifest: header.manifest, images };
  }
  async writePackage(path: string, files: PackageFiles): Promise<void> {
    const token = await invoke<string>("write_package_begin", { path });
    try {
      await invoke("write_package_manifest", { token, manifest: files.manifest });
      for (const image of files.images) {
        await invoke("write_package_image", image.bytes, { headers: { token, name: encodeURIComponent(image.name) } });
      }
      await invoke("write_package_commit", { token });
    } catch (e) { await invoke("write_package_abort", { token }); throw e; }
  }
  async readFile(path: string): Promise<Uint8Array> { return new Uint8Array(await invoke<ArrayBuffer>("read_file", { path })); }
  async writeFile(path: string, bytes: Uint8Array): Promise<void> { await invoke("write_file", bytes, { headers: { path: encodeURIComponent(path) } }); }
  recentPackages(): Promise<string[]> { return invoke<string[]>("recent_packages"); }
  async addRecentPackage(path: string): Promise<void> { await invoke("add_recent_package", { path }); }
  onFileDrop(handler: (paths: string[], position: { x: number; y: number } | null) => void): () => void {
    let unlisten: (() => void) | null = null;
    getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type === "drop") handler(event.payload.paths, event.payload.position ? { x: event.payload.position.x, y: event.payload.position.y } : null);
    }).then((u) => { unlisten = u; });
    return () => unlisten?.();
  }
  baseName(path: string): string { return baseName(path); }
}
```

`app/src/shell/mock-bridge.ts`:
```ts
import type { PackageFiles } from "../engine/types";
import { baseName, type ShellBridge } from "./bridge";

/** In-memory shell for browser tests. */
export class MockBridge implements ShellBridge {
  private files = new Map<string, Uint8Array>();
  private packages = new Map<string, PackageFiles>();
  private picks: (string | null)[] = [];
  private dropHandlers: ((paths: string[], position: { x: number; y: number } | null) => void)[] = [];
  private recent: string[] = [];

  setNextPick(path: string | null): void { this.picks.push(path); }
  seedFile(path: string, bytes: Uint8Array): void { this.files.set(path, bytes); }
  simulateDrop(paths: string[], position: { x: number; y: number } | null): void { for (const h of this.dropHandlers) h(paths, position); }
  hasPackage(path: string): boolean { return this.packages.has(path); }
  fileBytes(path: string): Uint8Array | undefined { return this.files.get(path); }

  private nextPick(): string | null { return this.picks.length ? this.picks.shift()! : null; }
  async pickOpenPackage(): Promise<string | null> { return this.nextPick(); }
  async pickSavePackage(): Promise<string | null> { return this.nextPick(); }
  async pickImportImages(): Promise<string[]> { const p = this.nextPick(); return p ? [p] : []; }
  async pickExportFile(): Promise<string | null> { return this.nextPick(); }
  async readPackage(path: string): Promise<PackageFiles> {
    const p = this.packages.get(path); if (!p) throw new Error(`${path} is not a Compositor project folder.`);
    return { manifest: p.manifest, images: p.images.map((i) => ({ name: i.name, bytes: new Uint8Array(i.bytes) })) };
  }
  async writePackage(path: string, files: PackageFiles): Promise<void> { this.packages.set(path, { manifest: files.manifest, images: files.images.map((i) => ({ name: i.name, bytes: new Uint8Array(i.bytes) })) }); }
  async readFile(path: string): Promise<Uint8Array> { const f = this.files.get(path); if (!f) throw new Error(`Cannot read ${path}`); return new Uint8Array(f); }
  async writeFile(path: string, bytes: Uint8Array): Promise<void> { this.files.set(path, new Uint8Array(bytes)); }
  async recentPackages(): Promise<string[]> { return [...this.recent]; }
  async addRecentPackage(path: string): Promise<void> { this.recent = [path, ...this.recent.filter((p) => p !== path)].slice(0, 10); }
  onFileDrop(handler: (paths: string[], position: { x: number; y: number } | null) => void): () => void {
    this.dropHandlers.push(handler);
    return () => { this.dropHandlers = this.dropHandlers.filter((h) => h !== handler); };
  }
  baseName(path: string): string { return baseName(path); }
}
```

- [ ] **Step 5: Run the tests and a dev launch**

```
cargo test -p compositor-shell
pnpm test
pnpm wasm:dev
pnpm tauri:dev
```
Expected: Rust tests pass (3), vitest passes, and a Compositor window opens showing the engine-ready status with the empty canvas view. Close it.

- [ ] **Step 6: Commit**

```
git add Cargo.toml package.json src-tauri app
git commit -m "feat(shell): Tauri 2 shell with package/file commands and shell bridges"
```

---

### Task 14: File workflow UI: menu, new canvas, open, save, import, export PNG, tabs, layer list

**Files:**
- Create: `app/src/actions/files.ts`, `app/src/panels/MenuBar.tsx`, `app/src/panels/ProjectTabs.tsx`, `app/src/panels/LayersList.tsx`, `app/src/panels/ToolRail.tsx`, `app/src/sheets/Sheet.tsx`, `app/src/sheets/NewCanvasSheet.tsx`, `app/src/styles.css`
- Modify: `app/src/App.tsx`, `app/src/state/store.ts` (add `bridge`, `busy`)
- Test: `app/tests/e2e/files.spec.ts`

**Interfaces:**
- `actions/files.ts` (all take the store and bridge from `useEditor.getState()`): `newCanvas(width, height)`, `openProject(path?: string)`, `saveProject()`, `saveProjectAs()`, `importImages(paths?: string[], at?: {x,y})`, `exportPng()`, `closeActive(): Promise<boolean>` (prompts to save when modified via `window.confirm`; returns false when cancelled). Errors go to `store.error`, shown in a dismissible banner with `data-testid="error-banner"`.
- Store additions: `bridge: ShellBridge | null`, `busy: boolean` (disables menus during file operations), `setBridge`, `setBusy`.
- `MenuBar`: File (New Canvas, Open Project, Open Recent, Import Images, Save, Save As, Export PNG, Export JPEG, Close), Edit (Undo, Redo), Image (Canvas Size, Image Size, Flip Canvas Horizontal, Flip Canvas Vertical), View (Zoom In, Zoom Out, Fit, Actual Size). Menu items carry `data-testid="menu-<id>"`.
- `ProjectTabs`: one tab per open document; title = base name of `path` or "Untitled"; a dot when modified; close button.
- `LayersList`: rows bottom-to-top reversed (top of list = top layer), indent by depth, visibility checkbox, click selects (`SetActiveLayer`), double-click renames inline (`RenameLayer`), Delete key removes the active layer, "+" adds a blank layer.
- `ToolRail`: buttons for move (V), hand (H), zoom (Z), crop (C).
- `Sheet`: modal container with title, body, Cancel and primary button; Esc cancels, Enter confirms.
- `NewCanvasSheet`: width and height inputs defaulting to 1920 by 1080, validation 1 to 30,000.

- [ ] **Step 1: Write the failing e2e test**

`app/tests/e2e/files.spec.ts`:
```ts
import { test, expect, type Page } from "@playwright/test";

const RED_PNG = "iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAYAAABytg0kAAAAEUlEQVQIW2P8z8DwnwEIGGEMAD1sBAOiD07mAAAAAElFTkSuQmCC";

async function seedImage(page: Page, path: string) {
  await page.evaluate(({ path, b64 }) => {
    const api = (window as unknown as { __compositor: any }).__compositor;
    api.bridge.seedFile(path, Uint8Array.from(atob(b64), (c) => c.charCodeAt(0)));
  }, { path, b64: RED_PNG });
}

test("new canvas, import, save, close and reopen", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  await page.getByTestId("menu-new").click();
  await page.getByLabel("Width").fill("300");
  await page.getByLabel("Height").fill("200");
  await page.getByRole("button", { name: "Create" }).click();
  await expect(page.getByTestId("project-tab").first()).toContainText("Untitled");
  await expect(page.getByTestId("layer-row")).toHaveCount(1);

  await seedImage(page, "C:/pics/red.png");
  await page.evaluate(() => (window as any).__compositor.bridge.setNextPick("C:/pics/red.png"));
  await page.getByTestId("menu-import").click();
  await expect(page.getByTestId("layer-row")).toHaveCount(2);
  await expect(page.getByTestId("layer-row").first()).toContainText("red");

  await page.evaluate(() => (window as any).__compositor.bridge.setNextPick("C:/projects/Test.comp"));
  await page.getByTestId("menu-save").click();
  await expect(page.getByTestId("project-tab").first()).toContainText("Test");
  await expect(page.getByTestId("project-tab").first()).not.toContainText("\u2022");
  const saved = await page.evaluate(() => (window as any).__compositor.bridge.hasPackage("C:/projects/Test.comp"));
  expect(saved).toBe(true);

  await page.getByTestId("menu-close").click();
  await expect(page.getByTestId("project-tab")).toHaveCount(0);

  await page.evaluate(() => (window as any).__compositor.bridge.setNextPick("C:/projects/Test.comp"));
  await page.getByTestId("menu-open").click();
  await expect(page.getByTestId("layer-row")).toHaveCount(2);
  const state = await page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId]; });
  expect([state.width, state.height]).toEqual([300, 200]);
  expect(state.isModified).toBe(false);
});

test("export png writes a file of the canvas size", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  await page.getByTestId("menu-new").click();
  await page.getByRole("button", { name: "Create" }).click();
  await page.evaluate(() => (window as any).__compositor.bridge.setNextPick("C:/out/Untitled.png"));
  await page.getByTestId("menu-export-png").click();
  const size = await page.evaluate(async () => {
    const bytes = (window as any).__compositor.bridge.fileBytes("C:/out/Untitled.png") as Uint8Array;
    const blob = new Blob([bytes], { type: "image/png" });
    const bmp = await createImageBitmap(blob);
    return [bmp.width, bmp.height];
  });
  expect(size).toEqual([1920, 1080]);
});

test("closing a modified document asks first", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  await page.getByTestId("menu-new").click();
  await page.getByRole("button", { name: "Create" }).click();
  await page.getByTestId("layer-add").click();
  await expect(page.getByTestId("project-tab").first()).toContainText("\u2022");
  page.once("dialog", (d) => d.dismiss());
  await page.getByTestId("menu-close").click();
  await expect(page.getByTestId("project-tab")).toHaveCount(1);
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `pnpm e2e --grep "new canvas"`
Expected: `menu-new` not found.

- [ ] **Step 3: Implement file actions**

`app/src/actions/files.ts`:
```ts
import { useEditor } from "../state/store";

function ctx() {
  const s = useEditor.getState();
  if (!s.engine || !s.bridge) throw new Error("Engine not ready");
  return { s, engine: s.engine, bridge: s.bridge };
}

async function guarded(work: () => Promise<void>): Promise<void> {
  const s = useEditor.getState();
  if (s.busy) return;
  s.setBusy(true);
  try { await work(); }
  catch (e) { useEditor.getState().setError(e instanceof Error ? e.message : String(e)); }
  finally { useEditor.getState().setBusy(false); }
}

export function newCanvas(width: number, height: number): void {
  const { s, engine } = ctx();
  const id = engine.newDocument(width, height, true);
  s.openDocument(id);
  s.closeSheet();
}

export async function openProject(path?: string): Promise<void> {
  await guarded(async () => {
    const { s, engine, bridge } = ctx();
    const target = path ?? (await bridge.pickOpenPackage());
    if (!target) return;
    const existing = s.order.find((id) => s.documents[id].path === target);
    if (existing) { s.setActive(existing); return; }
    const files = await bridge.readPackage(target);
    const id = engine.openPackage(files, target);
    s.openDocument(id);
    await bridge.addRecentPackage(target);
  });
}

async function saveTo(path: string): Promise<void> {
  const { s, engine, bridge } = ctx();
  const id = s.activeId; if (!id) return;
  const files = engine.savePackage(id);
  await bridge.writePackage(path, files);
  engine.markSaved(id, path);
  s.refresh(id);
  await bridge.addRecentPackage(path);
}

export async function saveProject(): Promise<void> {
  await guarded(async () => {
    const { s } = ctx();
    const doc = s.activeId ? s.documents[s.activeId] : null;
    if (!doc) return;
    if (doc.path) await saveTo(doc.path); else await saveProjectAs();
  });
}

export async function saveProjectAs(): Promise<void> {
  const { s, bridge } = ctx();
  const doc = s.activeId ? s.documents[s.activeId] : null;
  if (!doc) return;
  const suggested = doc.path ? bridge.baseName(doc.path) : "Untitled";
  const path = await bridge.pickSavePackage(suggested);
  if (path) await saveTo(path);
}

export async function importImages(paths?: string[], at?: { x: number; y: number }): Promise<void> {
  await guarded(async () => {
    const { s, engine, bridge } = ctx();
    const files = paths ?? (await bridge.pickImportImages());
    const failures: string[] = [];
    let target = s.activeId;
    for (const path of files) {
      try {
        const bytes = await bridge.readFile(path);
        const id = engine.importImage(target, bytes, bridge.baseName(path), target ? at ?? null : null);
        if (!target) { target = id; useEditor.getState().openDocument(id); } else { useEditor.getState().refresh(id); }
      } catch (e) { failures.push(`${bridge.baseName(path)}: ${e instanceof Error ? e.message : String(e)}`); }
    }
    if (failures.length) throw new Error(failures.join("\n"));
  });
}

export async function exportPng(): Promise<void> {
  await guarded(async () => {
    const { s, engine, bridge } = ctx();
    const doc = s.activeId ? s.documents[s.activeId] : null;
    if (!doc) return;
    const path = await bridge.pickExportFile(doc.path ? bridge.baseName(doc.path) : "Untitled", "png");
    if (!path) return;
    await bridge.writeFile(path, engine.exportPng(doc.id));
  });
}

/** Returns false when the user cancelled. */
export async function closeActive(): Promise<boolean> {
  const { s } = ctx();
  const doc = s.activeId ? s.documents[s.activeId] : null;
  if (!doc) return true;
  if (doc.isModified) {
    const save = window.confirm(`Save changes to ${doc.path ? s.bridge!.baseName(doc.path) : "Untitled"} before closing?\n\nOK saves, Cancel keeps the document open.`);
    if (!save) return false;
    await saveProject();
    if (useEditor.getState().documents[doc.id]?.isModified) return false;
  }
  useEditor.getState().closeDocument(doc.id);
  return true;
}
```

Extend the store with `bridge: ShellBridge | null`, `busy: boolean`, `setBridge`, `setBusy` (same pattern as `setEngine`).

- [ ] **Step 4: Implement the panels and sheet**

`app/src/sheets/Sheet.tsx`:
```tsx
import { useEffect, type ReactNode } from "react";

export function Sheet(props: { title: string; primary: string; canConfirm: boolean; onConfirm(): void; onCancel(): void; children: ReactNode }) {
  useEffect(() => {
    const key = (e: KeyboardEvent) => { if (e.key === "Escape") props.onCancel(); if (e.key === "Enter" && props.canConfirm) props.onConfirm(); };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [props]);
  return (
    <div className="sheet-backdrop" role="dialog" aria-label={props.title}>
      <div className="sheet">
        <h2>{props.title}</h2>
        <div className="sheet-body">{props.children}</div>
        <div className="sheet-buttons">
          <button onClick={props.onCancel}>Cancel</button>
          <button className="primary" disabled={!props.canConfirm} onClick={props.onConfirm}>{props.primary}</button>
        </div>
      </div>
    </div>
  );
}
```

`app/src/sheets/NewCanvasSheet.tsx`:
```tsx
import { useState } from "react";
import { Sheet } from "./Sheet";
import { newCanvas } from "../actions/files";
import { useEditor } from "../state/store";

function dimension(text: string): number | null { const n = Number(text.trim()); return Number.isInteger(n) && n >= 1 && n <= 30_000 ? n : null; }

export function NewCanvasSheet() {
  const [width, setWidth] = useState("1920");
  const [height, setHeight] = useState("1080");
  const close = useEditor((s) => s.closeSheet);
  const w = dimension(width), h = dimension(height);
  return (
    <Sheet title="New Canvas" primary="Create" canConfirm={w !== null && h !== null} onCancel={close} onConfirm={() => newCanvas(w!, h!)}>
      <label>Width <input aria-label="Width" value={width} onChange={(e) => setWidth(e.target.value)} /> px</label>
      <label>Height <input aria-label="Height" value={height} onChange={(e) => setHeight(e.target.value)} /> px</label>
    </Sheet>
  );
}
```

`app/src/panels/MenuBar.tsx`:
```tsx
import { useEffect, useState } from "react";
import { useEditor } from "../state/store";
import { closeActive, exportPng, importImages, openProject, saveProject, saveProjectAs } from "../actions/files";

type Item = { id: string; label: string; run(): void; enabled?: boolean } | "separator";

export function MenuBar() {
  const s = useEditor();
  const hasDoc = s.activeId !== null;
  const [recent, setRecent] = useState<string[]>([]);
  useEffect(() => { s.bridge?.recentPackages().then(setRecent); }, [s.bridge, s.documents]);
  const zoomBy = (factor: number) => { const vp = s.viewports[s.activeId!]; const d = s.documents[s.activeId!]; vp.setZoom(vp.zoom * factor, vp.center, { width: d.width, height: d.height }); s.invalidate(); };
  const menus: { title: string; items: Item[] }[] = [
    { title: "File", items: [
      { id: "new", label: "New Canvas...", run: () => s.openSheet({ kind: "new" }) },
      { id: "open", label: "Open Project...", run: () => void openProject() },
      ...recent.map((p, i) => ({ id: `recent-${i}`, label: `Open Recent: ${s.bridge!.baseName(p)}`, run: () => void openProject(p) })),
      { id: "import", label: "Import Images...", run: () => void importImages() },
      "separator",
      { id: "save", label: "Save", run: () => void saveProject(), enabled: hasDoc },
      { id: "save-as", label: "Save As...", run: () => void saveProjectAs(), enabled: hasDoc },
      { id: "export-png", label: "Export PNG...", run: () => void exportPng(), enabled: hasDoc },
      { id: "export-jpeg", label: "Export JPEG...", run: () => s.openSheet({ kind: "jpeg" }), enabled: hasDoc },
      "separator",
      { id: "close", label: "Close", run: () => void closeActive(), enabled: hasDoc },
    ] },
    { title: "Edit", items: [
      { id: "undo", label: "Undo", run: s.undo, enabled: hasDoc && s.documents[s.activeId!].canUndo },
      { id: "redo", label: "Redo", run: s.redo, enabled: hasDoc && s.documents[s.activeId!].canRedo },
    ] },
    { title: "Image", items: [
      { id: "canvas-size", label: "Canvas Size...", run: () => s.openSheet({ kind: "canvasSize" }), enabled: hasDoc },
      { id: "image-size", label: "Image Size...", run: () => s.openSheet({ kind: "imageSize" }), enabled: hasDoc },
      "separator",
      { id: "flip-h", label: "Flip Canvas Horizontal", run: () => s.run({ type: "FlipCanvas", horizontal: true }), enabled: hasDoc },
      { id: "flip-v", label: "Flip Canvas Vertical", run: () => s.run({ type: "FlipCanvas", horizontal: false }), enabled: hasDoc },
    ] },
    { title: "View", items: [
      { id: "zoom-in", label: "Zoom In", run: () => zoomBy(1.25), enabled: hasDoc },
      { id: "zoom-out", label: "Zoom Out", run: () => zoomBy(0.8), enabled: hasDoc },
      { id: "fit", label: "Fit on Screen", run: () => { const d = s.documents[s.activeId!]; s.viewports[s.activeId!].fit({ width: d.width, height: d.height }); s.invalidate(); }, enabled: hasDoc },
      { id: "actual", label: "Actual Size", run: () => { const vp = s.viewports[s.activeId!]; const d = s.documents[s.activeId!]; vp.setZoom(window.devicePixelRatio || 1, vp.center, { width: d.width, height: d.height }); s.invalidate(); }, enabled: hasDoc },
    ] },
  ];
  const [openMenu, setOpenMenu] = useState<string | null>(null);
  return (
    <div className="menubar" onMouseLeave={() => setOpenMenu(null)}>
      {menus.map((m) => (
        <div key={m.title} className="menu" onMouseEnter={() => openMenu && setOpenMenu(m.title)}>
          <button onClick={() => setOpenMenu(openMenu === m.title ? null : m.title)}>{m.title}</button>
          {openMenu === m.title && (
            <div className="menu-items">
              {m.items.map((it, i) => it === "separator" ? <hr key={i} /> : (
                <button key={it.id} data-testid={`menu-${it.id}`} disabled={it.enabled === false || s.busy} onClick={() => { setOpenMenu(null); it.run(); }}>{it.label}</button>
              ))}
            </div>
          )}
        </div>
      ))}
    </div>
  );
}
```

`app/src/panels/ProjectTabs.tsx`:
```tsx
import { useEditor } from "../state/store";
import { closeActive } from "../actions/files";

export function ProjectTabs() {
  const s = useEditor();
  return (
    <div className="tabs">
      {s.order.map((id) => {
        const d = s.documents[id];
        const title = d.path ? s.bridge!.baseName(d.path) : "Untitled";
        return (
          <div key={id} data-testid="project-tab" className={"tab" + (id === s.activeId ? " active" : "")} onClick={() => s.setActive(id)}>
            <span>{title}{d.isModified ? " \u2022" : ""}</span>
            <button aria-label={`Close ${title}`} onClick={(e) => { e.stopPropagation(); s.setActive(id); void closeActive(); }}>x</button>
          </div>
        );
      })}
    </div>
  );
}
```

`app/src/panels/LayersList.tsx`:
```tsx
import { useEffect, useState } from "react";
import { useEditor } from "../state/store";
import type { LayerState } from "../engine/types";

function depth(layer: LayerState, all: LayerState[]): number {
  let d = 0; let p = layer.parentId;
  while (p && d < 64) { d++; p = all.find((l) => l.id === p)?.parentId ?? null; }
  return d;
}

export function LayersList() {
  const s = useEditor();
  const doc = s.activeId ? s.documents[s.activeId] : null;
  const [renaming, setRenaming] = useState<{ id: string; name: string } | null>(null);
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if ((e.key === "Delete" || e.key === "Backspace") && doc?.activeLayerId && !(e.target instanceof HTMLInputElement) && !s.sheet) s.run({ type: "DeleteLayer", id: doc.activeLayerId });
    };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [doc, s]);
  if (!doc) return <div className="layers" />;
  return (
    <div className="layers">
      <div className="layers-header"><span>Layers</span><button data-testid="layer-add" onClick={() => s.run({ type: "AddBlankLayer" })}>+</button></div>
      {[...doc.layers].reverse().map((l) => (
        <div key={l.id} data-testid="layer-row" className={"layer-row" + (l.id === doc.activeLayerId ? " active" : "")}
          style={{ paddingLeft: 8 + depth(l, doc.layers) * 14 }}
          onClick={() => s.run({ type: "SetActiveLayer", id: l.id })}
          onDoubleClick={() => setRenaming({ id: l.id, name: l.name })}>
          <input type="checkbox" aria-label={`Visible ${l.name}`} checked={l.visible} onClick={(e) => e.stopPropagation()} onChange={(e) => s.run({ type: "SetLayerVisible", id: l.id, visible: e.target.checked })} />
          {renaming?.id === l.id ? (
            <input autoFocus value={renaming.name} onChange={(e) => setRenaming({ id: l.id, name: e.target.value })}
              onBlur={() => { if (renaming.name.trim()) s.run({ type: "RenameLayer", id: l.id, name: renaming.name }); setRenaming(null); }}
              onKeyDown={(e) => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); if (e.key === "Escape") setRenaming(null); }} />
          ) : <span>{l.isGroup ? "\u25B8 " : ""}{l.name}</span>}
        </div>
      ))}
    </div>
  );
}
```

`app/src/panels/ToolRail.tsx`:
```tsx
import { useEditor, type Tool } from "../state/store";
const TOOLS: { id: Tool; label: string; key: string }[] = [
  { id: "move", label: "Move", key: "V" }, { id: "hand", label: "Hand", key: "H" }, { id: "zoom", label: "Zoom", key: "Z" }, { id: "crop", label: "Crop", key: "C" },
];
export function ToolRail() {
  const tool = useEditor((s) => s.tool); const setTool = useEditor((s) => s.setTool);
  return (
    <div className="tool-rail">
      {TOOLS.map((t) => <button key={t.id} data-testid={`tool-${t.id}`} title={`${t.label} (${t.key})`} className={tool === t.id ? "active" : ""} onClick={() => setTool(t.id)}>{t.label[0]}</button>)}
    </div>
  );
}
```

`app/src/App.tsx`:
```tsx
import { useEffect, useState } from "react";
import { EngineClient } from "./engine/client";
import { BUILD_MARKER } from "./build-info";
import { installTestApi } from "./test-api";
import { useEditor } from "./state/store";
import { getBridge } from "./shell/bridge";
import { CanvasView } from "./canvas/CanvasView";
import { MenuBar } from "./panels/MenuBar";
import { ProjectTabs } from "./panels/ProjectTabs";
import { LayersList } from "./panels/LayersList";
import { ToolRail } from "./panels/ToolRail";
import { NewCanvasSheet } from "./sheets/NewCanvasSheet";
import "./styles.css";

export function App() {
  const [ready, setReady] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const sheet = useEditor((s) => s.sheet);
  const banner = useEditor((s) => s.error);
  useEffect(() => {
    Promise.all([EngineClient.load(), getBridge()]).then(([engine, bridge]) => {
      useEditor.getState().setEngine(engine); useEditor.getState().setBridge(bridge);
      installTestApi({ engine, bridge, store: useEditor });
      setReady(true);
    }).catch((e) => setError(String(e)));
  }, []);
  if (error) return <div data-testid="engine-error">Engine failed to load: {error}</div>;
  if (!ready) return <div data-testid="engine-loading">Loading engine...</div>;
  return (
    <div className="app">
      <MenuBar />
      <ProjectTabs />
      <div className="workspace">
        <ToolRail />
        <CanvasView />
        <LayersList />
      </div>
      <div className="status" data-testid="engine-ready">Compositor engine {useEditor.getState().engine!.version()} ({BUILD_MARKER})</div>
      {banner && <div data-testid="error-banner" className="error-banner">{banner}<button onClick={() => useEditor.getState().setError(null)}>Dismiss</button></div>}
      {sheet?.kind === "new" && <NewCanvasSheet />}
    </div>
  );
}
```

`app/src/styles.css`: a dark flex layout: `.app` column filling the viewport; `.workspace` row; `.tool-rail` 44 px wide; `.layers` 260 px wide with `.layer-row.active` highlighted; `.menubar` with absolutely positioned `.menu-items`; `.sheet-backdrop` fixed full-screen with a centered `.sheet`; `.error-banner` fixed at the bottom. Keep it under 120 lines; no external CSS.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `pnpm build && pnpm e2e`
Expected: all e2e tests pass (smoke, render x2, files x3).

- [ ] **Step 6: Commit**

```
git add app
git commit -m "feat(app): file workflow, menus, tabs, layers list and new canvas sheet"
```

---

### Task 15: Crop tool, Canvas Size, Image Size, JPEG export

**Files:**
- Create: `app/src/tools/crop-tool.ts`, `app/src/panels/CropOptions.tsx`, `app/src/sheets/CanvasSizeSheet.tsx`, `app/src/sheets/ImageSizeSheet.tsx`, `app/src/sheets/JpegExportSheet.tsx`
- Modify: `app/src/canvas/CanvasView.tsx` (crop pointer handling and guides), `app/src/App.tsx` (mount sheets and crop options)
- Test: `app/tests/unit/crop-tool.test.ts`, `app/tests/e2e/edit.spec.ts`

**Interfaces:**
- `crop-tool.ts`: `hitTest(rect, point, viewport, docSize): CropDragMode | null` (handle within 6 CSS px wins over inside = move, outside = create); `snapTargets(state: DocumentState): { xs: number[]; ys: number[] }` (canvas edges and every visible pixel layer's upright bounds, rounded); `ratioValue(choice: CropRatio, state): number | null`; `applyRatio(rect, ratio): Rect` (keeps width and the center, port of `changeCropRatio`); `CropSession` holding the drag and computing `update(point, shiftKey, altKey)` with snapping using tolerance `10 / viewport.pointsPerPixel` document pixels, returning `{ rect, guides }`.
- `CropOptions` header (visible with the crop tool): ratio select (None, Original, 1:1, 4:3, 16:9), Cancel, Apply (`Crop` command with the current rect, then `cropRect = null`).
- `CanvasSizeSheet`: uses `CanvasSizeDraft`; width and height inputs, unit select, Relative and Constrain checkboxes, 3x3 anchor buttons, "Fill extension with color" checkbox plus color input (default white); Apply runs `CanvasSize`.
- `ImageSizeSheet`: width, height, resolution (default document resolution), lock ratio (default on), resampling select (Nearest, Smooth, High quality); Apply runs `ImageSize`.
- `JpegExportSheet`: quality slider 0 to 100 (default 85), matte color (default white), live preview `<img>` from `engine.exportJpeg` debounced 150 ms showing byte size; Export picks a file and writes.

- [ ] **Step 1: Write the failing tests**

`app/tests/unit/crop-tool.test.ts`:
```ts
import { describe, expect, it } from "vitest";
import { applyRatio, hitTest, snapTargets } from "../../src/tools/crop-tool";
import { Viewport } from "../../src/canvas/viewport";
import type { DocumentState } from "../../src/engine/types";

const doc: DocumentState = {
  id: "D", width: 400, height: 300, resolution: 72, activeLayerId: null, canUndo: false, canRedo: false, isModified: false, path: null,
  layers: [{ id: "L", name: "Red", visible: true, isGroup: false, parentId: null, opacity: 1, blendMode: "Normal",
    transform: { origin: [150, 120], size: [100, 60], rotation: 0, flipX: false, flipY: false, sampling: "High quality" }, pixelsWidth: 100, pixelsHeight: 60, pixelsRevision: 1, hasMask: false }],
};

describe("crop tool", () => {
  it("snap targets are the canvas and layer bounds", () => {
    const t = snapTargets(doc);
    expect(new Set(t.xs)).toEqual(new Set([0, 400, 150, 250]));
    expect(new Set(t.ys)).toEqual(new Set([0, 300, 120, 180]));
  });
  it("hit tests handles, inside and outside", () => {
    const vp = new Viewport(); vp.resize({ width: 800, height: 600 }, 1, { width: 400, height: 300 });
    const rect = { x: 100, y: 100, width: 200, height: 100 };
    const size = { width: 400, height: 300 };
    const corner = vp.viewPoint({ x: 300, y: 200 }, size);
    expect(hitTest(rect, corner, vp, size)).toEqual({ kind: "resize", index: 4 });
    expect(hitTest(rect, vp.viewPoint({ x: 200, y: 150 }, size), vp, size)).toEqual({ kind: "move" });
    expect(hitTest(rect, vp.viewPoint({ x: 10, y: 10 }, size), vp, size)).toEqual({ kind: "create" });
  });
  it("ratio keeps width and center", () => {
    expect(applyRatio({ x: 0, y: 0, width: 200, height: 50 }, 2)).toEqual({ x: 0, y: -25, width: 200, height: 100 });
  });
});
```

`app/tests/e2e/edit.spec.ts`:
```ts
import { test, expect } from "@playwright/test";

test("crop via the options bar, canvas size and image size sheets", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  await page.getByTestId("menu-new").click();
  await page.getByLabel("Width").fill("400");
  await page.getByLabel("Height").fill("300");
  await page.getByRole("button", { name: "Create" }).click();
  const state = () => page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId]; });

  await page.getByTestId("tool-crop").click();
  await page.evaluate(() => (window as any).__compositor.store.getState().setCropRect({ x: 50, y: 40, width: 200, height: 100 }));
  await page.getByTestId("crop-apply").click();
  let d = await state();
  expect([d.width, d.height]).toEqual([200, 100]);
  expect(d.layers[0].transform.origin).toEqual([-50, -40]);

  await page.getByTestId("menu-canvas-size").click();
  await page.getByLabel("Width").fill("300");
  await page.getByTestId("anchor-0").click();
  await page.getByRole("button", { name: "Apply" }).click();
  d = await state();
  expect([d.width, d.height]).toEqual([300, 100]);
  expect(d.layers[0].transform.origin).toEqual([-50, -40]);

  await page.getByTestId("menu-image-size").click();
  await page.getByLabel("Width").fill("600");
  await page.getByLabel("Resolution").fill("300");
  await page.getByRole("button", { name: "Apply" }).click();
  d = await state();
  expect([d.width, d.height, d.resolution]).toEqual([600, 200, 300]);

  await page.getByTestId("menu-flip-h").click();
  d = await state();
  expect(d.layers[0].transform.flipX).toBe(true);
  await page.getByTestId("menu-undo").click();
  d = await state();
  expect(d.layers[0].transform.flipX).toBe(false);
});

test("jpeg export sheet previews and writes", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  await page.getByTestId("menu-new").click();
  await page.getByRole("button", { name: "Create" }).click();
  await page.getByTestId("menu-export-jpeg").click();
  await expect(page.getByTestId("jpeg-size")).not.toHaveText("");
  await page.evaluate(() => (window as any).__compositor.bridge.setNextPick("C:/out/Untitled.jpg"));
  await page.getByRole("button", { name: "Export" }).click();
  const head = await page.evaluate(() => Array.from(((window as any).__compositor.bridge.fileBytes("C:/out/Untitled.jpg") as Uint8Array).subarray(0, 3)));
  expect(head).toEqual([0xff, 0xd8, 0xff]);
});
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `pnpm test` and `pnpm e2e --grep "crop via"`
Expected: module not found; `tool-crop` found but `crop-apply` missing.

- [ ] **Step 3: Implement the crop tool logic**

`app/src/tools/crop-tool.ts`:
```ts
import type { DocumentState } from "../engine/types";
import type { Viewport, SizeLike, PointLike } from "../canvas/viewport";
import { cropDrag, CropSnap, HANDLES, isValid, snapped, type CropDrag, type CropDragMode, type Rect } from "./crop-geometry";
import type { CropRatio } from "../state/store";

export const HANDLE_HIT_PX = 6;
export const SNAP_SCREEN_PX = 10;

export function hitTest(rect: Rect, view: PointLike, viewport: Viewport, size: SizeLike): CropDragMode {
  const tl = viewport.viewPoint({ x: rect.x, y: rect.y }, size), br = viewport.viewPoint({ x: rect.x + rect.width, y: rect.y + rect.height }, size);
  for (let index = 0; index < HANDLES.length; index++) {
    const h = HANDLES[index]; const hx = tl.x + (br.x - tl.x) * h.x, hy = tl.y + (br.y - tl.y) * h.y;
    if (Math.abs(view.x - hx) <= HANDLE_HIT_PX && Math.abs(view.y - hy) <= HANDLE_HIT_PX) return { kind: "resize", index };
  }
  if (view.x >= tl.x && view.x <= br.x && view.y >= tl.y && view.y <= br.y) return { kind: "move" };
  return { kind: "create" };
}

/** Canvas edges and every visible pixel layer's upright bounds, in whole document pixels. */
export function snapTargets(state: DocumentState): { xs: number[]; ys: number[] } {
  const xs = [0, state.width], ys = [0, state.height];
  const byId = new Map(state.layers.map((l) => [l.id, l]));
  for (const layer of state.layers) {
    if (layer.isGroup || layer.pixelsWidth === 0) continue;
    let node: typeof layer | undefined = layer; let visible = true; let steps = 0;
    while (node && steps++ < 65) { if (!node.visible) { visible = false; break; } node = node.parentId ? byId.get(node.parentId) : undefined; }
    if (!visible) continue;
    const [ox, oy] = layer.transform.origin, [w, h] = layer.transform.size;
    const cx = ox + w / 2, cy = oy + h / 2, rad = (layer.transform.rotation % 360) * Math.PI / 180;
    const cos = Math.cos(rad), sin = Math.sin(rad);
    const corners = [[-w / 2, -h / 2], [w / 2, -h / 2], [w / 2, h / 2], [-w / 2, h / 2]].map(([x, y]) => ({ x: cx + x * cos - y * sin, y: cy + x * sin + y * cos }));
    xs.push(Math.round(Math.min(...corners.map((c) => c.x))), Math.round(Math.max(...corners.map((c) => c.x))));
    ys.push(Math.round(Math.min(...corners.map((c) => c.y))), Math.round(Math.max(...corners.map((c) => c.y))));
  }
  return { xs, ys };
}

export function ratioValue(choice: CropRatio, state: DocumentState): number | null {
  switch (choice) { case "Original": return state.width / state.height; case "1:1": return 1; case "4:3": return 4 / 3; case "16:9": return 16 / 9; default: return null; }
}

/** Keeps the width and the center, like changeCropRatio on macOS. */
export function applyRatio(rect: Rect, ratio: number): Rect {
  const height = rect.width / ratio;
  const next = snapped({ x: rect.x, y: rect.y + rect.height / 2 - height / 2, width: rect.width, height });
  return isValid(next) ? next : rect;
}

export class CropSession {
  private drag: CropDrag;
  private snap: CropSnap;
  constructor(mode: CropDragMode, startDoc: PointLike, original: Rect, state: DocumentState, private readonly ratio: number | null, tolerance: number) {
    this.drag = cropDrag(mode, startDoc, original);
    const t = snapTargets(state);
    this.snap = new CropSnap(t.xs, t.ys, tolerance);
  }
  update(pointDoc: PointLike, symmetric: boolean): { rect: Rect; guides: { xs: number[]; ys: number[] } } {
    const raw = this.drag.updated(pointDoc, this.ratio, symmetric);
    const rect = this.snap.apply(raw, this.drag, pointDoc, this.ratio, symmetric);
    const guides = { xs: [] as number[], ys: [] as number[] };
    if (rect.x !== raw.x) guides.xs.push(rect.x);
    if (rect.x + rect.width !== raw.x + raw.width) guides.xs.push(rect.x + rect.width);
    if (rect.y !== raw.y) guides.ys.push(rect.y);
    if (rect.y + rect.height !== raw.y + raw.height) guides.ys.push(rect.y + rect.height);
    return { rect: isValid(rect) ? rect : raw, guides };
  }
}
```

In `CanvasView.tsx` add a pointer handler active when `tool === "crop"`: on `pointerdown` (left button, no space) compute the document point, `hitTest` against the visible crop rect (`cropRect ?? whole canvas`), create a `CropSession` with `tolerance = SNAP_SCREEN_PX / viewport.pointsPerPixel` and `ratioValue(cropRatio, state)`; on `pointermove` call `update(docPoint, e.altKey)`, store `guides` in `guidesRef` and `setCropRect(rect)`; on `pointerup` clear the guides. Shift is passed as `symmetric = e.altKey` only (Shift constrains via the ratio already). Enter applies (`Crop` command), Escape cancels (`setCropRect(null)`), both only while the crop tool is active and no sheet is open.

- [ ] **Step 4: Implement the crop options and sheets**

`app/src/panels/CropOptions.tsx`:
```tsx
import { useEditor, type CropRatio } from "../state/store";
import { applyRatio, ratioValue } from "../tools/crop-tool";

const RATIOS: CropRatio[] = ["None", "Original", "1:1", "4:3", "16:9"];

export function CropOptions() {
  const s = useEditor();
  const doc = s.activeId ? s.documents[s.activeId] : null;
  if (s.tool !== "crop" || !doc) return null;
  const rect = s.cropRect ?? { x: 0, y: 0, width: doc.width, height: doc.height };
  const apply = () => { s.run({ type: "Crop", ...rect }); s.setCropRect(null); };
  return (
    <div className="tool-options" data-testid="crop-options">
      <label>Ratio <select value={s.cropRatio} onChange={(e) => { const choice = e.target.value as CropRatio; s.setCropRatio(choice); const r = ratioValue(choice, doc); if (r !== null) s.setCropRect(applyRatio(rect, r)); }}>
        {RATIOS.map((r) => <option key={r} value={r}>{r}</option>)}
      </select></label>
      <span>{Math.round(rect.width)} x {Math.round(rect.height)}</span>
      <button data-testid="crop-cancel" onClick={() => s.setCropRect(null)}>Cancel</button>
      <button data-testid="crop-apply" className="primary" onClick={apply}>Apply</button>
    </div>
  );
}
```

`app/src/sheets/CanvasSizeSheet.tsx`:
```tsx
import { useMemo, useState } from "react";
import { Sheet } from "./Sheet";
import { useEditor } from "../state/store";
import { CANVAS_UNITS, CanvasSizeDraft, type CanvasUnit } from "../tools/canvas-size-draft";

export function CanvasSizeSheet() {
  const s = useEditor();
  const doc = s.documents[s.activeId!];
  const draft = useMemo(() => new CanvasSizeDraft(doc.width, doc.height, doc.resolution), [doc]);
  const [, bump] = useState(0);
  const [anchor, setAnchor] = useState(4);
  const [fill, setFill] = useState(false);
  const [color, setColor] = useState("#ffffff");
  const rerender = () => bump((n) => n + 1);
  const field = (widthAxis: boolean) => (
    <input aria-label={widthAxis ? "Width" : "Height"} type="number" value={String(Math.round(draft.displayed(widthAxis) * 1000) / 1000)}
      onChange={(e) => { const v = Number(e.target.value); if (Number.isFinite(v)) { draft.set(v, widthAxis); rerender(); } }} />
  );
  const apply = () => {
    const rgb = fill ? ([1, 3, 5].map((i) => parseInt(color.slice(i, i + 2), 16) / 255) as [number, number, number]) : null;
    s.run({ type: "CanvasSize", width: Math.round(draft.width), height: Math.round(draft.height), anchor, fill: rgb });
    s.closeSheet();
  };
  return (
    <Sheet title="Canvas Size" primary="Apply" canConfirm={draft.valid} onCancel={s.closeSheet} onConfirm={apply}>
      <div>Current: {doc.width} x {doc.height} px</div>
      <label>Width {field(true)}</label>
      <label>Height {field(false)}</label>
      <label>Unit <select value={draft.unit} onChange={(e) => { draft.unit = e.target.value as CanvasUnit; rerender(); }}>{CANVAS_UNITS.map((u) => <option key={u}>{u}</option>)}</select></label>
      <label><input type="checkbox" checked={draft.relative} onChange={(e) => { draft.relative = e.target.checked; rerender(); }} /> Relative</label>
      <label><input type="checkbox" checked={draft.locked} onChange={(e) => { draft.locked = e.target.checked; rerender(); }} /> Constrain proportions</label>
      <div className="anchor-grid">{Array.from({ length: 9 }, (_, i) => <button key={i} data-testid={`anchor-${i}`} className={anchor === i ? "active" : ""} onClick={() => setAnchor(i)}>{i === anchor ? "\u25CF" : "\u00B7"}</button>)}</div>
      <label><input type="checkbox" checked={fill} onChange={(e) => setFill(e.target.checked)} /> Fill extension with color <input type="color" value={color} onChange={(e) => setColor(e.target.value)} disabled={!fill} /></label>
    </Sheet>
  );
}
```

`app/src/sheets/ImageSizeSheet.tsx`:
```tsx
import { useState } from "react";
import { Sheet } from "./Sheet";
import { useEditor } from "../state/store";
import type { Sampling } from "../engine/types";

export function ImageSizeSheet() {
  const s = useEditor();
  const doc = s.documents[s.activeId!];
  const [width, setWidth] = useState(doc.width);
  const [height, setHeight] = useState(doc.height);
  const [resolution, setResolution] = useState(doc.resolution);
  const [locked, setLocked] = useState(true);
  const [sampling, setSampling] = useState<Sampling>("High quality");
  const ok = (n: number) => Number.isFinite(n) && Math.round(n) >= 1 && Math.round(n) <= 30_000;
  const valid = ok(width) && ok(height) && Number.isFinite(resolution) && resolution >= 1 && resolution <= 9600;
  return (
    <Sheet title="Image Size" primary="Apply" canConfirm={valid} onCancel={s.closeSheet}
      onConfirm={() => { s.run({ type: "ImageSize", width: Math.round(width), height: Math.round(height), resolution, sampling }); s.closeSheet(); }}>
      <label>Width <input aria-label="Width" type="number" value={width} onChange={(e) => { const v = Number(e.target.value); setWidth(v); if (locked) setHeight(Math.round(v * doc.height / doc.width)); }} /> px</label>
      <label>Height <input aria-label="Height" type="number" value={height} onChange={(e) => { const v = Number(e.target.value); setHeight(v); if (locked) setWidth(Math.round(v * doc.width / doc.height)); }} /> px</label>
      <label>Resolution <input aria-label="Resolution" type="number" value={resolution} onChange={(e) => setResolution(Number(e.target.value))} /> pixels/inch</label>
      <label><input type="checkbox" checked={locked} onChange={(e) => setLocked(e.target.checked)} /> Constrain proportions</label>
      <label>Resample <select value={sampling} onChange={(e) => setSampling(e.target.value as Sampling)}><option>Nearest</option><option>Smooth</option><option>High quality</option></select></label>
    </Sheet>
  );
}
```

`app/src/sheets/JpegExportSheet.tsx`:
```tsx
import { useEffect, useState } from "react";
import { Sheet } from "./Sheet";
import { useEditor } from "../state/store";

export function JpegExportSheet() {
  const s = useEditor();
  const doc = s.documents[s.activeId!];
  const [quality, setQuality] = useState(85);
  const [color, setColor] = useState("#ffffff");
  const [preview, setPreview] = useState<{ url: string; bytes: Uint8Array } | null>(null);
  const matte = (): [number, number, number] => [1, 3, 5].map((i) => parseInt(color.slice(i, i + 2), 16) / 255) as [number, number, number];
  useEffect(() => {
    const t = setTimeout(() => {
      try {
        const bytes = s.engine!.exportJpeg(doc.id, quality / 100, matte());
        setPreview((old) => { if (old) URL.revokeObjectURL(old.url); return { url: URL.createObjectURL(new Blob([bytes], { type: "image/jpeg" })), bytes }; });
      } catch (e) { s.setError(e instanceof Error ? e.message : String(e)); }
    }, 150);
    return () => clearTimeout(t);
  }, [quality, color, doc.id]);
  const exportNow = async () => {
    if (!preview) return;
    const path = await s.bridge!.pickExportFile(doc.path ? s.bridge!.baseName(doc.path) : "Untitled", "jpg");
    if (!path) return;
    try { await s.bridge!.writeFile(path, preview.bytes); s.closeSheet(); } catch (e) { s.setError(e instanceof Error ? e.message : String(e)); }
  };
  return (
    <Sheet title="Export JPEG" primary="Export" canConfirm={preview !== null} onCancel={s.closeSheet} onConfirm={() => void exportNow()}>
      <label>Quality <input type="range" min={0} max={100} value={quality} onChange={(e) => setQuality(Number(e.target.value))} /> {quality}</label>
      <label>Matte <input type="color" value={color} onChange={(e) => setColor(e.target.value)} /></label>
      {preview && <img alt="Preview" src={preview.url} style={{ maxWidth: 480, maxHeight: 320, objectFit: "contain" }} />}
      <div data-testid="jpeg-size">{preview ? `${(preview.bytes.length / 1024).toFixed(1)} KB` : ""}</div>
    </Sheet>
  );
}
```

Mount in `App.tsx`: `<CropOptions />` above the workspace; `sheet?.kind === "canvasSize" && <CanvasSizeSheet />`, likewise `imageSize` and `jpeg`.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `pnpm test && pnpm build && pnpm e2e`
Expected: all unit and e2e tests pass.

- [ ] **Step 6: Commit**

```
git add app
git commit -m "feat(app): crop tool with snapping, canvas size, image size and JPEG export sheets"
```

---

### Task 16: Keyboard shortcuts and drag-and-drop import

**Files:**
- Create: `app/src/shortcuts/keymap.ts`, `app/src/shortcuts/useShortcuts.ts`
- Modify: `app/src/App.tsx` (install shortcuts and the drop handler), `app/src/canvas/CanvasView.tsx` (zoom tool clicks)
- Test: `app/tests/unit/keymap.test.ts`, `app/tests/e2e/drop.spec.ts`

**Interfaces:**
- `keymap.ts`: `type Shortcut = { key: string; ctrl?: boolean; shift?: boolean; alt?: boolean }`, `SHORTCUTS: Record<ActionId, Shortcut>` and `matchShortcut(e: KeyboardEvent): ActionId | null`. Photoshop-style on Windows: `Ctrl+N` new, `Ctrl+O` open, `Ctrl+S` save, `Ctrl+Shift+S` save as, `Ctrl+Shift+Alt+S` export JPEG, `Ctrl+Shift+E` export PNG, `Ctrl+W` close, `Ctrl+Z` undo, `Ctrl+Shift+Z` and `Ctrl+Y` redo, `Ctrl+Shift+N` new layer, `Ctrl+Alt+C` canvas size, `Ctrl+Alt+I` image size, `Ctrl+=` and `Ctrl++` zoom in, `Ctrl+-` zoom out, `Ctrl+0` fit, `Ctrl+1` actual size, `V` move, `H` hand, `Z` zoom, `C` crop, `Enter` apply crop, `Escape` cancel crop.
- `useShortcuts()` hook: ignores events when the target is an input, textarea or a sheet is open (except Enter and Escape handled by the sheet), and dispatches to the same actions the menu uses.
- Drop: `bridge.onFileDrop` paths ending in `.comp` open as projects; other paths import as images at the drop position converted to document coordinates (`viewport.documentPoint`), or create a new document when none is open.
- Zoom tool: click zooms in by 1.5 at the pointer; Alt-click zooms out.

- [ ] **Step 1: Write the failing tests**

`app/tests/unit/keymap.test.ts`:
```ts
import { describe, expect, it } from "vitest";
import { matchShortcut } from "../../src/shortcuts/keymap";

function ev(key: string, mods: Partial<{ ctrlKey: boolean; shiftKey: boolean; altKey: boolean }> = {}): KeyboardEvent {
  return { key, ctrlKey: false, shiftKey: false, altKey: false, metaKey: false, ...mods } as KeyboardEvent;
}

describe("keymap", () => {
  it("maps Photoshop-style shortcuts", () => {
    expect(matchShortcut(ev("s", { ctrlKey: true }))).toBe("save");
    expect(matchShortcut(ev("S", { ctrlKey: true, shiftKey: true }))).toBe("save-as");
    expect(matchShortcut(ev("z", { ctrlKey: true }))).toBe("undo");
    expect(matchShortcut(ev("Z", { ctrlKey: true, shiftKey: true }))).toBe("redo");
    expect(matchShortcut(ev("y", { ctrlKey: true }))).toBe("redo");
    expect(matchShortcut(ev("c"))).toBe("tool-crop");
    expect(matchShortcut(ev("c", { ctrlKey: true, altKey: true }))).toBe("canvas-size");
    expect(matchShortcut(ev("0", { ctrlKey: true }))).toBe("fit");
    expect(matchShortcut(ev("=", { ctrlKey: true }))).toBe("zoom-in");
    expect(matchShortcut(ev("x"))).toBeNull();
  });
});
```

`app/tests/e2e/drop.spec.ts`:
```ts
import { test, expect } from "@playwright/test";

const RED_PNG = "iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAYAAABytg0kAAAAEUlEQVQIW2P8z8DwnwEIGGEMAD1sBAOiD07mAAAAAElFTkSuQmCC";

test("dropping an image with no document creates one; dropping onto a document adds a layer", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  await page.evaluate((b64) => {
    const api = (window as any).__compositor;
    api.bridge.seedFile("C:/pics/red.png", Uint8Array.from(atob(b64), (c: string) => c.charCodeAt(0)));
    api.bridge.simulateDrop(["C:/pics/red.png"], null);
  }, RED_PNG);
  await expect(page.getByTestId("project-tab")).toHaveCount(1);
  await expect(page.getByTestId("layer-row")).toHaveCount(1);
  await page.evaluate(() => (window as any).__compositor.bridge.simulateDrop(["C:/pics/red.png"], { x: 10, y: 10 }));
  await expect(page.getByTestId("layer-row")).toHaveCount(2);
});

test("keyboard shortcuts drive tools and undo", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  await page.keyboard.press("Control+n");
  await page.getByRole("button", { name: "Create" }).click();
  await page.keyboard.press("c");
  await expect(page.getByTestId("crop-options")).toBeVisible();
  await page.keyboard.press("v");
  await expect(page.getByTestId("crop-options")).toHaveCount(0);
  await page.keyboard.press("Control+Shift+n");
  await expect(page.getByTestId("layer-row")).toHaveCount(2);
  await page.keyboard.press("Control+z");
  await expect(page.getByTestId("layer-row")).toHaveCount(1);
});
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `pnpm test` and `pnpm e2e --grep "dropping"`
Expected: module not found; drop handler never wired so no tab appears.

- [ ] **Step 3: Implement the keymap and hook**

`app/src/shortcuts/keymap.ts`:
```ts
export type ActionId = "new" | "open" | "save" | "save-as" | "export-png" | "export-jpeg" | "close" | "undo" | "redo" | "new-layer"
  | "canvas-size" | "image-size" | "zoom-in" | "zoom-out" | "fit" | "actual" | "tool-move" | "tool-hand" | "tool-zoom" | "tool-crop" | "crop-apply" | "crop-cancel";

export interface Shortcut { key: string; ctrl?: boolean; shift?: boolean; alt?: boolean; }

export const SHORTCUTS: Record<ActionId, Shortcut[]> = {
  "new": [{ key: "n", ctrl: true }], "open": [{ key: "o", ctrl: true }], "save": [{ key: "s", ctrl: true }],
  "save-as": [{ key: "s", ctrl: true, shift: true }], "export-jpeg": [{ key: "s", ctrl: true, shift: true, alt: true }],
  "export-png": [{ key: "e", ctrl: true, shift: true }], "close": [{ key: "w", ctrl: true }],
  "undo": [{ key: "z", ctrl: true }], "redo": [{ key: "z", ctrl: true, shift: true }, { key: "y", ctrl: true }],
  "new-layer": [{ key: "n", ctrl: true, shift: true }], "canvas-size": [{ key: "c", ctrl: true, alt: true }], "image-size": [{ key: "i", ctrl: true, alt: true }],
  "zoom-in": [{ key: "=", ctrl: true }, { key: "+", ctrl: true }], "zoom-out": [{ key: "-", ctrl: true }], "fit": [{ key: "0", ctrl: true }], "actual": [{ key: "1", ctrl: true }],
  "tool-move": [{ key: "v" }], "tool-hand": [{ key: "h" }], "tool-zoom": [{ key: "z" }], "tool-crop": [{ key: "c" }],
  "crop-apply": [{ key: "Enter" }], "crop-cancel": [{ key: "Escape" }],
};

export function matchShortcut(e: KeyboardEvent): ActionId | null {
  const key = e.key.length === 1 ? e.key.toLowerCase() : e.key;
  const ctrl = e.ctrlKey || e.metaKey;
  for (const [id, list] of Object.entries(SHORTCUTS) as [ActionId, Shortcut[]][]) {
    for (const s of list) {
      if (s.key.toLowerCase() === key && !!s.ctrl === ctrl && !!s.shift === e.shiftKey && !!s.alt === e.altKey) return id;
    }
  }
  return null;
}
```

`app/src/shortcuts/useShortcuts.ts`:
```ts
import { useEffect } from "react";
import { matchShortcut, type ActionId } from "./keymap";
import { useEditor } from "../state/store";
import { closeActive, exportPng, openProject, saveProject, saveProjectAs } from "../actions/files";

export function runAction(id: ActionId): void {
  const s = useEditor.getState();
  const doc = s.activeId ? s.documents[s.activeId] : null;
  const vp = s.activeId ? s.viewports[s.activeId] : null;
  const zoomBy = (f: number) => { if (doc && vp) { vp.setZoom(vp.zoom * f, vp.center, { width: doc.width, height: doc.height }); s.invalidate(); } };
  switch (id) {
    case "new": s.openSheet({ kind: "new" }); break;
    case "open": void openProject(); break;
    case "save": void saveProject(); break;
    case "save-as": void saveProjectAs(); break;
    case "export-png": void exportPng(); break;
    case "export-jpeg": if (doc) s.openSheet({ kind: "jpeg" }); break;
    case "close": void closeActive(); break;
    case "undo": s.undo(); break;
    case "redo": s.redo(); break;
    case "new-layer": if (doc) s.run({ type: "AddBlankLayer" }); break;
    case "canvas-size": if (doc) s.openSheet({ kind: "canvasSize" }); break;
    case "image-size": if (doc) s.openSheet({ kind: "imageSize" }); break;
    case "zoom-in": zoomBy(1.25); break;
    case "zoom-out": zoomBy(0.8); break;
    case "fit": if (doc && vp) { vp.fit({ width: doc.width, height: doc.height }); s.invalidate(); } break;
    case "actual": if (doc && vp) { vp.setZoom(window.devicePixelRatio || 1, vp.center, { width: doc.width, height: doc.height }); s.invalidate(); } break;
    case "tool-move": s.setTool("move"); break;
    case "tool-hand": s.setTool("hand"); break;
    case "tool-zoom": s.setTool("zoom"); break;
    case "tool-crop": s.setTool("crop"); break;
    case "crop-apply": if (doc && s.tool === "crop") { const r = s.cropRect ?? { x: 0, y: 0, width: doc.width, height: doc.height }; s.run({ type: "Crop", ...r }); s.setCropRect(null); } break;
    case "crop-cancel": if (s.tool === "crop") s.setCropRect(null); break;
  }
}

export function useShortcuts(): void {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      if (target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable)) return;
      if (useEditor.getState().sheet) return;
      const id = matchShortcut(e);
      if (!id) return;
      e.preventDefault();
      runAction(id);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
}
```

Have `MenuBar` call `runAction` for every item instead of duplicating the logic, and add `data-testid="menu-undo"` etc. as before.

- [ ] **Step 4: Wire drop and the zoom tool**

In `App.tsx`, after the bridge is ready:
```ts
bridge.onFileDrop((paths, position) => {
  const projects = paths.filter((p) => p.toLowerCase().endsWith(".comp"));
  const images = paths.filter((p) => !p.toLowerCase().endsWith(".comp"));
  for (const p of projects) void openProject(p);
  if (images.length) {
    const s = useEditor.getState();
    let at: { x: number; y: number } | undefined;
    if (position && s.activeId) {
      const el = document.querySelector('[data-testid="canvas-view"]') as HTMLElement | null;
      const r = el?.getBoundingClientRect();
      if (r) { const d = s.documents[s.activeId]; at = s.viewports[s.activeId].documentPoint({ x: position.x - r.left, y: position.y - r.top }, { width: d.width, height: d.height }); }
    }
    void importImages(images, at);
  }
});
```
Tauri reports drop positions in physical pixels relative to the window; divide by `window.devicePixelRatio` before subtracting the canvas rect. The mock passes CSS pixels; add a `positionIsPhysical` boolean to the bridge interface (true for Tauri, false for the mock) and scale accordingly.

In `CanvasView.tsx`, when `tool === "zoom"`, a click without drag calls `vp.setZoom(vp.zoom * (e.altKey ? 1 / 1.5 : 1.5), viewPoint, size)`.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `pnpm test && pnpm build && pnpm e2e`
Expected: all pass.

- [ ] **Step 6: Commit**

```
git add app
git commit -m "feat(app): keyboard shortcuts, zoom tool and drag-and-drop import"
```

---

### Task 17: Portable build, marker verification and smoke launch

**Files:**
- Create: `scripts/build-windows-x64.ps1`, `README.md`
- Modify: `package.json` (script `build:portable`)

**Interfaces:**
- `scripts/build-windows-x64.ps1 [-SkipWasm]`: imports the MSVC environment via `vswhere` (same routine as superai-agent's script), writes `app/src/build-info.ts` with a fresh marker, runs `pnpm wasm`, `pnpm build`, `pnpm tauri build --no-bundle --ci`, asserts the marker string appears in exactly one file under `app/dist/assets`, copies `src-tauri/target/release/compositor-shell.exe` to `build-artifacts/windows-x64/Compositor-portable-<version>-<stamp>/Compositor.exe` with a `README.txt`, zips it to `build-artifacts/windows-x64/Compositor-portable-<version>-<stamp>.zip`, prints the zip path and size, and launches the exe for 5 seconds to confirm it starts (exit code 0 after `Stop-Process`).

- [ ] **Step 1: Write the script**

`scripts/build-windows-x64.ps1`:
```powershell
[CmdletBinding()]
param([switch]$SkipWasm, [switch]$NoSmoke)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
Set-Location $root
function Step([string]$m) { Write-Host "[build-windows-x64] $m" }

# MSVC environment
$vswhere = 'C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe'
if (-not (Test-Path $vswhere)) { throw 'vswhere.exe not found. Install Visual Studio 2022 Build Tools with the C++ workload.' }
$installPath = & $vswhere -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath | Select-Object -First 1
if (-not $installPath) { throw 'VC.Tools.x86.x64 workload not found.' }
$vsDevCmd = Join-Path $installPath 'Common7\Tools\VsDevCmd.bat'
$env:VSCMD_SKIP_SENDTELEMETRY = '1'
$envDump = & cmd.exe /d /s /c "`"$vsDevCmd`" -arch=x64 -host_arch=x64 >nul && set"
foreach ($line in $envDump) { if ($line -match '^(.*?)=(.*)$') { [Environment]::SetEnvironmentVariable($matches[1], $matches[2], 'Process') } }
$cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
if (-not (($env:Path -split ';') -contains $cargoBin)) { $env:Path = "$cargoBin;$env:Path" }

$version = (Get-Content (Join-Path $root 'package.json') -Raw | ConvertFrom-Json).version
Step "Writing build marker"
$marker = & powershell -ExecutionPolicy Bypass -File (Join-Path $root 'scripts\write-build-info.ps1') -Version $version
$marker = ($marker | Select-Object -Last 1).Trim()
Step "Marker: $marker"

if (-not $SkipWasm) { Step 'Building wasm engine'; & pnpm wasm; if ($LASTEXITCODE -ne 0) { throw 'wasm build failed' } }
Step 'Building web app'; & pnpm build; if ($LASTEXITCODE -ne 0) { throw 'web build failed' }

Step 'Verifying marker is bundled'
$hits = Get-ChildItem (Join-Path $root 'app\dist\assets') -File | Where-Object { (Get-Content $_.FullName -Raw) -match [regex]::Escape($marker) }
if (@($hits).Count -ne 1) { throw "Expected the marker in exactly one bundle file, found $(@($hits).Count)." }

Step 'Building Tauri shell'
$env:TAURI_ENV_TARGET_TRIPLE = 'x86_64-pc-windows-msvc'
& pnpm tauri build --no-bundle --ci; if ($LASTEXITCODE -ne 0) { throw 'tauri build failed' }
$exe = Join-Path $root 'src-tauri\target\release\compositor-shell.exe'
if (-not (Test-Path $exe)) { throw "Built exe not found at $exe" }

$stamp = ($marker -split '_')[-1]
$outDir = Join-Path $root 'build-artifacts\windows-x64'
$stage = Join-Path $outDir "Compositor-portable-$version-$stamp"
New-Item -ItemType Directory -Force $stage | Out-Null
Copy-Item $exe (Join-Path $stage 'Compositor.exe')
@(
  "Compositor for Windows $version ($marker)",
  'Portable build: run Compositor.exe. Requires the Microsoft Edge WebView2 Runtime (preinstalled on Windows 10 19045 and later).',
  'Projects are .comp folders compatible with Compositor for macOS.'
) | Set-Content -Path (Join-Path $stage 'README.txt') -Encoding ascii
$zip = "$stage.zip"
if (Test-Path $zip) { Remove-Item $zip -Force }
Compress-Archive -Path (Join-Path $stage '*') -DestinationPath $zip
$size = [math]::Round((Get-Item $zip).Length / 1MB, 1)
Step "Portable zip: $zip ($size MB)"

if (-not $NoSmoke) {
  Step 'Smoke launch'
  $p = Start-Process -FilePath (Join-Path $stage 'Compositor.exe') -PassThru
  Start-Sleep -Seconds 5
  if ($p.HasExited) { throw "Compositor.exe exited early with code $($p.ExitCode)" }
  Stop-Process -Id $p.Id -Force
  Step 'Smoke launch OK'
}
Write-Output $zip
```

Add `"build:portable": "powershell -ExecutionPolicy Bypass -File scripts/build-windows-x64.ps1"` to `package.json`.

- [ ] **Step 2: Write the README**

`README.md`: what the app is (one paragraph, link to the macOS project), Phase 1 feature list, prerequisites (Rust 1.95 with `wasm32-unknown-unknown`, wasm-pack, Node 22, pnpm, Visual Studio Build Tools C++ workload, WebView2 runtime), commands (`pnpm install`, `pnpm wasm:dev`, `pnpm dev` for the browser with the mock shell, `pnpm tauri:dev`, `pnpm test`, `pnpm e2e`, `cargo test`, `pnpm build:portable`), the `.comp` interoperability note, and the pointer to `docs/superpowers/specs` and `plans`.

- [ ] **Step 3: Run the full build**

Run: `pnpm build:portable`
Expected: prints the marker, the zip path and size, and "Smoke launch OK". Then unzip to a fresh folder, launch, create a canvas, import an image, save a `.comp`, and open that `.comp` folder with the macOS build if one is reachable; otherwise inspect `manifest.json` by eye against `docs/project-format.md`.

- [ ] **Step 4: Commit**

```
git add scripts package.json README.md
git commit -m "build: portable Windows zip with bundled-marker verification and smoke launch"
```

---

## Self-review notes

Spec coverage for Phase 1: new canvas (Task 14), import (9, 14, 16), open and save `.comp` (4, 13, 14), export PNG (5, 14) and JPEG with preview (5, 15), crop with snapping (7, 11, 15), Canvas Size (7, 15), Image Size with resolution (8, 15), Flip Canvas (7, 14), zoom and pan, pixel grid, sharp downsampling (12), tabs (14), CPU fallback (12), version marker and portable zip (17), shortcuts (16), drag and drop (16), preservation of later-phase fields (2, 4), limits (2, 4, 7, 8, 9).

Deferred to Phase 2 by design: blend modes, masks and clipping in rendering (the compositor draws Normal with opacity; masks are loaded, saved and resampled but not applied), layer transforms by dragging, layer reordering by drag, folders in the layers list beyond display.

Known simplifications to revisit: the engine snapshot history clones the layer list per command (rasters shared); `import_image` for a first document pushes a synthetic history entry to read as modified; the CPU renderer allocates a scratch canvas per frame.

Type consistency checked: `Point`/`Size` serde tuples; `LayerRecord::new` signature used in Tasks 2, 4, 5; `Dirty::structure()` and `Dirty::everything()`; `EngineClient` method names used by the store, actions and sheets; `CropDragMode` shape shared by `crop-geometry.ts` and `crop-tool.ts`; `DocumentState.layers[].transform.origin` is a tuple in TypeScript and a `Point` (serialized as a tuple) in Rust.

