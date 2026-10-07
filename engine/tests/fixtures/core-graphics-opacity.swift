// Reproduce the synthetic oracle: xcrun swift core-graphics-opacity.swift
// Compositor 1.4.5's sRGB RGBA8 bitmap and CGContext setAlpha/draw path.
import Foundation
import CoreGraphics
let rgb = CGColorSpace(name: CGColorSpace.sRGB)!
let info = CGImageAlphaInfo.premultipliedLast.rawValue | CGBitmapInfo.byteOrder32Big.rawValue
func image(_ bytes: [UInt8]) -> CGImage {
    CGImage(width: 1, height: 1, bitsPerComponent: 8, bitsPerPixel: 32, bytesPerRow: 4, space: rgb,
            bitmapInfo: CGBitmapInfo(rawValue: info), provider: CGDataProvider(data: Data(bytes) as CFData)!,
            decode: nil, shouldInterpolate: false, intent: .defaultIntent)!
}
var rows = [[String: Any]]()
let backdrops: [[UInt8]] = [[35,100,220,255], [30,86,190,220], [71,82,93,127], [0,0,0,0]]
let sources: [[UInt8]] = [[166,85,25,180], [17,31,43,97], [101,101,101,255]]
for dst in backdrops {
    for src in sources {
        for opacity in [1,64,128,200,254,255] {
            for (name, mode) in [("Normal", CGBlendMode.normal), ("Multiply", .multiply), ("Screen", .screen)] {
                // The full-opacity Multiply integer fast path is outside the
                // covered-source fix; retain its separate diagnostic elsewhere.
                if name == "Multiply" && opacity == 255 { continue }
                let c = CGContext(data: nil, width: 1, height: 1, bitsPerComponent: 8, bytesPerRow: 4, space: rgb, bitmapInfo: info)!
                c.interpolationQuality = .none
                let rect = CGRect(x: 0, y: 0, width: 1, height: 1)
                c.setBlendMode(.copy); c.draw(image(dst), in: rect)
                c.setBlendMode(mode); c.setAlpha(Double(opacity) / 255); c.draw(image(src), in: rect)
                let p = c.data!.assumingMemoryBound(to: UInt8.self)
                rows.append(["backdrop": dst, "source": src, "opacityByte": opacity, "mode": name,
                             "result": Array(UnsafeBufferPointer(start: p, count: 4))])
            }
        }
    }
}
let data = try JSONSerialization.data(withJSONObject: rows, options: [.sortedKeys])
FileHandle.standardOutput.write(data)
