// Reproduce: xcrun swift core-image-geometry-ramp.swift > new-oracle.json
import Foundation
import CoreGraphics
import CoreImage

let rgb = CGColorSpace(name: CGColorSpace.sRGB)!
let straight = CGImageAlphaInfo.last.rawValue | CGBitmapInfo.byteOrder32Big.rawValue
let premul = CGImageAlphaInfo.premultipliedLast.rawValue | CGBitmapInfo.byteOrder32Big.rawValue
let context = CIContext(options: [.workingColorSpace: NSNull(), .outputColorSpace: NSNull()])
var bytes = [UInt8]()
for _ in 0..<4 { for x in 0..<256 { bytes += [UInt8(x), UInt8(x), UInt8(x), 255] } }
let image = CGImage(width: 256, height: 4, bitsPerComponent: 8, bitsPerPixel: 32,
    bytesPerRow: 1024, space: rgb, bitmapInfo: CGBitmapInfo(rawValue: straight),
    provider: CGDataProvider(data: Data(bytes) as CFData)!, decode: nil,
    shouldInterpolate: false, intent: .defaultIntent)!
var rows = [[String: Any]]()
for phase in [0, 64, 96, 112, 120, 124, 127, 128, 129, 132, 136, 144, 160, 192, 256] {
    let shift = Double(phase) / 256
    let warped = CIImage(cgImage: image).applyingFilter("CIPerspectiveTransform", parameters: [
        "inputTopLeft": CIVector(x: shift, y: 4),
        "inputTopRight": CIVector(x: 256 + shift, y: 4),
        "inputBottomRight": CIVector(x: 256 + shift, y: 0),
        "inputBottomLeft": CIVector(x: shift, y: 0)])
    let result = context.createCGImage(warped, from: CGRect(x: 0, y: 0, width: 256, height: 4),
        format: .RGBA8, colorSpace: rgb)!
    let bitmap = CGContext(data: nil, width: 256, height: 4, bitsPerComponent: 8,
        bytesPerRow: 1024, space: rgb, bitmapInfo: premul)!
    bitmap.interpolationQuality = .none
    bitmap.draw(result, in: CGRect(x: 0, y: 0, width: 256, height: 4))
    let data = bitmap.data!.assumingMemoryBound(to: UInt8.self)
    let red = (0..<256).map { data[1024 + $0 * 4] }
    let alpha = (0..<256).map { data[1027 + $0 * 4] }
    for x in 0..<256 {
        precondition(data[1025 + x * 4] == red[x] && data[1026 + x * 4] == red[x])
    }
    rows.append(["phase256": phase, "red": red, "alpha": alpha])
}
FileHandle.standardOutput.write(try JSONSerialization.data(withJSONObject: rows, options: [.sortedKeys]))
