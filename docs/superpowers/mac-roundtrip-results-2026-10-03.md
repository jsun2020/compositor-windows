# Mac 1.4.5 return-file verification

The user returned all three projects in `build-artifacts/mac-roundtrip-0.8.0.zip`
and reported Compositor 1.4.5 on macOS 26.5.2. Returned ZIP SHA-256:
`87999AF2A1C6215E35A25A2BBEA58F4821BCA373ACB45CE8902DF04BEF080FEC`.
The preserved original archive is `Mac-roundtrip-0.8.0-original.zip`, SHA-256
`C95E1718036DC90371951383360DE6E0FF86BF36F38B05541790056C6780D179`.
Both ZIP CRC checks pass. The original archives and user files were preserved;
verification uses new extracted copies under
`build-artifacts/mac-roundtrip-validation-20261003/`.

## File and engine round trips

- `Mac-no-edit.comp` and `Mac-edited.comp` both contain seven layers. Text,
  UTF-16 color/font ranges, live shape records, masks and effects load without
  unsupported records or missing assets.
- `Mac-created.comp` contains six layers: text plus five live shapes. It also
  opens and saves on Windows without unsupported records.
- All three projects passed Windows release-WASM open/save/reopen. Layer pixels
  and grayscale masks are byte-identical in their premultiplied engine form
  across that Windows save/reopen. Text, shape, effects and transforms retain
  their values. This checks engine/file interoperability separately from the GUI.
- The no-edit manifest differs from the Windows-authored source only by explicit
  default values: Normal blend, opacity 1, isGroup false and maskLinked true.
- The edited point text now ends in a second line `-test`, has a 442 x 116 fixed
  box, a changed emoji color at UTF-16 location 1/length 2 and `Verdana-Bold` at
  location 5/length 1. These changes survive Windows save/reopen.
- The edited rounded rectangle is at (545, 66.5), size 100 x 53, radius 18. The
  ellipse is at (475, 199), size 203 x 156. The result sheet's 100 x 53 entry is
  filed under fixed-box text, but the manifest identifies it as the rectangle;
  the second text layer's box remains 340 x 190.
- The line width remains 12, the painted mask retains its faded center, and both
  existing Stroke/Drop Shadow records retain their parameters. The returned
  manifests do not show an intentional Mac effect-parameter change; the result
  sheet also leaves preview/cancel/apply and undo/redo entries blank. Those
  individual Mac GUI gestures are not inferred from successful file loading.

## Composite comparison

Comparisons decode each PNG into straight RGBA8, rather than comparing compressed
file bytes. Mac-open and Mac-no-edit are identical to one another and to the
original Windows reference. Windows export from Mac-no-edit is also identical.

| Returned project | Size | Different pixels | Max channel difference | Mean absolute channel difference |
| --- | --- | ---: | ---: | ---: |
| Mac-no-edit | 800 x 600 | 0 | 0 | 0 |
| Mac-edited | 800 x 600 | 2,118 | 121 | 0.053 |
| Mac-created | 1920 x 1080 | 14,037 | 111 | 0.037835 |

The edited-project differences are confined to (540,61)-(664,137), around the
resized rounded rectangle and its shadow. All other pixels, including edited
text, ellipse and mask, match. The rectangle's fractional Y origin is recorded;
the comparison does not establish the cause of every edge difference. The
Mac-created differences have bounding box (137,267)-(1281,674), around transformed
text and shape edges. Pixel-identical acceptance is not claimed for these two
exports. Existing sampling work remains separate, and its user probe files are
untouched. Difference images and metrics are retained with the validation copies.

## Font-face fix found during continued editing

Saved bitmaps retain the Mac glyphs when opened. Re-rendering during a Windows
text edit previously treated PostScript names as CSS families. Measured at 72 px,
`Verdana-Bold` fell back to Arial's width (159.996 px for `Bwm`), whereas installed
bold Verdana measured 201.551 px. `TimesNewRomanPSMT` likewise selected fallback
Arial instead of Times New Roman.

Rasterization now resolves known Mac face names into a CSS family plus
weight/style, retaining the original name in the saved style and partial runs.
Helvetica explicitly falls back to Arial when absent; arbitrary family names
remain unchanged. The new browser regression compares the real worker's
`Verdana-Bold` bitmap against Canvas's installed bold Verdana glyphs, requires
exact pixels, and verifies the original saved face name after reopen. It passes.

Validation after the change: production TypeScript/Vite builds pass, 261 unit
tests pass in 40 files, and 13 browser file/format/text tests pass. Logs are
`build-mac-font.log`, `unit-mac-font.log` and `e2e-mac-font.log` under
`build-artifacts/phase4-remaining-validation/`.

## Production portable and continued editing

Portable `Compositor-portable-0.8.0-20261003-0927.zip` (4,742,141 bytes) uses marker
`COMPOSITOR_BUILD_0.8.0_20261003-0927`. ZIP SHA-256 is
`05F4777A81408E788494B0EDDB0361CF6E411C56367346053A7EC99E690DCDDE`;
executable SHA-256 is
`1D177B74314D5A799BE80E62DBE5036D1C1D5D33F0BB26283772F7B4DD49F42B`.
ZIP CRCs pass; its executable matches the unpacked one. The archive contains
only the executable, README and retained Compositor MIT license.

The production Tauri window, without the development test API, opened all three
Mac projects through the File menu, exported PNGs and atomically saved new
packages. The native outputs' manifests and image files match the separately
checked engine saves exactly, and all three native PNGs match the engine exports.
Text editor content and base font settings were inspected on each project.

On a new copy of `Mac-edited`, Windows selected the B font run (`Verdana-Bold`),
changed the text's font size to 38, resized the rounded rectangle to width 150,
changed Stroke Size to 7 and Drop Shadow Distance to 14, then saved, closed and
reopened it. The reopened controls retain width 150, stroke 7 and shadow 14. The
saved manifest retains `Verdana-Bold` at UTF-16 location 5/length 1, font size 38
and corner radius 18. The Move bar applies the existing integer transform
rounding: the rectangle is saved at (545,67), size 150 x 80. No-edit save does not
round or alter its original fractional transform.

The native run recorded four package reads, four atomic package commits and zero
page errors. Only file-picker choices were substituted via dialog-only IPC
responses; package/image reads, exports, writes and editing used the production
native bridge. This does not test the Windows file-picker UI itself. The first
harness attempts failed because Tauri's invoke property is immutable and the
test destination parent did not exist; the corrected harness uses dialog-only
fetch responses and creates the destination parents. Those failed logs are
retained separately and were not application-source failures.

Evidence: `native-runtime-mac-return-ready.log` in the validation-log directory;
`native-mac-roundtrip.json`, `native-output-checks.json`, the native screenshot,
`native-windows-resaved/` and `native-windows-continued/` in the Mac validation
directory. All returned originals remain unchanged.

## Remaining acceptance

File and editable-record interoperability has live Mac return evidence. The
quantified composite differences above remain open. Windows external-image
clipboard interoperability and the previously failing 100 MP Content-Aware Fill
frame budget also remain open; this return-file validation does not close them.

## Production 1206 acceptance follow-up

The latest portable repeats four native reads and four atomic saves with zero
page errors and no development API. It additionally checks Windows text-size and
shape-width undo/redo, effect preview/cancel, Apply and saved/reopened parameters.
All three native PNGs and 34 saved/continued package files match the prior checked
Windows outputs. This preserves the exact no-edit composite match and the
quantified edited/created export differences; it does not resolve those differences
or establish the Mac gestures left blank in the return sheet.

See [the full acceptance record](phase5-acceptance-2026-10-03.md) for package hashes,
new performance coverage and clipboard origin/DIB fixes. At 12:46 the production
1206 package independently passed six native clipboard protocol groups and four
exact PNG/bitmap pixel comparisons after native access returned. The original
clipboard was restored. Clipboard acceptance is now closed by that separate
Windows evidence; stable performance and the Mac gates above remain open.
