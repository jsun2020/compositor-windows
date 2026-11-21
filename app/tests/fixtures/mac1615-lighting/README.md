# Lighting photo references

Both implementations read the same four opaque, 8-bit RGB PNGs, downsampled with Lanczos to 440 pixels on the longest side. The three color photographs come from the scikit-image v0.23.2 data directory. The historical photograph comes from the Wellcome Collection.

| Fixture | Credit and source |
| --- | --- |
| astronaut.png | Eileen Collins, NASA, public domain. [Original file](https://github.com/scikit-image/scikit-image/blob/v0.23.2/skimage/data/astronaut.png) |
| coffee.png | Rachel Michetti, courtesy of Pikolo Espresso Bar, CC0. [Original file](https://github.com/scikit-image/scikit-image/blob/v0.23.2/skimage/data/coffee.png) |
| chelsea.png | Chelsea the cat, Stefan van der Walt, released into the public domain by the photographer. [Original file](https://github.com/scikit-image/scikit-image/blob/v0.23.2/skimage/data/chelsea.png) |
| hands.png | L. Haase after H. W. Berend, 1862. Wellcome Collection, V0029446. The image was resized, converted to PNG, and filtered and cropped for the comparison sheets. [Source image](https://commons.wikimedia.org/wiki/File:A_man%27s_hands,_palms_down,_opened_out_and_spread_onto_black_Wellcome_V0029446.jpg) · [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/) |

The source photographs and their licensing are also documented in the [scikit-image data catalog](https://scikit-image.org/docs/0.26.x/api/skimage.data.html).

## Generating the Mac reference

The eight `mac-*.png` files were rendered from the pinned Mac source on Apple Silicon using Apple Vision, Core ML and Metal. A command-line entry point loaded each file with ImageIO and passed its CGImage and CIImage to the production filter. The filter's default-constructed settings were used throughout. The volume light was placed at one quarter of the image width and 30% of the height from the top, equivalent to `CGPoint(x: 0.25 * width, y: 0.70 * height)` in Core Image. The final image was rendered with `PixelAdjust.render(_:width:height:isMask:)` using the original dimensions and `isMask: false`, then saved as PNG. Passing each input through that renderer without a filter reproduced all RGB samples exactly. [Mac source at 1.6.15](https://github.com/robbietilton/Compositor/tree/a48247eb668e4c00a7c8dafef544758c3012a719)

## Checking the Windows port

After building the WASM engine and installing Playwright Chromium, run `pnpm exec playwright test lighting-photos --workers 1 --retries 0`. The test opens the filter on each photo, applies it, exports the document and reads the RGB samples. The Windows CI workflow automatically discovers these cases.

`references.json` records the SHA-256 of each input and Mac output, the pixel rectangle used in the detail sheet, and the reviewed difference between the Windows code running in Chromium on macOS and the native Mac reference. Each test checks mean absolute error, the 95th percentile of channel errors, and mean error in the detail rectangle. The permitted drift above the reviewed values is 0.15, 1 and 0.35 channel levels, respectively. It also checks that applying a filter actually changes the photo. The test attaches the measured values to its Playwright result.
