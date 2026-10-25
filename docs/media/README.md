# README media

These are captures of the real Compositor 0.11.0 editor, not generated interface mockups.

- `editor-cover.png`: an alpine cover with one photo layer and two editable text layers. Georgia title, Arial label, warm white text; all composed inside Compositor.
- `editor-color.png`: the photograph after a Hue/Saturation adjustment.
- `color-preview.gif` / `color-preview.mp4`: approximately 9.3 seconds, silent. Image → Hue/Saturation, saturation 0 → 10 → 22 → 35, Preview off/on, Apply. Captured frames retain their original timing; GIF encoding uses 8 fps and a 1000 px width, MP4 uses 12 fps and a 1080 px width. No UI states or processing results were fabricated.

## Capture environment

Recorded on 2026-10-25 using the editor source from commit `824488defc40bd55dc314efb2e6dcd8c6820f8e1`, including the validated Windows WebAssembly engine, in a separate local browser-preview copy on macOS. The mock bridge supplies shell services; actual image and text rendering use the application engine. Windows uses the same frontend. Native Windows chrome/dialogs and platform font rendering are not represented by these captures. The visible development marker has been retained.

The demonstration is illustrative media, not a replacement for the independent Windows acceptance or release verification. Capturing it did not modify production code or accepted artifacts.

## Source image provenance

The alpine landscape was generated for this demonstration with the built-in image-generation tool. It is an illustrative scene, not a photograph of an identified location. Only the landscape was generated; the editor interface, adjustments and text composition were captured from the running app. No third-party photograph or private user document was used.

Prompt:

> Create a single photorealistic landscape source image for a real image-editor product demonstration. This is ONLY the photograph to import, NOT a screenshot, no UI, no frame, no typography, no text, no logos. Wide 3:2 composition. A serene alpine lake, sculptural mountain peaks receding into mist, a small cluster of deep green pines on a rocky island at right, mirror reflections, soft coral-pink dawn clouds and muted blue-green water. Refined editorial travel photography, compelling composition, natural details, gently subdued contrast and restrained colors so a saturation adjustment can visibly enrich them. Crisp but not overprocessed, no people, no buildings, no boats. Save a high quality landscape image.
