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
for (name, both, blur, fringe, offset) in [("Prism-Crisp", false, 0.0, 0.0, 22.0), ("Prism-Both", true, 0.0, 30.0, 22.0), ("Prism-Blur", true, 80.0, 60.0, 22.0), ("Prism-ZeroOffset", false, 0.0, 100.0, 0.0)] {
    var s = LensEffectSettings(); s.effect = .prism; s.prismBothWays = both; s.prismBlur = blur; s.prismFringe = fringe; s.prismOffset = offset; s.prismAngle = 30
    cases.append((name, s))
}
for threshold in [96.0, 98.0, 100.0] {
    for effect in [LensEffect.anamorphicFlare, .starburst] { var s = LensEffectSettings(); s.effect = effect; s.threshold = threshold; cases.append(("\(effect.rawValue)-\(Int(threshold))", s)) }
}
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

// Probe source selection independently of the approximate streak resampler.
for threshold in [0.96,0.98,1.0] {
    let extent=CGRect(x:0,y:0,width:width,height:height)
    let opaque=CIImage(cgImage:image).unpremultiplyingAlpha().settingAlphaOne(in:extent)
    let selected=LensOptics.sources(opaque,threshold:threshold,side:CGFloat(max(width,height)))
    let cg=PixelAdjust.ciContext.createCGImage(selected,from:extent,format:.RGBAf,colorSpace:space)!
    var values=[Float](repeating:0,count:width*height*4)
    values.withUnsafeMutableBytes { ptr in
        let c=CGContext(data:ptr.baseAddress,width:width,height:height,bitsPerComponent:32,bytesPerRow:width*16,space:space,bitmapInfo:CGImageAlphaInfo.premultipliedLast.rawValue | CGBitmapInfo.floatComponents.rawValue | CGBitmapInfo.byteOrder32Little.rawValue)!
        c.setBlendMode(.copy);c.draw(cg,in:extent)
    }
    try values.withUnsafeBytes { try Data($0).write(to:root.appendingPathComponent("sources-\(Int(threshold*100)).f32")) }
}
// Reusable density gate transfer functions, sampled independently of the image fixtures.
let n = 1025
var ramp = [Float](repeating: 1, count: n * 4)
for i in 0..<n { for c in 0..<3 { ramp[i*4+c] = Float(i)/Float(n-1) } }
let source = ramp.withUnsafeBytes { CIImage(bitmapData: Data($0), bytesPerRow:n*16, size:CGSize(width:n,height:1), format:.RGBAf, colorSpace:nil) }
var tables = [Float]()
for index in 0...100 {
    let density = Double(index)/100, cut = 0.25+0.6*density, soft = 0.05
    let points = [(0.0,0.0),(max(0,cut-soft),0),(cut,0.5),(cut+soft,1),(max(1,cut+soft*2),1)]
    var parameters: [String:Any] = [:]
    for (i,p) in points.enumerated() { parameters["inputPoint\(i)"] = CIVector(x:p.0,y:p.1) }
    var values = [Float](repeating:0,count:n*4)
    PixelAdjust.ciContext.render(source.applyingFilter("CIToneCurve",parameters:parameters), toBitmap:&values,rowBytes:n*16,bounds:CGRect(x:0,y:0,width:n,height:1),format:.RGBAf,colorSpace:nil)
    tables.append(contentsOf:(0..<n).map { values[$0*4] })
}
try tables.withUnsafeBytes { try Data($0).write(to:URL(fileURLWithPath:CommandLine.arguments[2])) }
print("Mac 1.5.11 original lens fixtures and density tables generated")
