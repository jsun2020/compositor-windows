# Mac round-trip acceptance for Phase 4 and Phase 5

The Windows 0.8.0 authoring implementation is published at
`04902eb80b5f4087baed50ec8ba994b6c8847e03` on
`codex/phase4-and-phase5`. Native macOS acceptance requires the user's Mac.
The format oracle is Compositor v1.4.5, which writes manifest version 11.
Record the actual application and macOS versions; an older application rejecting
version 11 does not establish a regression against the v1.4.5 oracle.

## Supplied fixture

The local delivery archive is
`build-artifacts/Mac-roundtrip-0.8.0.zip`. It contains:

- `Windows-authored.comp/manifest.json` and every referenced PNG under `images/`.
- `Windows-reference.png`, the exported 800 x 600 Windows composite.
- `Windows-state.json`, layer settings and the originating implementation commit.
- A Chinese README and an editable result template.

The fixture was created through the release-WASM Windows application, saved and
reopened there. It has seven layers: white background, point text with emoji/CJK
and partial UTF-16 color/font runs, wrapping fixed-box text with effects, rounded
rectangle with effects, ellipse, line and a shape with a painted grayscale mask.
The font run on the letter B uses `TimesNewRomanPSMT`; other text uses Arial.
Platform font availability, fallback and metrics can change text rasterization.
Record such changes separately from missing text, damaged runs or lost records.

## No-edit round trip

1. Unzip the archive and keep the original project intact. Open a copy of the
   entire `.comp` package in Compositor for Mac, preferably v1.4.5.
2. Compare the layer count, visibility, positions, mask, stroke and shadow with
   the reference PNG. Export `Mac-open.png` before making any edit.
3. Save a separate `Mac-no-edit.comp`, close it, reopen it and export
   `Mac-no-edit.png`. Record any opening, saving or missing-font message.

## Editing round trip

Use a separate copy saved as `Mac-edited.comp`:

1. Edit point text to `A` + emoji + CJK + `B - Mac edited`. Change the emoji's
   color and the B's font through partial selections. Check that the surrounding
   characters retain their styling. Undo and redo the edit.
2. Edit the fixed-box text and change its box size. Confirm wrapping, alignment,
   tracking and leading remain editable. Record the chosen dimensions.
3. Resize the rounded rectangle, ellipse and line. The rectangle should retain
   its corner-radius setting (18) and the line its width (12), rather than become
   a stretched bitmap. Record the new dimensions and any intentional parameter
   edits. Check that the masked shape retains its faded center.
4. Change the rounded rectangle's Stroke and Drop Shadow. Toggle preview and
   cancel once, then apply an intentional change. Reopen the effect editor and
   confirm the settings; text and shape records should remain editable.
5. Save, close, reopen, verify editability again, and export `Mac-edited.png`.

Optionally create a small `Mac-created.comp` with fresh Mac text, a shape and
effects. This covers the opposite starting direction as well.

## Return and Windows verification

Zip the full returned `.comp` packages together with the PNGs and completed
result template. A manifest alone omits the layer and mask images. The user
returns that ZIP in the current chat. Windows verification then checks:

- Opening every returned project without a missing asset or unsupported record.
- Layer identities, editable text/shape records, UTF-16 runs, effects and masks.
- No-edit pixel and manifest differences, with font rerasterization distinguished.
- Intentional edits against the user's notes and Mac exported composite.
- Windows save/reopen and continued editing of the returned Mac project.

The Compositor 1.4.5 return files were verified on 2026-10-03. See
[the return-file results](mac-roundtrip-results-2026-10-03.md) for Windows native
open/save/reopen and continued editing, retained records, and the exact no-edit
composite match. The edited and Mac-created exports retain quantified edge
differences. Mac evidence also does not close the Windows native clipboard or
Windows large-image responsiveness gates.
