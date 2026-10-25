use sha2::{Digest,Sha256};
fn main() {
    let shadows = std::fs::read("native/CameraRawShadows.bin").expect("Camera Raw Shadows");
    assert_eq!(format!("{:x}", Sha256::digest(&shadows)), "76578539bc45a32817aff669c4e41523d7040f3c691ad35adf0723a70a89a705", "fixed v1.5.7 Shadows resource changed");
    println!("cargo:rerun-if-changed=native/CameraRawShadows.bin");
    // Decode the immutable upstream CRT3 resource on the build host. The WASM target
    // receives raw RGB tables and needs no native LZMA runtime or startup decoding.
    let file = std::fs::read("native/CameraRawTables.bin").expect("Camera Raw tables");
    assert_eq!(format!("{:x}",Sha256::digest(&file)),"e67ff4dbcce35cb7765752acff4405be5c934d913ef1396b2ca3fc6dc8a85190","fixed v1.4.9 LUT resource changed");
    assert_eq!(&file[..4], b"CRT3");
    let size = u16::from_le_bytes([file[4], file[5]]) as usize;
    let count = u16::from_le_bytes([file[6], file[7]]) as usize;
    let step = u16::from_le_bytes([file[8], file[9]]) as i32;
    assert_eq!((size, count, step), (17, 763, 3));
    let mut planes = Vec::new();
    lzma_rs::xz_decompress(&mut std::io::Cursor::new(&file[10..]), &mut planes).expect("CRT3 XZ stream");
    let cells = size * size * size;
    assert_eq!(planes.len(), count * cells * 3);
    let mut tables = vec![0u8; planes.len()];
    for table in 0..count { for channel in 0..3 {
        let mut running = 0u8;
        for cell in 0..cells {
            running = running.wrapping_add(planes[(table * 3 + channel) * cells + cell]);
            let axis = match channel { 0 => cell / (size * size), 1 => cell / size % size, _ => cell % size };
            let identity = (axis as f64 * 255.0 / (size - 1) as f64).round() as i32;
            tables[(table * cells + cell) * 3 + channel] = (identity + running as i8 as i32 * step).clamp(0, 255) as u8;
        }
    }}
    std::fs::write(std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("camera-raw-tables.rgb"), tables).unwrap();
    println!("cargo:rerun-if-changed=native/CameraRawTables.bin");
    let mut build = cc::Build::new();
    build.files(["native/HealPixels.c", "native/PatchPixels.c", "native/DeepPixels.c", "native/AdjustPixels.c", "native/LensPixels.c", "native/DitherPixels.c"])
        .include("native").define("_USE_MATH_DEFINES", None);
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
        build.compiler("clang").archiver("llvm-ar").no_default_flags(true)
            .flag("--target=wasm32-unknown-unknown").flag("-ffreestanding").flag("-O2");
    }
    build.compile("compositor_retouch");
    for file in ["native/HealPixels.c", "native/HealPixels.h", "native/PatchPixels.c", "native/DeepPixels.c", "native/PatchPixels.h", "native/DeepPixels.h", "native/WasmCompat.h", "native/AdjustPixels.c", "native/AdjustPixels.h", "native/LensPixels.c", "native/LensPixels.h", "native/CameraRawRuntime.h", "native/DitherPixels.c", "native/DitherPixels.h"] {
        println!("cargo:rerun-if-changed={file}");
    }
}
