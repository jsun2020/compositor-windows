"""Package fixed upstream arithmetic separately from the Windows implementation."""
import argparse, hashlib, json, shutil, subprocess
from pathlib import Path

COMMIT = "0ab3ff58a3dcf3d52a455fe3ab0336ac95923edd"

def block(text, marker):
    start = text.index(marker)
    opening = text.index("{", start)
    # Swift snippets selected here contain balanced interpolation/closure braces.
    depth = 1
    at = opening + 1
    while depth:
        depth += (text[at] == "{") - (text[at] == "}")
        at += 1
    return text[start:at]

def assignments(value, prefix):
    result = []
    for key, val in value.items():
        target = prefix + "." + key
        if isinstance(val, dict):
            result.extend(assignments(val, target))
        elif key == "seed":
            continue
        elif key in ("rgb", "red", "green", "blue") and isinstance(val, list):
            result.append(target + " = [" + ",".join(f"CurvePoint(x: {p['x']}, y: {p['y']})" for p in val) + "]")
        elif isinstance(val, list):
            result.append(target + " = " + json.dumps(val))
        elif isinstance(val, bool):
            result.append(target + " = " + str(val).lower())
        elif isinstance(val, str):
            member = {"Bloom":"bloom","Halation":"halation","Diffusion":"diffusion", "Original":"original","Two Colors":"twoColors","Black & White":"blackWhite"}[val]
            result.append(target + " = ." + member)
        else:
            result.append(target + " = " + str(val))
    return result

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("source", type=Path)
    parser.add_argument("outputs", type=Path)
    parser.add_argument("destination", type=Path)
    args = parser.parse_args()
    assert subprocess.check_output(["git", "-C", str(args.source), "rev-parse", "HEAD"], text=True).strip() == COMMIT
    assert not args.destination.exists(), "Preserve previous packages"
    root = args.destination
    root.mkdir(parents=True)
    originals = root / "upstream"
    originals.mkdir()
    selected = ["Document/CameraRaw.swift", "Document/CameraRawTables.swift", "Document/CameraRawColor.swift", "Document/CameraRawGeometryCalibration.swift", "Document/CameraRawDetailOptics.swift", "Document/Scanlines.swift", "Document/Dither.swift", "Document/Curves.swift", "Document/BrushStroke.swift", "Document/ImageAdjustments.swift", "Document/PixelAdjust.swift", "Rendering/AdjustPixels.c", "Rendering/AdjustPixels.h", "Rendering/DitherPixels.c", "Rendering/DitherPixels.h", "Rendering/LensPixels.c", "Rendering/LensPixels.h", "Resources/CameraRawTables.bin"]
    sources = {}
    for relative in selected:
        source = args.source / "Compositor" / relative
        target = originals / source.name
        target.write_bytes(source.read_bytes())
        if source.suffix != ".bin": sources[source.name] = source.read_text(encoding="utf-8")
    shutil.copyfile(args.source / "LICENSE", root / "LICENSE-Compositor.txt")
    for name in ["input.rgba", "input16.rgba", "input.png", "cases.json"]: shutil.copyfile(args.outputs / name, root / name)
    for name in ["effects-opaque.comp", "effects-alpha.comp"]: shutil.copytree(args.outputs / name, root / name)
    # Keep all selected pixel functions unchanged. Exclude application/session UI and
    # unexercised geometry only, and remove isolation annotations for older Swift CLIs.
    raw = sources["CameraRaw.swift"].split("/// Histogram or the vectorscope")[0]
    color = sources["CameraRawColor.swift"]
    # Add back the declaration consumed by split, after the process enum.
    process = block(sources["CameraRawGeometryCalibration.swift"], "nonisolated enum CameraRawProcessVersion")
    calibration = process + "\nnonisolated struct CameraRawCalibrationSettings" + sources["CameraRawGeometryCalibration.swift"].split("nonisolated struct CameraRawCalibrationSettings",1)[1]
    brush = block(sources["BrushStroke.swift"], "nonisolated enum BrushRaster")
    brush = brush.replace(block(brush, "static func pixelToDocument"), "")
    curves = block(sources["Curves.swift"], "nonisolated struct CurvePoint") + "\nstruct CurvesSettings { var channels = Array(repeating: [CurvePoint(x:0,y:0), CurvePoint(x:255,y:255)], count:4)\n" + block(sources["Curves.swift"], "func value(") + "\n}"
    adjust = block(sources["ImageAdjustments.swift"], "nonisolated enum ImageAdjustmentPixels") + "\n" + block(sources["ImageAdjustments.swift"], "nonisolated struct AdjustmentColor")
    pixel = 'enum PixelAdjust { static let ciContext = CIContext(options: [.workingColorSpace: NSNull(), .outputColorSpace: NSNull()])\n' + block(sources["PixelAdjust.swift"], "static func render(") + "\n}"
    stub = """import AppKit
import CoreImage
enum ExportError: Error { case render }
enum ProjectError: Error { case invalid }
enum PixelFilter { static let lensStrength = 0.35 }
struct PaletteColor { var red: CGFloat; var green: CGFloat; var blue: CGFloat }
// Geometry is unchanged by this delta and deliberately absent from these cases.
struct CameraRawGeometrySettings: Equatable, Sendable {
 var adjusts: Bool { false }; var normalized: Self { self }
 func applying(shows: Bool) -> Self { self }
 func apply(_ image: CGImage) throws -> CGImage { image }
}
"""
    combined = "\n".join([stub, adjust, brush, curves, pixel, raw, color, calibration, sources["CameraRawDetailOptics.swift"], sources["CameraRawTables.swift"], block(sources["Dither.swift"], "nonisolated enum DitherColors"), sources["Scanlines.swift"]])
    combined = combined.replace("nonisolated ", "")
    (root / "Oracle.swift").write_text(combined, encoding="utf-8")
    cases = json.loads((root / "cases.json").read_text(encoding="utf-8"))
    lines = []
    for case in cases["cases"]:
        lines.append("do {")
        kind = case["kind"]
        lines.append("var settings = " + ("CameraRawSettings()" if kind == "CameraRaw" else "ScanlinesSettings()"))
        patch = case["patch"].copy()
        if kind == "Scanlines":
            for key in ("dark", "light"):
                if key in patch:
                    c = patch.pop(key)
                    lines.append(f"settings.{key} = AdjustmentColor(red:{c[0]},green:{c[1]},blue:{c[2]})")
        lines.extend(assignments(patch, "settings"))
        call = f"try settings.apply(image, seed: {case['patch'].get('seed',0)})" if kind == "CameraRaw" else "try settings.apply(image)"
        lines.append(f'let result = {call}; try save(result, "{case["name"]}")')
        lines.append("}")
    runner = """import AppKit
import ImageIO
let root = URL(fileURLWithPath: CommandLine.arguments[1], isDirectory:true)
let out = URL(fileURLWithPath: CommandLine.arguments[2], isDirectory:true)
let bytes = try Data(contentsOf: root.appendingPathComponent("input.rgba"))
let provider = CGDataProvider(data: bytes as CFData)!
let image = CGImage(width:128,height:96,bitsPerComponent:8,bitsPerPixel:32,bytesPerRow:512,space:CGColorSpace(name:CGColorSpace.sRGB)!,bitmapInfo:CGBitmapInfo(rawValue:CGImageAlphaInfo.premultipliedLast.rawValue|CGBitmapInfo.byteOrder32Big.rawValue),provider:provider,decode:nil,shouldInterpolate:false,intent:.defaultIntent)!
guard CameraRawTables.tables != nil else { fatalError("Fixed CRT3 resource did not load") }
func save(_ image: CGImage, _ name:String) throws {
 let copy = try BrushRaster.copy(image); var raw = Data()
 for y in 0..<image.height { raw.append(copy.data!.advanced(by:y*copy.bytesPerRow).assumingMemoryBound(to:UInt8.self),count:image.width*4) }
 try raw.write(to:out.appendingPathComponent(name+".rgba"),options:.withoutOverwriting)
 let dest = CGImageDestinationCreateWithURL(out.appendingPathComponent(name+".png") as CFURL,"public.png" as CFString,1,nil)!
 CGImageDestinationAddImage(dest,image,nil); guard CGImageDestinationFinalize(dest) else { throw ExportError.render }
 print("PASS "+name)
}
try save(image,"input-copy")
""" + "\n".join(lines) + r'''
let wideData = try Data(contentsOf: root.appendingPathComponent("input16.rgba"))
var wide = stride(from:0,to:wideData.count,by:2).map { UInt16(wideData[$0]) | UInt16(wideData[$0+1]) << 8 }
var quantized = [UInt8](repeating:0,count:128*96*4)
dither_quantize16(&wide,&quantized,128,96,512)
try Data(quantized).write(to:out.appendingPathComponent("quantize16.rgba"),options:.withoutOverwriting)
print("PASS quantize16")
print("PASS: all source pixel cases completed")
'''
    (root / "main.swift").write_text(runner, encoding="utf-8")
    (root / "Bridge.h").write_text('#include "upstream/AdjustPixels.h"\n#include "upstream/DitherPixels.h"\n#include "upstream/LensPixels.h"\n',encoding="utf-8")
    command = """#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")"
root="$PWD"
out="$root/Mac149-return-$(date +%Y%m%d-%H%M%S)"
mkdir "$out"
exec > >(tee "$out/run.log") 2>&1
echo "Checking frozen input hashes..."
shasum -a 256 -c SHA256SUMS.txt
echo "Recording this Mac and installed Compositor version..."
sw_vers > "$out/system.txt"
xcrun swiftc --version >> "$out/system.txt"
app="/Applications/Compositor.app"
if [ -f "$app/Contents/Info.plist" ]; then
  /usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$app/Contents/Info.plist" > "$out/installed-version.txt"
  test "$(cat "$out/installed-version.txt")" = '1.4.9' || { echo 'Compositor 1.4.9 required'; exit 1; }
else
  echo 'Compositor.app not found in /Applications'; exit 1
fi
mkdir -p "$out/Oracle.app/Contents/MacOS" "$out/Oracle.app/Contents/Resources"
cp upstream/CameraRawTables.bin "$out/Oracle.app/Contents/Resources/"
cat > "$out/Oracle.app/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?><plist version="1.0"><dict><key>CFBundleExecutable</key><string>Oracle</string><key>CFBundleIdentifier</key><string>local.compositor.mac149.oracle</string><key>CFBundlePackageType</key><string>APPL</string></dict></plist>
PLIST
for file in AdjustPixels DitherPixels LensPixels; do
  xcrun clang -O2 -fblocks -c "upstream/$file.c" -o "$out/$file.o"
done
xcrun swiftc -swift-version 5 -O -import-objc-header Bridge.h Oracle.swift main.swift "$out/AdjustPixels.o" "$out/DitherPixels.o" "$out/LensPixels.o" -framework AppKit -framework CoreImage -o "$out/Oracle.app/Contents/MacOS/Oracle"
"$out/Oracle.app/Contents/MacOS/Oracle" "$root" "$out"
cp source-receipt.json "$out/"
echo 'PASS source oracle. Now complete the two actual-app exports in MAC-STEPS.md.'
echo "Return this folder after the manual checks: $out"
read -r -p 'Press Enter to close...' ignored
"""
    (root / "MAC-149.command").write_bytes(command.encode("utf-8"))
    shutil.copyfile(Path(__file__).with_name("mac149-MAC-STEPS.md"),root / "MAC-STEPS.md")
    (root / "RESULT.txt").write_text("Mac Compositor version: 1.4.9\nSource oracle: PENDING\nActual application effects-opaque export: PENDING\nActual application effects-alpha export: PENDING\nNew Canvas / Rotate / Scanlines / Undo: PENDING\nExport As PNG/JPEG/PDF: PENDING\nSearch Commands / Last Filter / Fullscreen / Navigator: PENDING\nNotes:\n",encoding="utf-8")
    hashes = []
    for path in sorted(root.rglob("*")):
        if path.is_file() and path.name != "RESULT.txt":
            hashes.append({"name":path.relative_to(root).as_posix(),"bytes":path.stat().st_size,"sha256":hashlib.sha256(path.read_bytes()).hexdigest()})
    receipt = {"upstreamCommit":COMMIT,"oracle":"Unchanged selected upstream pixel functions; CLI isolation annotations removed; unexercised geometry/session/UI excluded","cases":len(cases["cases"]),"files":hashes}
    (root / "source-receipt.json").write_text(json.dumps(receipt,indent=2),encoding="utf-8")
    hashes.append({"name":"source-receipt.json","sha256":hashlib.sha256((root / "source-receipt.json").read_bytes()).hexdigest()})
    (root / "SHA256SUMS.txt").write_bytes("".join(f"{r['sha256']}  {r['name']}\n" for r in hashes).encode("utf-8"))
    print(root)

if __name__ == "__main__": main()
