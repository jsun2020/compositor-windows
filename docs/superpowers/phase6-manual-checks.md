# Phase 6 manual gesture checks

Use Compositor for Mac **1.4.5**, or the Windows **Phase 6 development portable**
supplied with these projects. Published Windows 0.8.0 does not include these modes.
These small synthetic projects contain no fonts or personal images.

Copy this entire folder to the Mac. Keep each `.comp` folder intact. Work on
copies; keep the original three projects and seed PNGs unchanged. Record the
actual gestures and outcomes in `RESULT.txt`; saved projects cannot prove
preview cancellation or undo by themselves.

1. Open `01-liquify.comp`. Select the **image thumbnail** of **Warp this layer**.
   Press **R**. On Mac the top bar is **Smear** with a Liquify / Blur / Smudge
   mode picker; Windows shows **Mode**. Choose **Liquify**, Size **40**,
   Hardness **50%**, Strength **50%**. Fit the canvas so the stripes are visible.
2. Drag the white circle horizontally to the right by approximately its own
   diameter. While holding the mouse, verify that the pixels move live. Press
   **Escape before releasing**. Verify that the original circle and stripes
   return. Record whether cancellation worked.
3. Repeat a similar stroke and release. Press **Cmd+Z / Cmd+Shift+Z** on Mac,
   or **Ctrl+Z / Ctrl+Shift+Z** on Windows. Verify one undo restores the image
   and redo restores the edit. Save a new copy as `01-liquify-edited.comp`,
   close/reopen it, then export a **640 × 480 PNG** as `01-liquify-edited.png`.
4. Open `02-smudge.comp`, choose **Smudge** with the same settings. Drag from
   the white circle into the colored stripes. Verify a fading white trail,
   live preview, Escape cancellation and one-step undo/redo as above. Save and
   reopen `02-smudge-edited.comp`, then export `02-smudge-edited.png` at 640 × 480.
5. Open `03-styled-mask.comp`. Select the image thumbnail, choose either warp
   mode and drag across the middle boundary. Verify that the stroke/shadow,
   80% layer opacity and the lighter left-half mask still affect the preview
   and committed result. Undo/redo and save/reopen. Save
   `03-styled-mask-edited.comp` and export `03-styled-mask-edited.png`.
6. In a fresh copy of the styled project, use **M** to select a rectangle over
   part of the circle. Return to **R**, drag across the selection boundary,
   and verify only the selected portion changes. Undo the stroke. Click the
   **mask thumbnail** and attempt a warp stroke: it should refuse and leave
   the mask unchanged. Record both checks; you need not save this copy.

Return the three edited `.comp` folders, three PNGs and `RESULT.txt`. If any
check fails, retain that copy and describe the exact action. Manual pointer
positions differ across machines: these returns establish real UI behavior
and saved-project rendering, not exact Metal/WebGL kernel equality.

The unchanged history budget is **256 MiB**. These small projects support
undo; a 100 MP full-layer replacement exceeds that budget and cannot retain
its preceding raster. Do not use a large personal project for these checks.
