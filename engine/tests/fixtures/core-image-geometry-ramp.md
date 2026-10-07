# Core Image Geometry phase oracle

`core-image-geometry-ramp.json` contains independent results from macOS 26.5.2
(25F84). The synthetic source is a 256 by 4 straight RGBA8 grey ramp, with RGB
equal to the column index and alpha 255. `CIPerspectiveTransform` translates it
by the recorded horizontal phase divided by 256. The companion Swift program
reproduces the inputs and captures row 1 of the resulting premultiplied RGBA8
bitmap using the Compositor 1.4.5 Core Image context configuration.

The fifteen phases include the half-byte rounding boundary and the image edge.
Every RGB channel in the received row equals `red`; `alpha` is captured
independently. Expected bytes come from the actual Mac return, never from the
Windows sampler. The Rust test requires exact RGBA equality for all 3,840 pixels
and preserves the source image.

This oracle verifies interpolation and byte conversion for these translations.
It does not establish perspective-coordinate precision for a rotated or
trapezoidal image, or complete Phase 7 acceptance. The complete returned
Geometry recipe remains a separate strict comparison. Do not relax it or
regenerate this fixture from Windows output.
