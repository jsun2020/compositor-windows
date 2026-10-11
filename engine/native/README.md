# Compositor native kernels

All upstream kernels and resources are covered by the included MIT license,
[LICENSE-Compositor.txt](LICENSE-Compositor.txt), Wonder Assembly LLC.

- HealPixels.c/.h and ContentFill.c/.h: v1.4.5, commit
  086f1631573ccb2b57644e53b52bf1488fc976aa, Compositor/Rendering.
- AdjustPixels.c/.h, DitherPixels.c/.h and CameraRawTables.bin: v1.4.9, commit
  0ab3ff58a3dcf3d52a455fe3ab0336ac95923edd, Compositor/Rendering and
  Compositor/Resources. The LUT SHA256 is
  e67ff4dbcce35cb7765752acff4405be5c934d913ef1396b2ca3fc6dc8a85190.

The C arithmetic is retained. Apple dispatch/blocks scheduling is replaced by
serial band loops; runtime includes select bounded Windows/WASM allocation and
math bridges. engine/build.rs compiles these kernels and decompresses the CRT3
resource on the build host; the resulting RGB table is embedded in WASM.

The 16-to-8-bit RAW quantizer in src-tauri/native/RawQuantize.h transcribes
Compositor/Rendering/DitherPixels.c at the same v1.4.9 commit. LibRaw still
develops the actual sensor data; this does not promise Apple CIRAWFilter parity.
