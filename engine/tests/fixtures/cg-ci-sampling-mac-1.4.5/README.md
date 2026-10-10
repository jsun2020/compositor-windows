# Independent Compositor 1.4.5 CG / CI sampling return

These fixed synthetic projects contain generated colour, alpha and gray patterns;
no user photographs or imported PSD/RAW files are included.

Reference source: Compositor 1.4.5, commit 086f1631573ccb2b57644e53b52bf1488fc976aa.
A standalone harness mechanically extracts the original LayerRenderer,
DistortWarp and PixelAdjust methods. The 42-case Mac return has 44 raw records,
including two separate perspective-mask L8 records. Returned on 2026-10-09
from Apple M5 Pro, macOS 26.5.2 (25F84). All 82 distributed source/input hashes
match the original fixed package. Two actual Compositor application exports
(sampling-high-rotated and sampling-high-mask-400) are byte exact against the
harness's corresponding outputs. Harness execution and application controls
remain distinct evidence.

mac-sha256.json pins every raw reference. result.json pins dimensions, channels
and placed transforms. Nearest affine rotation has conservative CG rectangle
coverage. Convex CI distortion uses the same filter for High and Nearest;
Nearest is retained as saved metadata. CI masks have clear black texture taps,
while uniform 1x1 masks pass through.

The test compares every output byte; independent CG/CI mask rounding is bounded
to one byte. Colour and ordinary affine/CI images are exact. Original Mac 1.2.10
fixture gates, source kernel goldens and hardware budgets remain separate.
