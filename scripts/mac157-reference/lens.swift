import Foundation
import CoreGraphics
import CoreImage
enum ExportError: Error { case render }
enum PixelAdjust { static let ciContext = CIContext(options: [.workingColorSpace: NSNull(), .outputColorSpace: NSNull()]) }
enum ImageAdjustmentPixels {
    static func clamp(_ value: Double, _ range: ClosedRange<Double>, _ fallback: Double) -> Double {
        value.isFinite ? min(range.upperBound, max(range.lowerBound, value)) : fallback
    }
}
let root = URL(fileURLWithPath: CommandLine.arguments[1], isDirectory: true)
let width = 96, height = 64, space = CGColorSpace(name: CGColorSpace.sRGB)!
var input = [UInt8](repeating: 0, count: width * height * 4)
for y in 0..<height { for x in 0..<width {
    let i = (y * width + x) * 4
    let alpha: UInt8 = x < 5 ? 0 : (x < 12 ? 128 : 255)
    var color = [Double(x) / Double(width) * 0.4, Double(y) / Double(height) * 0.2, 0.08]
    if (x - 35) * (x - 35) + (y - 20) * (y - 20) < 10 { color = [1, 1, 1] }
    if x > 65 && x < 72 && y > 39 && y < 45 { color = [1, 0.95, 0.65] }
    if x > 82 && y < 8 { color = [0.95, 1, 1] }
    for c in 0..<3 { input[i + c] = UInt8((color[c] * Double(alpha)).rounded()) }
    input[i + 3] = alpha
} }
try Data(input).write(to: root.appendingPathComponent("lens-input.rgba"))
let image = input.withUnsafeMutableBytes { bytes in
    CGContext(data: bytes.baseAddress, width: width, height: height, bitsPerComponent: 8, bytesPerRow: width * 4,
              space: space, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!.makeImage()!
}
var cases: [(String, LensEffectSettings)] = []
for effect in LensEffect.allCases { var s = LensEffectSettings(); s.effect = effect; cases.append((effect.rawValue, s)) }
for shape in [BokehShape.hexagon, .oval, .catsEye] {
    var s = LensEffectSettings(); s.effect = .bokeh; s.bokehRadius = 6; s.bokehShape = shape
    cases.append(("Bokeh-\(shape.rawValue)", s))
}
var prism = LensEffectSettings(); prism.effect = .prism; prism.prismAngle = 45; prism.prismOffset = 22
cases.append(("Prism-45", prism))
var star = LensEffectSettings(); star.effect = .starburst; star.starPoints = 14; star.starAngle = -35
cases.append(("Starburst-14", star))
for (name, settings) in cases {
    let filtered = try settings.apply(to: image)
    let output = PixelAdjust.ciContext.createCGImage(filtered, from: CGRect(x: 0, y: 0, width: width, height: height), format: .RGBA8, colorSpace: space)!
    var pixels = [UInt8](repeating: 0, count: width * height * 4)
    pixels.withUnsafeMutableBytes { bytes in
        let context = CGContext(data: bytes.baseAddress, width: width, height: height, bitsPerComponent: 8, bytesPerRow: width * 4, space: space,
            bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
        context.setBlendMode(.copy); context.draw(output, in: CGRect(x: 0, y: 0, width: width, height: height))
    }
    try Data(pixels).write(to: root.appendingPathComponent("lens-\(name).rgba"))
}
// Separate ramp probes isolate Core Image's tone-curve interpolation from optics/resampling.
let count = 4097
var ramp = [Float](repeating: 1, count: count * 4)
for i in 0..<count { for c in 0..<3 { ramp[i * 4 + c] = Float(i) / Float(count - 1) } }
let rampImage = ramp.withUnsafeBytes { CIImage(bitmapData: Data($0), bytesPerRow: count * 16, size: CGSize(width: count, height: 1), format: .RGBAf, colorSpace: nil) }
for threshold in [0.5, 0.75, 0.8, 0.9, 0.95] {
    let k = 1 - threshold
    let points = [(0.0, 0.0), (threshold, 0), (threshold + k * 0.4, 0.25), (threshold + k * 0.75, 0.7), (1, 1)]
    var parameters: [String: Any] = [:]
    for (i, point) in points.enumerated() { parameters["inputPoint\(i)"] = CIVector(x: point.0, y: point.1) }
    let result = rampImage.applyingFilter("CIToneCurve", parameters: parameters)
    var values = [Float](repeating: 0, count: count * 4)
    PixelAdjust.ciContext.render(result, toBitmap: &values, rowBytes: count * 16, bounds: CGRect(x: 0, y: 0, width: count, height: 1), format: .RGBAf, colorSpace: nil)
    let red = (0..<count).map { values[$0 * 4] }
    try red.withUnsafeBytes { try Data($0).write(to: root.appendingPathComponent("lens-threshold-\(Int(threshold * 100)).f32")) }
}
print("10 original Swift/Core Image/Metal lens references and 5 independent tone-curve ramps generated")
// Port Core Image's tone-curve transfer functions as a sampled table. These are reusable scalar
// functions, not images from the regression fixtures. Thresholds are spaced by 0.25 percentage
// points and inputs by 1/1024; the Windows implementation interpolates both axes.
if CommandLine.arguments.count > 2 {
    let destination = URL(fileURLWithPath: CommandLine.arguments[2])
    let n = 1025
    var input = [Float](repeating: 1, count: n * 4)
    for i in 0..<n { for c in 0..<3 { input[i * 4 + c] = Float(i) / Float(n - 1) } }
    let source = input.withUnsafeBytes { CIImage(bitmapData: Data($0), bytesPerRow: n * 16, size: CGSize(width: n, height: 1), format: .RGBAf, colorSpace: nil) }
    var tables = [Float]()
    for index in 0...181 {
        let t = 0.5 + Double(index) * 0.0025, k = 1 - t
        let points: [(Double, Double)] = index == 181 ? [(0.0, 0.0), (0.0625, 0.224), (0.125, 0.398), (0.25, 0.638), (1, 1)]
            : [(0.0, 0.0), (t, 0), (t + k * 0.4, 0.25), (t + k * 0.75, 0.7), (1, 1)]
        var parameters: [String: Any] = [:]
        for (i, point) in points.enumerated() { parameters["inputPoint\(i)"] = CIVector(x: point.0, y: point.1) }
        var values = [Float](repeating: 0, count: n * 4)
        PixelAdjust.ciContext.render(source.applyingFilter("CIToneCurve", parameters: parameters), toBitmap: &values, rowBytes: n * 16,
                                    bounds: CGRect(x: 0, y: 0, width: n, height: 1), format: .RGBAf, colorSpace: nil)
        tables.append(contentsOf: (0..<n).map { values[$0 * 4] })
    }
    try tables.withUnsafeBytes { try Data($0).write(to: destination) }
    print("182 scalar tone-curve tables generated for the portable renderer")
}
