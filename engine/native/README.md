# Retouch kernels

`HealPixels.c/.h` and `ContentFill.c/.h` come from Compositor tag v1.4.5,
commit `086f1631573ccb2b57644e53b52bf1488fc976aa`, under the included MIT
license (`LICENSE-Compositor.txt`, Wonder Assembly LLC).

The kernel bodies are unchanged. Runtime includes are conditional: native
Windows uses the C library; WASM uses `WasmCompat.h` and the allocation/math
bridges in `engine/src/retouch.rs`. `engine/build.rs` compiles both sources.

The original sources are `Compositor/Rendering/HealPixels.c` and
`Compositor/Rendering/ContentFill.c` at that tag.
