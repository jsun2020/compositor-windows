# Mac 1.2.10 probe results

The user opened the seven Phase 3.5a probe projects in Compositor 1.2.10 on the Mac and exported each
at 100% as PNG (2026-09-24). The projects (manifest plus referenced images only) and the exports are
committed as fixtures in `engine/tests/fixtures/mac-1.2.10-probes/` (`<name>.comp/` and
`<name>.mac-1.2.10.png`). The Mac did not re-save any probe (manifest mtimes precede the exports).
The comparison was made against the port's CPU compositor at 59eb62c with a throwaway crate; the
generated tables follow the verdicts.

## Export format

All seven exports are 8-bit RGBA, straight alpha, with chunks IHDR, sRGB (intent 0), eXIf (72 dpi,
ColorSpace sRGB), pHYs, IDAT, IEND. No iCCP, gAMA or cHRM, and no value shift: several probes match the
port bit for bit. (The port's own PNG export writes pHYs only, no sRGB chunk and no eXIf.)

## Verdicts per probe

- folder-opacity (120x60): AGREE, 7200/7200 pixels identical. Overlap premultiplied (64,0,128,192):
  per-descendant multiplication confirmed; Photoshop group fade ruled out.
- clipped-in-dimmed-folder (120x60): AGREE, bit-identical, (128,64,64,128) premultiplied. The double
  dim read from LiveMaskRenderer is confirmed by a Mac render.
- guides (120x60): AGREE, bit-identical; guides are not exported.
- new-blend-modes (220x60): differs as expected (port draws Normal). Every Mac row identical; output
  alpha 255.
- new-adjustment-layers (120x60): differs as expected (port identity). The Mac equals a
  re-implementation of `adjust_black_white` (AdjustPixels.c) with the Photoshop defaults (reds 40,
  yellows 60, greens 40, cyans 60, blues 20, magentas 80, weights / 100) on all 7200 pixels; an absent
  `blackWhiteSettings` resolves to those defaults. Rec.709 luma (max error 106), Rec.601 (80) and a plain
  average (28) are ruled out.
- grain (120x60): differs as expected. The Mac equals a re-implementation of the 1.2.6/1.2.10 kernel
  (fine noise interpolated at detail size max(0.5, size * 0.35)) on all 7200 pixels, no float drift.
  Mac minus port: mean +0.03, sd about 6.2; grain sd Mac 6.70 vs port 6.22.
- edited-rich-file (160x80): the Mac OPENED a port-edited file (duplicate, group, canvas size, flip)
  and exported it. Differences come from the Gaussian Blur layer (sigma 24) and two drop shadows the
  port does not draw. Modelling the blur with the port's existing `gaussian_blur` (sigma = radius,
  transparent outside the canvas) plus the shadows as R 4.2 describes (offset (0,+20), sigma = blur/2 = 10,
  black at 0.5, under both layers, then blurred) leaves at most 1 (colour) and 2 (alpha) premultiplied.

## Blend modes

Every mode matches `result = (1 - a) * backdrop + a * B(backdrop, source)` on sRGB 8-bit values with the
source's STRAIGHT colour (max error 0; Soft Light 1). Premultiplied-source and ignore-alpha variants are
ruled out, so the Core Image modes composite alpha like the Core Graphics ones (settles R 4.4 for these
inputs).

- Linear Burn: max(0, b + s - 1).
- Linear Dodge (Add): min(1, b + s).
- Soft Light: W3C/PDF, Photoshop and Pegtop all fit within 1 (pure-green source cannot separate them).
- Hard Light: W3C; equals Normal for this source (coincidental match).
- Vivid Light: colour burn at 2s, colour dodge at 2s - 1, W3C edge rules; backdrop 0, source 1 gives 0.
- Linear Light: clamp(b + 2s - 1); equals Normal here (coincidental).
- Pin Light: min/max form; equals Normal here (coincidental).
- Hard Mix: 1 only if b + s > 1 (a sum of exactly 1 gives 0; ">= 1" ruled out).
- Exclusion: b + s - 2bs.
- Subtract: max(0, backdrop - source) (reverse order ruled out).
- Divide: backdrop / source; source 0 gives 1 when backdrop > 0 and 0 when backdrop is 0 (x = 219).

Not yet settled by a Mac render: the Soft Light variant, and Hard Light / Linear Light / Pin Light on a
source that is not pure green. A follow-up probe with 25%, 50% and 75% grey sources, opaque and
translucent, would settle them.

## Phase 3.5b follow-up probes (exported 2026-09-27)

The user exported 14 of the 15 Phase 3.5b follow-up probes from Compositor 1.2.10 (color-balance-preserve
was not exported; none of the 16 Phase 3.5c effects probes nor mac-effects yet). None was re-saved by the
Mac. They are committed beside the first set in `engine/tests/fixtures/mac-1.2.10-probes/`. Compared with
the port's CPU compositor at 12a0e34 (scratch test, deleted), straight RGBA8 and premultiplied:

| probe | result |
|---|---|
| color-balance-no-preserve, add-noise-uniform, add-noise-gaussian-mono, cgmode-levels-divide, color-dodge-adjustment, cgmode-stack-bases, invert | bit-identical on every pixel |
| black-white-tint | colour max 1 (2 pixels) |
| cgmode-blur-linear-burn | colour max 3, 28 pixels over 2, all at the canvas edge (the canvas edge does not fade: ruling E-I1 confirmed) |
| gaussian-blur-6 | premultiplied colour 2, alpha 2 (straight colour is ill-conditioned where alpha is a few units) |
| gaussian-blur-40 | premultiplied colour 2, alpha 3 (the halved path) |
| blur-soft-mask | premultiplied colour 1, alpha 1 |
| blend-greys | Hard Light, Linear Light, Pin Light, Vivid Light and Hard Mix within 1 at every grey and alpha; Soft Light 14 off at 75 % grey (see below) |
| motion-blur-30-24 | premultiplied colour 26, alpha 28, mean 3.1 (see below) |

So Color Burn and Color Dodge on adjustment layers and clipped groups, Hard Light / Linear Light /
Pin Light on non-pure sources, the Hard Mix edge, Color Balance without Preserve Luminosity, the B&W
tint, Invert, both noise modes and both Gaussian radii are settled as the port draws them.

Soft Light: fitted on band 0 of blend-greys (interior of each 40 x 60 column, R G B, W3C alpha model
`(1 - a) cb + a B(cb, cs)`), max / mean error in 8-bit levels:

| formula | 25 % | 50 % | 75 % | 25 % half alpha | 50 % half alpha | 75 % half alpha |
|---|---|---|---|---|---|---|
| W3C / PDF (the port) | 1 / 0.05 | 0 / 0 | 14 / 7.86 | 1 / 0.02 | 0 / 0 | 7 / 3.77 |
| Photoshop (sqrt always) | 1 / 0.05 | 0 / 0 | 15 / 8.19 | 1 / 0.02 | 0 / 0 | 7 / 3.77 |
| Pegtop `(1 - 2cs) cb^2 + 2 cs cb` | 1 / 0.05 | 0 / 0 | 1 / 0.05 | 1 / 0.02 | 0 / 0 | 1 / 0.02 |

The Mac's Soft Light is Pegtop's formula.

Motion Blur (30 degrees, 24 px; the Mac passes CIMotionBlur a radius of distance / sqrt(12),
Filters.swift:170-207, and so does the adjustment layer): models run on the port's composite of the
layers under the blur, sampled bilinearly every 0.25 px along the angle, transparent outside the
canvas, compared premultiplied:

| model | max | mean |
|---|---|---|
| the port (even streak of length 24) | 28 | 3.108 |
| Gaussian along the angle, sigma = 0.75 r | 36 | 4.529 |
| Gaussian along the angle, sigma = r = 6.93 | 4 | 0.103 |
| Gaussian along the angle, sigma = 1.25 r | 27 | 4.381 |
| tent with the same spread | 8 | 0.938 |

CIMotionBlur is a Gaussian along the angle whose sigma is its radius; with the Mac's radius that is
sigma = distance / sqrt(12).

## Generated tables

# Mac probe exports vs the Windows port's CPU compositor

Generated by probe-compare (throwaway crate). Both sides converted to straight RGBA8 before comparison. Diff images (|d| x4) in `C:/Users/sr9rfx/AppData/Local/Temp/claude/probe-compare/out`.


## folder-opacity

Document 120x60; Mac PNG 120x60; port composite 120x60.

Mac PNG chunks:
- IHDR: 120x60, bit depth 8, colour type 6 (RGBA), interlace 0
- sRGB: rendering intent 0
- eXIf: 120 bytes (TIFF MM: XResolution 72, YResolution 72, ResolutionUnit inch, ExifIFD: ColorSpace 1 = sRGB, PixelXDimension, PixelYDimension)
- pHYs: 2835 x 2835 per unit 1 (72.01 dpi)
- chunk order: IHDR(13) sRGB(1) eXIf(120) pHYs(9) IDAT(342) IEND(0)

Mac vs port: max |d| R 0 G 0 B 0 A 0; mean |d| R 0.000 G 0.000 B 0.000 A 0.000; pixels with any channel > 2: 0 of 7200 (0.0%); bit-identical pixels: 7200

| point | (x,y) | Mac straight | port straight | Mac premul | port premul |
|---|---|---|---|---|---|
| red only | (20,30) | [255, 0, 0, 128] | [255, 0, 0, 128] | [128, 0, 0, 128] | [128, 0, 0, 128] |
| overlap | (60,30) | [85, 0, 170, 192] | [85, 0, 170, 192] | [64, 0, 128, 192] | [64, 0, 128, 192] |
| blue only | (100,30) | [0, 0, 255, 128] | [0, 0, 255, 128] | [0, 0, 128, 128] | [0, 0, 128, 128] |
| overlap corner | (40,0) | [85, 0, 170, 192] | [85, 0, 170, 192] | [64, 0, 128, 192] | [64, 0, 128, 192] |

Expected per-descendant multiplication (R 4.1): red-only premul (128,0,0,128); overlap premul (64,0,128,192) i.e. straight (85,0,170,192); blue-only premul (0,0,128,128). Photoshop group compositing would give overlap straight (0,0,255,128).

## clipped-in-dimmed-folder

Document 120x60; Mac PNG 120x60; port composite 120x60.

Mac PNG chunks:
- IHDR: 120x60, bit depth 8, colour type 6 (RGBA), interlace 0
- sRGB: rendering intent 0
- eXIf: 120 bytes (TIFF MM: XResolution 72, YResolution 72, ResolutionUnit inch, ExifIFD: ColorSpace 1 = sRGB, PixelXDimension, PixelYDimension)
- pHYs: 2835 x 2835 per unit 1 (72.01 dpi)
- chunk order: IHDR(13) sRGB(1) eXIf(120) pHYs(9) IDAT(230) IEND(0)

Mac vs port: max |d| R 0 G 0 B 0 A 0; mean |d| R 0.000 G 0.000 B 0.000 A 0.000; pixels with any channel > 2: 0 of 7200 (0.0%); bit-identical pixels: 7200

| point | (x,y) | Mac straight | port straight | Mac premul | port premul |
|---|---|---|---|---|---|
| centre | (60,30) | [255, 128, 128, 128] | [255, 128, 128, 128] | [128, 64, 64, 128] | [128, 64, 64, 128] |
| corner | (0,0) | [255, 128, 128, 128] | [255, 128, 128, 128] | [128, 64, 64, 128] | [128, 64, 64, 128] |
| far corner | (119,59) | [255, 128, 128, 128] | [255, 128, 128, 128] | [128, 64, 64, 128] | [128, 64, 64, 128] |

Expected double dim (R 4.1): premul (128,64,64,128), straight (255,128,128,128). Photoshop (single dim): straight (255,0,0,128).

## guides

Document 120x60; Mac PNG 120x60; port composite 120x60.

Mac PNG chunks:
- IHDR: 120x60, bit depth 8, colour type 6 (RGBA), interlace 0
- sRGB: rendering intent 0
- eXIf: 120 bytes (TIFF MM: XResolution 72, YResolution 72, ResolutionUnit inch, ExifIFD: ColorSpace 1 = sRGB, PixelXDimension, PixelYDimension)
- pHYs: 2835 x 2835 per unit 1 (72.01 dpi)
- chunk order: IHDR(13) sRGB(1) eXIf(120) pHYs(9) IDAT(230) IEND(0)

Mac vs port: max |d| R 0 G 0 B 0 A 0; mean |d| R 0.000 G 0.000 B 0.000 A 0.000; pixels with any channel > 2: 0 of 7200 (0.0%); bit-identical pixels: 7200
Every Mac pixel is (255,255,255,255): true.

| point | (x,y) | Mac straight | port straight | Mac premul | port premul |
|---|---|---|---|---|---|
| on vertical guide x=30 | (30,10) | [255, 255, 255, 255] | [255, 255, 255, 255] | [255, 255, 255, 255] | [255, 255, 255, 255] |
| on horizontal guide y=45 | (60,45) | [255, 255, 255, 255] | [255, 255, 255, 255] | [255, 255, 255, 255] | [255, 255, 255, 255] |
| y=46 | (60,46) | [255, 255, 255, 255] | [255, 255, 255, 255] | [255, 255, 255, 255] | [255, 255, 255, 255] |
| crossing | (30,45) | [255, 255, 255, 255] | [255, 255, 255, 255] | [255, 255, 255, 255] | [255, 255, 255, 255] |


## new-blend-modes

Document 220x60; Mac PNG 220x60; port composite 220x60.

Mac PNG chunks:
- IHDR: 220x60, bit depth 8, colour type 6 (RGBA), interlace 0
- sRGB: rendering intent 0
- eXIf: 120 bytes (TIFF MM: XResolution 72, YResolution 72, ResolutionUnit inch, ExifIFD: ColorSpace 1 = sRGB, PixelXDimension, PixelYDimension)
- pHYs: 2835 x 2835 per unit 1 (72.01 dpi)
- chunk order: IHDR(13) sRGB(1) eXIf(120) pHYs(9) IDAT(530) IEND(0)

Mac vs port (port draws the 11 modes as Normal): max |d| R 128 G 128 B 128 A 0; mean |d| R 32.286 G 69.818 B 32.868 A 0.000; pixels with any channel > 2: 9600 of 13200 (72.7%); bit-identical pixels: 3600
Source column pixel (straight, as the port decodes it): [0, 255, 0, 128]; alpha a = 128/255 = 0.5020.
Max difference between any Mac row and row 30: 0 (0 means every row is the same, so row 30 speaks for the column).

### Sample points (row 30): backdrop, source, Mac, port

| mode | x | backdrop RGB | source straight RGBA | Mac RGBA | port RGBA (Normal) |
|---|---|---|---|---|---|
| Linear Burn | 1 | (254,0,1) | [0, 255, 0, 128] | [127, 0, 0, 255] | [127, 128, 0, 255] |
| Linear Burn | 10 | (243,0,12) | [0, 255, 0, 128] | [121, 0, 6, 255] | [121, 128, 6, 255] |
| Linear Burn | 18 | (234,0,21) | [0, 255, 0, 128] | [117, 0, 10, 255] | [117, 128, 10, 255] |
| Linear Dodge (Add) | 21 | (231,0,24) | [0, 255, 0, 128] | [231, 128, 24, 255] | [115, 128, 12, 255] |
| Linear Dodge (Add) | 30 | (220,0,35) | [0, 255, 0, 128] | [220, 128, 35, 255] | [110, 128, 17, 255] |
| Linear Dodge (Add) | 38 | (211,0,44) | [0, 255, 0, 128] | [211, 128, 44, 255] | [105, 128, 22, 255] |
| Soft Light | 41 | (207,0,48) | [0, 255, 0, 128] | [187, 0, 28, 255] | [103, 128, 24, 255] |
| Soft Light | 50 | (197,0,58) | [0, 255, 0, 128] | [174, 0, 35, 255] | [98, 128, 29, 255] |
| Soft Light | 58 | (187,0,68) | [0, 255, 0, 128] | [162, 0, 43, 255] | [93, 128, 34, 255] |
| Hard Light | 61 | (184,0,71) | [0, 255, 0, 128] | [92, 128, 35, 255] | [92, 128, 35, 255] |
| Hard Light | 70 | (173,0,82) | [0, 255, 0, 128] | [86, 128, 41, 255] | [86, 128, 41, 255] |
| Hard Light | 78 | (164,0,91) | [0, 255, 0, 128] | [82, 128, 45, 255] | [82, 128, 45, 255] |
| Vivid Light | 81 | (161,0,94) | [0, 255, 0, 128] | [80, 0, 47, 255] | [80, 128, 47, 255] |
| Vivid Light | 90 | (150,0,105) | [0, 255, 0, 128] | [75, 0, 52, 255] | [75, 128, 52, 255] |
| Vivid Light | 98 | (141,0,114) | [0, 255, 0, 128] | [70, 0, 57, 255] | [70, 128, 57, 255] |
| Linear Light | 101 | (137,0,118) | [0, 255, 0, 128] | [68, 128, 59, 255] | [68, 128, 59, 255] |
| Linear Light | 110 | (127,0,128) | [0, 255, 0, 128] | [63, 128, 64, 255] | [63, 128, 64, 255] |
| Linear Light | 118 | (118,0,137) | [0, 255, 0, 128] | [59, 128, 68, 255] | [59, 128, 68, 255] |
| Pin Light | 121 | (114,0,141) | [0, 255, 0, 128] | [57, 128, 70, 255] | [57, 128, 70, 255] |
| Pin Light | 130 | (104,0,151) | [0, 255, 0, 128] | [52, 128, 75, 255] | [52, 128, 75, 255] |
| Pin Light | 138 | (94,0,161) | [0, 255, 0, 128] | [47, 128, 80, 255] | [47, 128, 80, 255] |
| Hard Mix | 141 | (91,0,164) | [0, 255, 0, 128] | [45, 0, 82, 255] | [45, 128, 82, 255] |
| Hard Mix | 150 | (80,0,175) | [0, 255, 0, 128] | [40, 0, 87, 255] | [40, 128, 87, 255] |
| Hard Mix | 158 | (71,0,184) | [0, 255, 0, 128] | [35, 0, 92, 255] | [35, 128, 92, 255] |
| Exclusion | 161 | (68,0,187) | [0, 255, 0, 128] | [68, 128, 187, 255] | [34, 128, 93, 255] |
| Exclusion | 170 | (57,0,198) | [0, 255, 0, 128] | [57, 128, 198, 255] | [28, 128, 99, 255] |
| Exclusion | 178 | (48,0,207) | [0, 255, 0, 128] | [48, 128, 207, 255] | [24, 128, 103, 255] |
| Subtract | 181 | (44,0,211) | [0, 255, 0, 128] | [44, 0, 211, 255] | [22, 128, 105, 255] |
| Subtract | 190 | (34,0,221) | [0, 255, 0, 128] | [34, 0, 221, 255] | [17, 128, 110, 255] |
| Subtract | 198 | (24,0,231) | [0, 255, 0, 128] | [24, 0, 231, 255] | [12, 128, 115, 255] |
| Divide | 201 | (21,0,234) | [0, 255, 0, 128] | [138, 0, 245, 255] | [10, 128, 117, 255] |
| Divide | 210 | (10,0,245) | [0, 255, 0, 128] | [133, 0, 250, 255] | [5, 128, 122, 255] |
| Divide | 218 | (1,0,254) | [0, 255, 0, 128] | [128, 0, 255, 255] | [0, 128, 127, 255] |

### Formula fit per mode

Each candidate is evaluated over the column's interior (x = 20i+1 .. 20i+18, row 30, R, G and B), in sRGB-encoded 8-bit values, prediction rounded to nearest. Error = max |Mac - prediction| in 8-bit units.

| mode | formula B(cb,cs) | alpha model | max err | mean err |
|---|---|---|---|---|
| Linear Burn | cb+cs-1 | W3C: (1-a)cb + a*B(cb,cs) | 0 | 0.00 |
| Linear Burn | cb+cs-1 | B(cb, cs*a) premul source as colour | 127 | 42.33 |
| Linear Burn | cb+cs-1 | B at full strength (alpha ignored) | 127 | 42.33 |
| Linear Burn | Normal (=cs) | W3C: (1-a)cb + a*B(cb,cs) | 128 | 42.67 |
| Linear Burn | Normal (=cs) | B(cb, cs*a) premul source as colour | 128 | 85.00 |
| Linear Burn | Normal (=cs) | B at full strength (alpha ignored) | 255 | 127.33 |
| Linear Dodge (Add) | cb+cs | W3C: (1-a)cb + a*B(cb,cs) | 0 | 0.00 |
| Linear Dodge (Add) | cb+cs | B(cb, cs*a) premul source as colour | 0 | 0.00 |
| Linear Dodge (Add) | cb+cs | B at full strength (alpha ignored) | 127 | 42.33 |
| Linear Dodge (Add) | Normal (=cs) | W3C: (1-a)cb + a*B(cb,cs) | 116 | 42.67 |
| Linear Dodge (Add) | Normal (=cs) | B(cb, cs*a) premul source as colour | 231 | 85.00 |
| Linear Dodge (Add) | Normal (=cs) | B at full strength (alpha ignored) | 231 | 127.33 |
| Soft Light | W3C/PDF | W3C: (1-a)cb + a*B(cb,cs) | 1 | 0.11 |
| Soft Light | W3C/PDF | B(cb, cs*a) premul source as colour | 25 | 14.63 |
| Soft Light | W3C/PDF | B at full strength (alpha ignored) | 25 | 14.63 |
| Soft Light | Photoshop | W3C: (1-a)cb + a*B(cb,cs) | 1 | 0.11 |
| Soft Light | Photoshop | B(cb, cs*a) premul source as colour | 25 | 14.63 |
| Soft Light | Photoshop | B at full strength (alpha ignored) | 25 | 14.63 |
| Soft Light | Pegtop | W3C: (1-a)cb + a*B(cb,cs) | 1 | 0.11 |
| Soft Light | Pegtop | B(cb, cs*a) premul source as colour | 25 | 14.63 |
| Soft Light | Pegtop | B at full strength (alpha ignored) | 25 | 14.63 |
| Soft Light | Normal (=cs) | W3C: (1-a)cb + a*B(cb,cs) | 128 | 70.33 |
| Soft Light | Normal (=cs) | B(cb, cs*a) premul source as colour | 187 | 112.67 |
| Soft Light | Normal (=cs) | B at full strength (alpha ignored) | 255 | 155.00 |
| Hard Light | W3C | W3C: (1-a)cb + a*B(cb,cs) | 0 | 0.00 |
| Hard Light | W3C | B(cb, cs*a) premul source as colour | 127 | 84.67 |
| Hard Light | W3C | B at full strength (alpha ignored) | 127 | 84.67 |
| Hard Light | Normal (=cs) | W3C: (1-a)cb + a*B(cb,cs) | 0 | 0.00 |
| Hard Light | Normal (=cs) | B(cb, cs*a) premul source as colour | 92 | 42.33 |
| Hard Light | Normal (=cs) | B at full strength (alpha ignored) | 127 | 84.67 |
| Vivid Light | burn/dodge W3C edges | W3C: (1-a)cb + a*B(cb,cs) | 0 | 0.00 |
| Vivid Light | burn/dodge W3C edges | B(cb, cs*a) premul source as colour | 80 | 42.33 |
| Vivid Light | burn/dodge W3C edges | B at full strength (alpha ignored) | 80 | 42.33 |
| Vivid Light | burn/dodge raw (no cb edge rule) | W3C: (1-a)cb + a*B(cb,cs) | 128 | 42.67 |
| Vivid Light | burn/dodge raw (no cb edge rule) | B(cb, cs*a) premul source as colour | 80 | 42.33 |
| Vivid Light | burn/dodge raw (no cb edge rule) | B at full strength (alpha ignored) | 255 | 127.33 |
| Vivid Light | Normal (=cs) | W3C: (1-a)cb + a*B(cb,cs) | 128 | 42.67 |
| Vivid Light | Normal (=cs) | B(cb, cs*a) premul source as colour | 128 | 85.00 |
| Vivid Light | Normal (=cs) | B at full strength (alpha ignored) | 255 | 127.33 |
| Linear Light | cb+2cs-1 | W3C: (1-a)cb + a*B(cb,cs) | 0 | 0.00 |
| Linear Light | cb+2cs-1 | B(cb, cs*a) premul source as colour | 127 | 84.67 |
| Linear Light | cb+2cs-1 | B at full strength (alpha ignored) | 127 | 84.67 |
| Linear Light | Normal (=cs) | W3C: (1-a)cb + a*B(cb,cs) | 0 | 0.00 |
| Linear Light | Normal (=cs) | B(cb, cs*a) premul source as colour | 68 | 42.33 |
| Linear Light | Normal (=cs) | B at full strength (alpha ignored) | 127 | 84.67 |
| Pin Light | min/max | W3C: (1-a)cb + a*B(cb,cs) | 0 | 0.00 |
| Pin Light | min/max | B(cb, cs*a) premul source as colour | 127 | 84.67 |
| Pin Light | min/max | B at full strength (alpha ignored) | 127 | 84.67 |
| Pin Light | Normal (=cs) | W3C: (1-a)cb + a*B(cb,cs) | 0 | 0.00 |
| Pin Light | Normal (=cs) | B(cb, cs*a) premul source as colour | 80 | 42.33 |
| Pin Light | Normal (=cs) | B at full strength (alpha ignored) | 127 | 84.67 |
| Hard Mix | cb+cs>=1 | W3C: (1-a)cb + a*B(cb,cs) | 128 | 42.67 |
| Hard Mix | cb+cs>=1 | B(cb, cs*a) premul source as colour | 92 | 42.33 |
| Hard Mix | cb+cs>=1 | B at full strength (alpha ignored) | 255 | 127.33 |
| Hard Mix | cb+cs>1 | W3C: (1-a)cb + a*B(cb,cs) | 0 | 0.00 |
| Hard Mix | cb+cs>1 | B(cb, cs*a) premul source as colour | 92 | 42.33 |
| Hard Mix | cb+cs>1 | B at full strength (alpha ignored) | 92 | 42.33 |
| Hard Mix | vivid<0.5?0:1 | W3C: (1-a)cb + a*B(cb,cs) | 0 | 0.00 |
| Hard Mix | vivid<0.5?0:1 | B(cb, cs*a) premul source as colour | 92 | 42.33 |
| Hard Mix | vivid<0.5?0:1 | B at full strength (alpha ignored) | 92 | 42.33 |
| Hard Mix | Normal (=cs) | W3C: (1-a)cb + a*B(cb,cs) | 128 | 42.67 |
| Hard Mix | Normal (=cs) | B(cb, cs*a) premul source as colour | 128 | 85.00 |
| Hard Mix | Normal (=cs) | B at full strength (alpha ignored) | 255 | 127.33 |
| Exclusion | cb+cs-2cbcs | W3C: (1-a)cb + a*B(cb,cs) | 0 | 0.00 |
| Exclusion | cb+cs-2cbcs | B(cb, cs*a) premul source as colour | 0 | 0.00 |
| Exclusion | cb+cs-2cbcs | B at full strength (alpha ignored) | 127 | 42.33 |
| Exclusion | Normal (=cs) | W3C: (1-a)cb + a*B(cb,cs) | 104 | 42.67 |
| Exclusion | Normal (=cs) | B(cb, cs*a) premul source as colour | 207 | 85.00 |
| Exclusion | Normal (=cs) | B at full strength (alpha ignored) | 207 | 127.33 |
| Subtract | cb-cs | W3C: (1-a)cb + a*B(cb,cs) | 0 | 0.00 |
| Subtract | cb-cs | B(cb, cs*a) premul source as colour | 0 | 0.00 |
| Subtract | cb-cs | B at full strength (alpha ignored) | 0 | 0.00 |
| Subtract | cs-cb | W3C: (1-a)cb + a*B(cb,cs) | 128 | 85.33 |
| Subtract | cs-cb | B(cb, cs*a) premul source as colour | 231 | 127.67 |
| Subtract | cs-cb | B at full strength (alpha ignored) | 255 | 170.00 |
| Subtract | Normal (=cs) | W3C: (1-a)cb + a*B(cb,cs) | 128 | 85.33 |
| Subtract | Normal (=cs) | B(cb, cs*a) premul source as colour | 231 | 127.67 |
| Subtract | Normal (=cs) | B at full strength (alpha ignored) | 255 | 170.00 |
| Divide | cb/cs (cs=0 -> 1) | W3C: (1-a)cb + a*B(cb,cs) | 0 | 0.00 |
| Divide | cb/cs (cs=0 -> 1) | B(cb, cs*a) premul source as colour | 127 | 42.33 |
| Divide | cb/cs (cs=0 -> 1) | B at full strength (alpha ignored) | 127 | 42.33 |
| Divide | cb/cs (cs=0 -> 1 always) | W3C: (1-a)cb + a*B(cb,cs) | 0 | 0.00 |
| Divide | cb/cs (cs=0 -> 1 always) | B(cb, cs*a) premul source as colour | 127 | 42.33 |
| Divide | cb/cs (cs=0 -> 1 always) | B at full strength (alpha ignored) | 127 | 42.33 |
| Divide | cs/cb | W3C: (1-a)cb + a*B(cb,cs) | 128 | 128.00 |
| Divide | cs/cb | B(cb, cs*a) premul source as colour | 255 | 212.67 |
| Divide | cs/cb | B at full strength (alpha ignored) | 255 | 212.67 |
| Divide | Normal (=cs) | W3C: (1-a)cb + a*B(cb,cs) | 128 | 128.00 |
| Divide | Normal (=cs) | B(cb, cs*a) premul source as colour | 255 | 170.33 |
| Divide | Normal (=cs) | B at full strength (alpha ignored) | 255 | 212.67 |

### Best fit per mode

- Linear Burn: best fit cb+cs-1 / W3C: (1-a)cb + a*B(cb,cs), max err 0
- Linear Dodge (Add): best fit cb+cs / W3C: (1-a)cb + a*B(cb,cs), max err 0
- Soft Light: best fit W3C/PDF / W3C: (1-a)cb + a*B(cb,cs), max err 1
- Hard Light: best fit W3C / W3C: (1-a)cb + a*B(cb,cs), max err 0
- Vivid Light: best fit burn/dodge W3C edges / W3C: (1-a)cb + a*B(cb,cs), max err 0
- Linear Light: best fit cb+2cs-1 / W3C: (1-a)cb + a*B(cb,cs), max err 0
- Pin Light: best fit min/max / W3C: (1-a)cb + a*B(cb,cs), max err 0
- Hard Mix: best fit cb+cs>1 / W3C: (1-a)cb + a*B(cb,cs), max err 0
- Exclusion: best fit cb+cs-2cbcs / W3C: (1-a)cb + a*B(cb,cs), max err 0
- Subtract: best fit cb-cs / W3C: (1-a)cb + a*B(cb,cs), max err 0
- Divide: best fit cb/cs (cs=0 -> 1) / W3C: (1-a)cb + a*B(cb,cs), max err 0

Column edge pixels (row 30), to check the columns' hard edges and the gradient endpoints:

| x | backdrop | Mac | port |
|---|---|---|---|
| 0 | (255,0,0) | [127, 0, 0, 255] | [127, 128, 0, 255] |
| 19 | (233,0,22) | [116, 0, 11, 255] | [116, 128, 11, 255] |
| 20 | (232,0,23) | [232, 128, 23, 255] | [116, 128, 11, 255] |
| 39 | (210,0,45) | [210, 128, 45, 255] | [105, 128, 22, 255] |
| 40 | (208,0,47) | [189, 0, 28, 255] | [104, 128, 23, 255] |
| 199 | (23,0,232) | [23, 0, 232, 255] | [11, 128, 116, 255] |
| 200 | (22,0,233) | [139, 0, 244, 255] | [11, 128, 116, 255] |
| 219 | (0,0,255) | [0, 0, 255, 255] | [0, 128, 127, 255] |

Minimum Mac alpha over the image: 255.

## new-adjustment-layers

Document 120x60; Mac PNG 120x60; port composite 120x60.

Mac PNG chunks:
- IHDR: 120x60, bit depth 8, colour type 6 (RGBA), interlace 0
- sRGB: rendering intent 0
- eXIf: 120 bytes (TIFF MM: XResolution 72, YResolution 72, ResolutionUnit inch, ExifIFD: ColorSpace 1 = sRGB, PixelXDimension, PixelYDimension)
- pHYs: 2835 x 2835 per unit 1 (72.01 dpi)
- chunk order: IHDR(13) sRGB(1) eXIf(120) pHYs(9) IDAT(312) IEND(0)

Mac vs port (port leaves Black & White undrawn = identity): max |d| R 124 G 165 B 165 A 0; mean |d| R 72.100 G 84.750 B 84.683 A 0.000; pixels with any channel > 2: 7200 of 7200 (100.0%); bit-identical pixels: 0
### Candidate formulas vs the Mac

- Mac adjust_black_white, Photoshop defaults (R40 Y60 G40 C60 B20 M80): max |d| 0, mean |d| 0.000
- Rec.709 luma (0.2126, 0.7152, 0.0722): max |d| 106, mean |d| 50.400
- Rec.601 luma (0.299, 0.587, 0.114): max |d| 80, mean |d| 35.250
- mean (R+G+B)/3: max |d| 28, mean |d| 12.667

Mac vs the B&W model: max |d| R 0 G 0 B 0 A 0; mean |d| R 0.000 G 0.000 B 0.000 A 0.000; pixels with any channel > 2: 0 of 7200 (0.0%); bit-identical pixels: 7200

Every Mac pixel has R = G = B: true.

| x (row 30) | backdrop RGB | Mac | port (identity) | B&W model |
|---|---|---|---|---|
| 0 | (242,36,36) | [118, 118, 118, 255] | [242, 36, 36, 255] | [118, 118, 118, 255] |
| 10 | (242,139,36) | [139, 139, 139, 255] | [242, 139, 36, 255] | [139, 139, 139, 255] |
| 20 | (242,242,36) | [160, 160, 160, 255] | [242, 242, 36, 255] | [160, 160, 160, 255] |
| 30 | (139,242,36) | [139, 139, 139, 255] | [139, 242, 36, 255] | [139, 139, 139, 255] |
| 40 | (36,242,36) | [118, 118, 118, 255] | [36, 242, 36, 255] | [118, 118, 118, 255] |
| 50 | (36,242,139) | [139, 139, 139, 255] | [36, 242, 139, 255] | [139, 139, 139, 255] |
| 60 | (36,242,242) | [160, 160, 160, 255] | [36, 242, 242, 255] | [160, 160, 160, 255] |
| 70 | (36,139,242) | [118, 118, 118, 255] | [36, 139, 242, 255] | [118, 118, 118, 255] |
| 80 | (36,36,242) | [77, 77, 77, 255] | [36, 36, 242, 255] | [77, 77, 77, 255] |
| 90 | (139,36,242) | [139, 139, 139, 255] | [139, 36, 242, 255] | [139, 139, 139, 255] |
| 100 | (242,36,242) | [201, 201, 201, 255] | [242, 36, 242, 255] | [201, 201, 201, 255] |
| 110 | (242,36,139) | [160, 160, 160, 255] | [242, 36, 139, 255] | [160, 160, 160, 255] |
| 5 | (242,88,36) | [129, 129, 129, 255] | [242, 88, 36, 255] | [129, 129, 129, 255] |
| 45 | (36,242,88) | [129, 129, 129, 255] | [36, 242, 88, 255] | [129, 129, 129, 255] |
| 85 | (88,36,242) | [108, 108, 108, 255] | [88, 36, 242, 255] | [108, 108, 108, 255] |
| 119 | (242,36,47) | [123, 123, 123, 255] | [242, 36, 47, 255] | [123, 123, 123, 255] |

## grain

Document 120x60; Mac PNG 120x60; port composite 120x60.

Mac PNG chunks:
- IHDR: 120x60, bit depth 8, colour type 6 (RGBA), interlace 0
- sRGB: rendering intent 0
- eXIf: 120 bytes (TIFF MM: XResolution 72, YResolution 72, ResolutionUnit inch, ExifIFD: ColorSpace 1 = sRGB, PixelXDimension, PixelYDimension)
- pHYs: 2835 x 2835 per unit 1 (72.01 dpi)
- chunk order: IHDR(13) sRGB(1) eXIf(120) pHYs(9) IDAT(12239) IEND(0)

Mac vs port (port uses the OLD roughness kernel): max |d| R 22 G 22 B 22 A 0; mean |d| R 4.903 G 4.923 B 4.905 A 0.000; pixels with any channel > 2: 4925 of 7200 (68.4%); bit-identical pixels: 488
Grain settings as the port resolves them (no grainSettings key in the manifest): amount 25, size 1.5, roughness 50, seed 0. The Mac's GrainSettings() defaults are amount 25, size 1.5, roughness 50, seed 0.

Mac vs NEW-kernel model (1.2.10 AdjustPixels.c re-implemented here): max |d| R 0 G 0 B 0 A 0; mean |d| R 0.000 G 0.000 B 0.000 A 0.000; pixels with any channel > 2: 0 of 7200 (0.0%); bit-identical pixels: 7200
Port vs NEW-kernel model: max |d| R 22 G 22 B 22 A 0; mean |d| R 4.903 G 4.923 B 4.905 A 0.000; pixels with any channel > 2: 4925 of 7200 (68.4%); bit-identical pixels: 488
Mac vs backdrop (the grain itself): max |d| R 25 G 26 B 26 A 0; mean |d| R 5.377 G 5.393 B 5.376 A 0.000; pixels with any channel > 2: 5137 of 7200 (71.3%); bit-identical pixels: 390
Port vs backdrop (the grain itself): max |d| R 22 G 22 B 22 A 0; mean |d| R 4.995 G 5.004 B 4.995 A 0.000; pixels with any channel > 2: 5026 of 7200 (69.8%); bit-identical pixels: 445

- signed Mac - port, R: mean 0.033, sd 6.163
- signed Mac - port, G: mean 0.040, sd 6.191
- signed Mac - port, B: mean 0.033, sd 6.169

Grain residual (image - backdrop, mean of R,G,B): Mac mean -0.050 sd 6.704; port mean -0.085 sd 6.220; NEW model mean -0.050 sd 6.704.

| lag | Mac | port (OLD kernel) | NEW-kernel model |
|---|---|---|---|
| x+1 | 0.235 | 0.273 | 0.235 |
| y+1 | 0.239 | 0.278 | 0.239 |
| x+2 | -0.009 | 0.004 | -0.009 |
| x+3 | -0.016 | -0.001 | -0.016 |

| (x,y) | backdrop | Mac | port | NEW model |
|---|---|---|---|---|
| (0,0) | [242, 36, 36, 255] | [235, 29, 29, 255] | [240, 34, 34, 255] | [235, 29, 29, 255] |
| (1,0) | [242, 47, 36, 255] | [228, 33, 22, 255] | [241, 46, 35, 255] | [228, 33, 22, 255] |
| (2,0) | [242, 57, 36, 255] | [255, 70, 49, 255] | [245, 60, 39, 255] | [255, 70, 49, 255] |
| (30,20) | [139, 242, 36, 255] | [145, 248, 42, 255] | [135, 238, 32, 255] | [145, 248, 42, 255] |
| (31,20) | [129, 242, 36, 255] | [136, 249, 43, 255] | [130, 243, 37, 255] | [136, 249, 43, 255] |
| (60,30) | [36, 242, 242, 255] | [31, 237, 237, 255] | [33, 239, 239, 255] | [31, 237, 237, 255] |
| (61,30) | [36, 232, 242, 255] | [55, 251, 255, 255] | [34, 230, 240, 255] | [55, 251, 255, 255] |
| (90,45) | [139, 36, 242, 255] | [136, 33, 239, 255] | [133, 30, 236, 255] | [136, 33, 239, 255] |
| (119,59) | [242, 36, 47, 255] | [243, 37, 48, 255] | [242, 36, 47, 255] | [243, 37, 48, 255] |

## edited-rich-file

Document 160x80; Mac PNG 160x80; port composite 160x80.

Mac PNG chunks:
- IHDR: 160x80, bit depth 8, colour type 6 (RGBA), interlace 0
- sRGB: rendering intent 0
- eXIf: 120 bytes (TIFF MM: XResolution 72, YResolution 72, ResolutionUnit inch, ExifIFD: ColorSpace 1 = sRGB, PixelXDimension, PixelYDimension)
- pHYs: 2835 x 2835 per unit 1 (72.01 dpi)
- chunk order: IHDR(13) sRGB(1) eXIf(120) pHYs(9) IDAT(16384) IDAT(3771) IEND(0)

Mac vs port: max |d| R 255 G 174 B 255 A 190; mean |d| R 104.657 G 73.249 B 89.606 A 57.351; pixels with any channel > 2: 12038 of 12800 (94.0%); bit-identical pixels: 173
Layers as the port reads them (bottom to top):

- Folder | group true | opacity 0.5 | parent None | pixels None | transform origin (20, 10) size (120, 60) flipX true | adjustment None | undrawn ["settings from a newer version of Compositor"]
- Titled | group false | opacity 1 | parent None | pixels Some((60, 40)) | transform origin (70, 20) size (60, 40) flipX true | adjustment None | undrawn ["layer effects", "settings from a newer version of Compositor"]
- Folder 1 | group true | opacity 1 | parent None | pixels None | transform origin (20, 10) size (120, 60) flipX true | adjustment None | undrawn []
- Titled copy | group false | opacity 1 | parent Some(Some("Folder 1")) | pixels Some((60, 40)) | transform origin (70, 20) size (60, 40) flipX true | adjustment None | undrawn ["layer effects", "settings from a newer version of Compositor"]
- Blur | group false | opacity 1 | parent Some(Some("Folder 1")) | pixels None | transform origin (20, 10) size (120, 60) flipX true | adjustment Some(GaussianBlur) | undrawn ["Gaussian Blur adjustment layers", "settings from a newer version of Compositor"]

Document undrawn: ["Gaussian Blur adjustment layers", "layer effects", "settings from a newer version of Compositor"]

Bounding box of pixels differing by > 2 (Mac vs port): x 6..159, y 0..79.

Mean |d| (over R,G,B,A) per 20x20 cell; rows are y, columns are x:

| y \ x | 0-19 | 20-39 | 40-59 | 60-79 | 80-99 | 100-119 | 120-139 | 140-159 |
|---|---|---|---|---|---|---|---|---|
| 0-19 | 35.7 | 85.2 | 97.1 | 107.6 | 114.6 | 114.4 | 106.8 | 95.9 |
| 20-39 | 45.5 | 79.4 | 93.2 | 102.2 | 89.9 | 89.8 | 101.2 | 92.0 |
| 40-59 | 35.8 | 65.9 | 81.6 | 96.0 | 88.1 | 88.0 | 94.8 | 80.6 |
| 60-79 | 17.7 | 47.9 | 62.2 | 77.0 | 87.3 | 87.1 | 76.6 | 61.5 |

Alpha: Mac min 0 max 146; port min 0 max 255.

Mac vs (port composite + port gaussian_blur sigma 24, transparent edges): max |d| R 255 G 114 B 255 A 43; mean |d| R 42.841 G 26.465 B 38.209 A 12.469; pixels with any channel > 2: 11860 of 12800 (92.7%); bit-identical pixels: 358

Mean |d| (over R,G,B,A) per 20x20 cell; rows are y, columns are x:

| y \ x | 0-19 | 20-39 | 40-59 | 60-79 | 80-99 | 100-119 | 120-139 | 140-159 |
|---|---|---|---|---|---|---|---|---|
| 0-19 | 13.4 | 23.8 | 13.1 | 8.9 | 7.0 | 6.9 | 8.8 | 13.0 |
| 20-39 | 30.9 | 32.5 | 22.1 | 17.9 | 15.9 | 15.8 | 17.8 | 21.8 |
| 40-59 | 36.0 | 45.8 | 36.9 | 33.7 | 32.5 | 32.4 | 33.5 | 36.4 |
| 60-79 | 26.0 | 60.8 | 54.3 | 52.3 | 52.2 | 52.1 | 51.9 | 53.5 |

Mac vs (port composite + two modelled drop shadows + sigma-24 blur): max |d| R 255 G 85 B 255 A 2; mean |d| R 3.422 G 1.806 B 3.788 A 0.433; pixels with any channel > 2: 3352 of 12800 (26.2%); bit-identical pixels: 5212

Mean |d| (over R,G,B,A) per 20x20 cell; rows are y, columns are x:

| y \ x | 0-19 | 20-39 | 40-59 | 60-79 | 80-99 | 100-119 | 120-139 | 140-159 |
|---|---|---|---|---|---|---|---|---|
| 0-19 | 8.0 | 4.8 | 1.0 | 1.3 | 1.1 | 1.2 | 1.2 | 1.0 |
| 20-39 | 13.9 | 4.4 | 0.9 | 1.2 | 1.0 | 1.0 | 1.1 | 1.1 |
| 40-59 | 11.4 | 3.3 | 0.8 | 1.0 | 1.0 | 1.0 | 1.0 | 0.8 |
| 60-79 | 3.9 | 2.3 | 0.6 | 1.0 | 0.9 | 0.9 | 0.9 | 0.6 |

Same comparison in PREMULTIPLIED values (straight colour is ill-conditioned where alpha is a few units): max |d| R 1 G 1 B 1 A 2; mean |d| R 0.163 G 0.194 B 0.207 A 0.433; pixels with any channel > 2: 0 of 12800 (0.0%); bit-identical pixels: 5212
Of the straight-RGBA pixels differing by > 2 from this model, 867 have Mac alpha < 16.

| (x,y) | Mac straight | port straight | port+blur24 | port+shadow+blur24 |
|---|---|---|---|---|
| (0,0) | [0, 0, 0, 0] | [0, 0, 0, 0] | [0, 0, 0, 0] | [0, 0, 0, 0] |
| (10,10) | [0, 0, 0, 1] | [0, 0, 0, 0] | [0, 0, 0, 0] | [0, 0, 0, 1] |
| (30,20) | [146, 36, 146, 7] | [0, 0, 0, 0] | [170, 43, 170, 6] | [146, 36, 146, 7] |
| (50,40) | [128, 55, 140, 42] | [0, 0, 0, 0] | [167, 72, 183, 32] | [128, 55, 140, 42] |
| (60,40) | [118, 61, 145, 67] | [0, 0, 0, 0] | [154, 82, 183, 53] | [122, 65, 145, 67] |
| (80,40) | [106, 94, 142, 120] | [222, 36, 242, 255] | [129, 113, 175, 99] | [105, 93, 143, 121] |
| (100,40) | [102, 125, 123, 143] | [36, 242, 222, 255] | [121, 151, 147, 120] | [101, 126, 122, 144] |
| (60,65) | [91, 50, 109, 56] | [0, 0, 0, 0] | [150, 83, 180, 34] | [89, 49, 107, 57] |
| (60,75) | [77, 42, 95, 43] | [0, 0, 0, 0] | [151, 81, 185, 22] | [75, 41, 93, 44] |
| (80,70) | [74, 66, 100, 89] | [0, 0, 0, 0] | [132, 113, 177, 52] | [77, 66, 103, 89] |
| (100,75) | [67, 81, 81, 91] | [0, 0, 0, 0] | [122, 153, 148, 50] | [67, 83, 80, 92] |
| (150,70) | [94, 94, 34, 30] | [0, 0, 0, 0] | [175, 175, 64, 16] | [90, 90, 33, 31] |
| (159,79) | [73, 73, 18, 14] | [0, 0, 0, 0] | [170, 170, 43, 6] | [73, 73, 18, 14] |
