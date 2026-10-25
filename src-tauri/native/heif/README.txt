Compositor bundled HEIF decoder

CompositorHeif.exe uses the separately replaceable heif.dll (libheif) and
 libde265.dll (libde265). Both libraries are licensed under LGPL version 3 or later.
The complete, unmodified source archives, license texts and pinned build recipe
are included here. No Microsoft Store HEVC extension is required.

To rebuild with Visual Studio 2022 C++ Build Tools and CMake, copy this directory
to a writable folder, then run in PowerShell:
  powershell -ExecutionPolicy Bypass -File .\build-heif-helper.ps1 -OutputDirectory C:\path\to\new-decoder
The script verifies the source archives and builds shared libraries. The helper
source and CMake project can be edited and rebuilt. Replace CompositorHeif.exe,
heif.dll and libde265.dll beside Compositor.exe with compatible rebuilt versions.
Do not modify the archive hashes to conceal a source change; unpack and build a
modified library separately if you wish to exercise your LGPL modification rights.
Reverse engineering of this combined work for debugging modifications to these
LGPL libraries is permitted as required by their licenses.

The helper imports the primary HEIF image, applies crop/rotation/mirroring, and
returns straight 16-bit RGBA. 8-bit sources are expanded; 10/12-bit sources retain
their precision. NCLX is converted to sRGB; embedded RGB ICC profiles are converted
by the application using moxcms 0.8.1 at 16-bit precision. HDR gain-map reconstruction is not implemented.
Decoding is software based, not a GPU acceleration feature.
