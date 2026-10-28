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
let ci=CIImage(cgImage:image), extent=CGRect(x:0,y:0,width:width,height:height)
for radius in [1.0,4.0,12.0] { for amount in [50.0,100.0,300.0] {for clamp in [false,true] {
    let input=clamp ? ci.clampedToExtent() : ci
    let result=input.applyingFilter("CIUnsharpMask",parameters:[kCIInputRadiusKey:radius,kCIInputIntensityKey:amount/100]).cropped(to:extent)
    let output=PixelAdjust.ciContext.createCGImage(result,from:extent,format:.RGBA8,colorSpace:space)!
    var bytes=[UInt8](repeating:0,count:width*height*4)
    bytes.withUnsafeMutableBytes { ptr in
        let c=CGContext(data:ptr.baseAddress,width:width,height:height,bitsPerComponent:8,bytesPerRow:width*4,space:space,bitmapInfo:CGImageAlphaInfo.premultipliedLast.rawValue)!
        c.setBlendMode(.copy);c.draw(output,in:extent)
    }
    try Data(bytes).write(to:root.appendingPathComponent("sharpen-\(Int(radius))-\(Int(amount))-\(clamp).rgba"))
}}}
@_silgen_name("adjust_colored_vignette")
func vignette(_ p: UnsafeMutablePointer<UInt8>,_ w:Int,_ h:Int,_ stride:Int,_ x:Double,_ y:Double,_ fw:Double,_ fh:Double,_ fill:Int32,_ amount:Double,_ midpoint:Double,_ roundness:Double,_ feather:Double,_ highlights:Double,_ r:Double,_ g:Double,_ b:Double)
@_silgen_name("adjust_tonal_contrast")
func tonal(_ p:UnsafeMutablePointer<UInt8>,_ blurred:UnsafePointer<UInt8>,_ w:Int,_ h:Int,_ stride:Int,_ blurStride:Int,_ amount:Double,_ shadows:Double,_ midtones:Double,_ highlights:Double)
for amount in [0.0,35.0,100.0] {for roundness in [-100.0,0.0,100.0] {
    var bytes=input
    vignette(&bytes,width,height,width*4,0,0,Double(width),Double(height),0,amount,50,roundness,60,25,0.2,0.1,0.3)
    try Data(bytes).write(to:root.appendingPathComponent("vignette-\(Int(amount))-\(Int(roundness)).rgba"))
}}
for radius in [1.0,16.0,100.0] {for amount in [50.0,100.0] {
    let blurred=ci.applyingGaussianBlur(sigma:radius).cropped(to:extent)
    let image=PixelAdjust.ciContext.createCGImage(blurred,from:extent,format:.RGBA8,colorSpace:space)!
    var base=[UInt8](repeating:0,count:width*height*4), bytes=input
    base.withUnsafeMutableBytes { ptr in
        let c=CGContext(data:ptr.baseAddress,width:width,height:height,bitsPerComponent:8,bytesPerRow:width*4,space:space,bitmapInfo:CGImageAlphaInfo.premultipliedLast.rawValue)!
        c.setBlendMode(.copy);c.draw(image,in:extent)
    }
    try Data(base).write(to:root.appendingPathComponent("tonal-blurred-\(Int(radius)).rgba"))
    tonal(&bytes,base,width,height,width*4,width*4,amount,40,60,30)
    try Data(bytes).write(to:root.appendingPathComponent("tonal-\(Int(radius))-\(Int(amount)).rgba"))
}}
