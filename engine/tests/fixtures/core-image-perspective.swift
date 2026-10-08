import Foundation
import CoreGraphics
import CoreImage
import Metal

// Synthetic inputs only. The observer changes dispatch solely in this process.
guard CommandLine.arguments.count == 3 else {
    fatalError("usage: capture new-output-directory matrix-fixture.json")
}
let output = URL(fileURLWithPath: CommandLine.arguments[1], isDirectory: true)
let rgb = CGColorSpace(name: CGColorSpace.sRGB)!
let context = CIContext(options: [.workingColorSpace: NSNull(), .outputColorSpace: NSNull(),
    .workingFormat: NSNumber(value: CIFormat.RGBAf.rawValue)])
func write(_ name: String, _ bytes: Data) throws {
    try bytes.write(to: output.appendingPathComponent(name), options: .withoutOverwriting)
}
func rendered(_ image: CIImage, _ bounds: CGRect) throws -> (CGImage, Data) {
    guard let cg = context.createCGImage(image, from: bounds, format: .RGBAf, colorSpace: rgb),
          cg.bitsPerComponent == 32, cg.bitsPerPixel == 128, cg.bitmapInfo.rawValue == 8449,
          cg.bytesPerRow >= cg.width * 16, let provider = cg.dataProvider?.data else {
        throw NSError(domain: "RequiredFloatLayout", code: 1)
    }
    let bytes = provider as Data
    guard bytes.count >= cg.bytesPerRow * cg.height else {
        throw NSError(domain: "RequiredFloatBytes", code: 1)
    }
    return (cg, bytes)
}
func values(_ raw: UnsafeRawBufferPointer, _ cg: CGImage, _ i: Int) -> [Float] {
    let offset = (i / cg.width) * cg.bytesPerRow + (i % cg.width) * 16
    return (0..<4).map { raw.loadUnaligned(fromByteOffset: offset + $0 * 4, as: Float.self) }
}
func floatInput(_ pixels: [Float], _ w: Int, _ h: Int) throws -> CIImage {
    let data = pixels.withUnsafeBytes { Data($0) }
    guard let cg = CGImage(width: w, height: h, bitsPerComponent: 32, bitsPerPixel: 128,
        bytesPerRow: w * 16, space: rgb, bitmapInfo: CGBitmapInfo(rawValue: 8449),
        provider: CGDataProvider(data: data as CFData)!, decode: nil,
        shouldInterpolate: false, intent: .defaultIntent) else {
        throw NSError(domain: "FloatInputUnavailable", code: 1)
    }
    return CIImage(cgImage: cg)
}

// Codable deliberately ignores expected Float32 and Double bits in the fixture.
struct Quad: Decodable {
    let name: String, width: Int, height: Int
    let cornersYUp: [[Double]]
}
let quads = try JSONDecoder().decode([Quad].self, from: Data(contentsOf:
    URL(fileURLWithPath: CommandLine.arguments[2])))
guard quads.count == 15 else { throw NSError(domain: "RequiredMatrixInputs", code: 1) }
let source = """
kernel vec4 generatedCoordinates() {
    vec2 p = destCoord();
    return vec4(p.x / 1024.0, (320.0 - p.y) / 1024.0, 0.0, 1.0);
}
"""
guard let generator = CIColorKernel(source: source) else {
    throw NSError(domain: "GeneratorUnavailable", code: 1)
}
func warped(_ q: Quad) throws -> CIImage {
    let extent = CGRect(x: 0, y: 0, width: q.width, height: q.height)
    guard let image = generator.apply(extent: extent, arguments: []) else {
        throw NSError(domain: "GeneratorApplyUnavailable", code: 1)
    }
    let c = q.cornersYUp.map { CIVector(x: $0[0], y: $0[1]) }
    return image.applyingFilter("CIPerspectiveTransform", parameters: [
        "inputTopLeft": c[0], "inputTopRight": c[1],
        "inputBottomRight": c[2], "inputBottomLeft": c[3]])
}
var baseline = [String: Data]()
for q in quads {
    baseline[q.name] = try rendered(warped(q), CGRect(x: 0, y: 0, width: q.width, height: q.height)).1
}
P7InstallKernelObserver()
defer { P7RemoveKernelObserver() }
let sentinel = CIVector(x: 0.123456789012345, y: 0.987654321098765, z: 0.333333333333333)
guard let identity = CIWarpKernel(source:
    "kernel vec2 observerIdentity(vec3 row) { return destCoord() + vec2(row.x-row.x, row.y-row.y); }"),
      let input = generator.apply(extent: CGRect(x: 0, y: 0, width: 512, height: 320), arguments: []) else {
    throw NSError(domain: "ObserverSelftestUnavailable", code: 1)
}
"observer-selftest".withCString { P7SetCaptureLabel($0) }
guard let observedIdentity = identity.apply(extent: input.extent, roiCallback: { _, r in r },
    image: input, arguments: [sentinel]) else { throw NSError(domain: "ObserverSelftestApply", code: 1) }
guard try rendered(input, input.extent).1 == rendered(observedIdentity, input.extent).1 else {
    throw NSError(domain: "ObserverSelftestPixelsChanged", code: 1)
}
for q in quads {
    q.name.withCString { P7SetCaptureLabel($0) }
    let bytes = try rendered(warped(q), CGRect(x: 0, y: 0, width: q.width, height: q.height)).1
    guard bytes == baseline[q.name] else { throw NSError(domain: "ObserverPixelsChanged", code: 1) }
    try write(q.name + ".provider.bin", bytes)
}
P7RemoveKernelObserver()
let capture = P7CaptureJSON()
try write("kernel-arguments.json", capture)
let captureObject = try JSONSerialization.jsonObject(with: capture) as! [String: Any]
let events = captureObject["events"] as! [[String: Any]]
let names = Set(events.filter { ($0["kernelName"] as? String) == "_perspectiveTransform" }
    .compactMap { $0["label"] as? String })
let unavailable = quads.map { $0.name }.filter { !names.contains($0) }

let reciprocalSource = """
kernel vec4 isolatedReciprocal(sample_f v) {
    return vec4(v.r, 1.0/v.r, v.g, 1.0);
}
"""
guard let reciprocalKernel = CIColorKernel(source: reciprocalSource) else {
    throw NSError(domain: "ReciprocalKernelUnavailable", code: 1)
}
let denseReport: [String: Any] = try autoreleasepool {
    let width = 2048, height = 4096, count = 1 << 23
    var data = [Float](repeating: 0, count: count * 4)
    for i in 0..<count {
        data[i * 4] = Float(bitPattern: 0x3f800000 + UInt32(i))
        data[i * 4 + 1] = Float(i) / Float(count)
        data[i * 4 + 3] = 1
    }
    let image = try floatInput(data, width, height)
    guard let mapped = reciprocalKernel.apply(extent: image.extent, arguments: [image]) else {
        throw NSError(domain: "ReciprocalApplyUnavailable", code: 1)
    }
    let (cg, bytes) = try rendered(mapped, image.extent)
    var results = [Float](repeating: 0, count: count)
    try bytes.withUnsafeBytes { raw in
        for i in 0..<count {
            let v = values(raw, cg, i)
            guard v[0].bitPattern == 0x3f800000 + UInt32(i),
                  v[2].bitPattern == (Float(i) / Float(count)).bitPattern,
                  v[1].isFinite, v[1] > 0 else {
                throw NSError(domain: "CompleteInputEchoFailed", code: i)
            }
            results[i] = v[1]
        }
    }
    try write("reciprocal-domain-f32.bin", results.withUnsafeBytes { Data($0) })
    return ["samples": count, "inputEchoMismatch": 0, "indexEchoMismatch": 0,
            "nonFinite": 0, "bytes": count * 4]
}
let report: [String: Any] = ["scope": "Standalone public Core Image synthetic arithmetic only",
    "os": ProcessInfo.processInfo.operatingSystemVersionString,
    "defaultMetalDevice": MTLCreateSystemDefaultDevice()?.name ?? "unavailable",
    "matrixInputs": quads.count, "matrixCaptureUnavailable": unavailable,
    "observerPixelsUnchanged": true, "reciprocal": denseReport,
    "sentinelHostValues": [Double(sentinel.x), Double(sentinel.y), Double(sentinel.z)]]
try write("result.json", JSONSerialization.data(withJSONObject: report, options: [.sortedKeys]))
print("Captured all normalized reciprocals; matrix inputs unavailable: \(unavailable)")
