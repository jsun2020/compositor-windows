fn main() {
    let mut build = cc::Build::new();
    build.files(["native/HealPixels.c", "native/ContentFill.c"])
        .include("native").define("_USE_MATH_DEFINES", None);
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
        build.compiler("clang").archiver("llvm-ar").no_default_flags(true)
            .flag("--target=wasm32-unknown-unknown").flag("-ffreestanding").flag("-O2");
    }
    build.compile("compositor_retouch");
    for file in ["native/HealPixels.c", "native/HealPixels.h", "native/ContentFill.c", "native/ContentFill.h", "native/WasmCompat.h"] {
        println!("cargo:rerun-if-changed={file}");
    }
}
