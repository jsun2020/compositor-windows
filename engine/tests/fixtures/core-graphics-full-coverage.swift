import Foundation
import CoreGraphics

let rgb = CGColorSpace(name: CGColorSpace.sRGB)!
let premul = CGImageAlphaInfo.premultipliedLast.rawValue | CGBitmapInfo.byteOrder32Big.rawValue
func image(_ bytes: [UInt8], width: Int, height: Int, info: UInt32 = premul) -> CGImage {
    CGImage(width: width, height: height, bitsPerComponent: 8, bitsPerPixel: 32,
        bytesPerRow: width * 4, space: rgb, bitmapInfo: CGBitmapInfo(rawValue: info),
        provider: CGDataProvider(data: Data(bytes) as CFData)!, decode: nil,
        shouldInterpolate: false, intent: .defaultIntent)!
}
func bitmap() -> CGContext {
    let c = CGContext(data: nil, width: 1, height: 1, bitsPerComponent: 8,
        bytesPerRow: 4, space: rgb, bitmapInfo: premul)!
    c.interpolationQuality = .none
    return c
}
func read(_ c: CGContext) -> [UInt8] {
    Array(UnsafeBufferPointer(start: c.data!.assumingMemoryBound(to: UInt8.self), count: 4))
}
let pixelRect = CGRect(x: 0, y: 0, width: 1, height: 1)
func deduplicate(_ tuples: [[UInt8]]) -> [[UInt8]] {
    var seen = Set<String>()
    return tuples.filter { seen.insert($0.map { String($0) }.joined(separator: ",")).inserted }
}
var levels = [[UInt8]]()
for alpha in [0, 1, 64, 97, 127, 180, 220, 254, 255] {
    let a = UInt8(alpha)
    levels += [[0,0,0,a], [a,a,a,a], [UInt8((alpha+2)/4),UInt8((alpha+1)/2),UInt8((alpha*3+2)/4),a],
        [a,0,UInt8((alpha+1)/3),a]]
}
let backdrops = deduplicate([[35,100,220,255], [30,86,190,220], [71,82,93,127], [0,0,0,0]] + levels)
let sources = deduplicate([[166,85,25,180], [17,31,43,97], [101,101,101,255]] + levels)
var materialized = [[String: Any]](), blends = [[String: Any]]()
for tuple in deduplicate(backdrops + sources) {
    let c = bitmap()
    c.setBlendMode(.copy); c.draw(image(tuple, width: 1, height: 1), in: pixelRect)
    materialized.append(["input": tuple, "copied": read(c)])
}
for dst in backdrops { for src in sources { for opacity in [254,255] {
    for (name, mode) in [("Normal", CGBlendMode.normal), ("Multiply", .multiply), ("Screen", .screen)] {
        let c = bitmap()
        c.setBlendMode(.copy); c.draw(image(dst, width: 1, height: 1), in: pixelRect)
        let initial = read(c)
        c.setBlendMode(mode); c.setAlpha(Double(opacity)/255)
        c.draw(image(src, width: 1, height: 1), in: pixelRect)
        blends.append(["backdrop": dst, "initialBackdrop": initial, "source": src,
            "opacityByte": opacity, "mode": name, "result": read(c)])
    }
} } }

let data = try JSONSerialization.data(withJSONObject: blends, options: [.sortedKeys])
try data.write(to: URL(fileURLWithPath: CommandLine.arguments[1]), options: .withoutOverwriting)
print("Captured \(blends.count) independent blend rows.")
