# Core Graphics covered-source oracle

`core-graphics-opacity.json` contains 204 independent one-pixel results captured
on macOS 26.5.2 (25F84), using the sRGB premultiplied RGBA8 CGContext image draw
used by Compositor 1.4.5 (40). All inputs are synthetic byte tuples; no user image
or local path is included. The companion Swift script reproduces those inputs.

The matrix contains four backdrops, three sources, six opacity values and Normal,
Multiply and Screen. Twelve full-opacity Multiply cases are explicitly outside
this covered-source fix. The complete retained local diagnostic includes them;
five use a different Core Graphics integer blend result. They are not represented
as passing the floating-point blend contract here. The tested matrix covers all
180 partial-opacity cases plus 24 full-opacity Normal/Screen controls.

The CPU and GPU tests require exact RGBA8 equality to these independent results.
Do not regenerate expected bytes from the Windows renderer or relax assertions
to hide a mismatch. Preserve the original Mac diagnostic when changing an oracle.
