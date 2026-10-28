<div align="center">

# Compositor for Windows

### Your photo. Your layers. Your next idea.

A free, open-source desktop editor for photo touch-ups, layered graphics and small GIF animations.

**[Download for Windows](https://github.com/jsun2020/compositor-windows/releases/latest)** · [See it in action](#a-small-edit-in-nine-seconds) · [简体中文](README.zh-CN.md)

[![Latest release](https://img.shields.io/github/v/release/jsun2020/compositor-windows?color=3c7d78&label=Windows)](https://github.com/jsun2020/compositor-windows/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-3c7d78)](engine/native/LICENSE-Compositor.txt)

</div>

![Compositor workspace showing an alpine travel cover, with a photo and two editable text layers](docs/media/editor-cover.png)

*From a photo to a cover: adjust the image, add a headline, keep the text editable.*

## A workspace you can recognize

Tools on the left, your canvas in the middle, layers on the right. Crop a photo, try a color adjustment, or combine images and text without leaving the same workspace. Preview changes before applying them, and use familiar shortcuts such as **B** for Brush, **T** for Type and **Ctrl+Z** for Undo.

## A small edit in nine seconds

Open **Image → Hue/Saturation**, increase saturation, toggle **Preview** to compare, then apply.

![Silent demonstration: open Hue/Saturation, increase saturation, compare before and after, then apply](docs/media/color-preview.gif)

[Watch the silent MP4](docs/media/color-preview.mp4) · [View a full-size edited image](docs/media/editor-color.png)

*Captured from the actual 0.11.0 editor frontend in its browser preview. Windows uses the same editing interface; native window chrome, dialogs and fonts may differ. [Media details](docs/media/README.md).*

## What will you make?

| Your next task | Tools to get it done | Take away |
| --- | --- | --- |
| Give a travel or product photo a finishing pass | Crop, Levels, Curves, color adjustments, Clone Stamp and Spot Healing | A PNG or JPEG ready to share |
| Make a social cover, poster or thumbnail | Image layers, editable text and shapes, masks, blend modes, shadows and strokes | An exported graphic and an editable `.comp` project |
| Develop a camera RAW image | Sensor decoding, exposure, white balance, tone curves and Camera Raw adjustments | A developed image you can keep editing with layers |
| Polish a short GIF | Frame editing, timing and playback preview | An animated GIF |

Need more room to experiment? Work with **16-bit/HDR layers**, import **PDF pages** and **HEIF** images, open supported **PSD/PSB** content, or use an ICC printer profile for **CMYK proofing and export**. See [format support and limits](docs/technical-reference.md#camera-raw-and-photoshop-import) before bringing a complex production file.

## Already have an image editor?

Compositor is worth a try when you want:

- **A portable tool for a focused job.** Extract a ZIP and open the app. Keep a copy for quick photo edits, covers and layered compositions.
- **Local editing, without an account.** Your working images stay on your computer; editing does not require a cloud upload or a subscription.
- **More than a flattened result.** Keep text, shapes, masks and layers in a project, then return to it for the next revision.
- **An open-source tool you can understand and adapt.** The editor is MIT-licensed, with its source and build instructions available here.

Use it alongside your existing workflow. Complex Photoshop documents and Mac/Windows rendering are not guaranteed to match exactly; check a copy of an important file first.

## Try your first edit

1. **[Download the latest Windows x64 ZIP](https://github.com/jsun2020/compositor-windows/releases/latest)** and extract the **entire** archive. Keep its helpers, DLLs and license files together.
2. Open **`Compositor.exe`**, then open an image with **File → Open**. Microsoft Edge WebView2 Runtime is required; Node and Rust are not.
3. Try **Image → Hue/Saturation** or **Curves**, compare the preview, then export a PNG or JPEG. Save a `.comp` project if you want to keep editing later.

`.comp` projects are folders: keep their contents together when copying or sharing them. This is a portable, unsigned release; it has no installer or automatic updater. The release page includes checksums for verifying the download.

<details>
<summary><strong>A few useful shortcuts</strong></summary>

| Action | Shortcut |
| --- | --- |
| Move / Type / Brush | V / T / B |
| Crop / Hand / Zoom | C / H / Z |
| Levels / Curves / Hue-Saturation | Ctrl+L / Ctrl+M / Ctrl+U |
| Duplicate layer | Ctrl+J |
| Undo | Ctrl+Z |
| Apply / cancel transform | Enter / Escape |

[Full shortcut reference](docs/technical-reference.md#shortcuts)

</details>

## What's new, and what's next?

**Windows 0.12.0** brings the Mac 1.5.11 update: align and distribute layers for cleaner layouts, revisit Vignette, Lens Effects, Tonal Contrast and Sharpen as editable adjustment layers, and crop to your own aspect ratio. Prism gains Blur, Fringe and two-way copies. [Read the release notes](docs/releases/0.12.0.md).

Found a rough edge or have an idea? **[Open an issue](https://github.com/jsun2020/compositor-windows/issues)** with what you were doing, your app version and, when useful, a small example you can share.

---

[Build & contribute](docs/technical-reference.md#build-and-test) · [Compatibility reference](docs/technical-reference.md#interoperability-and-limits) · [Acceptance report](pair-test-report.md) · [Release history](https://github.com/jsun2020/compositor-windows/releases)

Ported from Compositor for macOS. Built with Tauri, Rust/WebAssembly and React. Original Compositor sources and retouch kernels use MIT; [Lucide](https://lucide.dev) icons use ISC. Bundled decoder licenses and notices accompany every release.
