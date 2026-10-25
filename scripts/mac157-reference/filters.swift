import Foundation
import CoreGraphics
import CoreImage
// Uses the upstream PixelAdjust context options and FilterSettings' CIBloom call.
let root=URL(fileURLWithPath:CommandLine.arguments[1],isDirectory:true)
let w=64,h=48,space=CGColorSpace(name:CGColorSpace.sRGB)!
let context=CIContext(options:[.workingColorSpace:NSNull(),.outputColorSpace:NSNull()])
var input=[UInt8](repeating:0,count:w*h*4)
for y in 0..<h{for x in 0..<w{let a:UInt8=(x<8||y<5) ? 0:(x<20 ? 128:255),i=(y*w+x)*4
 input[i]=UInt8(Int(a)*((x*17+y*7)%256)/255);input[i+1]=UInt8(Int(a)*((x*3+y*13)%256)/255);input[i+2]=(x>27&&x<34&&y>18&&y<25) ? a:0;input[i+3]=a
}}
try Data(input).write(to:root.appendingPathComponent("bloom-input.rgba"))
let image=input.withUnsafeMutableBytes {bytes in CGContext(data:bytes.baseAddress,width:w,height:h,bitsPerComponent:8,bytesPerRow:w*4,space:space,bitmapInfo:CGImageAlphaInfo.premultipliedLast.rawValue)!.makeImage()!}
for (radius,amount) in [(2.0,25.0),(4.0,50.0),(8.0,100.0)]{
 let source=CIImage(cgImage:image)
 let result=source.applyingFilter("CIBloom",parameters:[kCIInputRadiusKey:radius,kCIInputIntensityKey:amount/50])
 let cg=context.createCGImage(result,from:source.extent,format:.RGBA8,colorSpace:space)!
 var output=[UInt8](repeating:0,count:w*h*4)
 output.withUnsafeMutableBytes {bytes in let c=CGContext(data:bytes.baseAddress,width:w,height:h,bitsPerComponent:8,bytesPerRow:w*4,space:space,bitmapInfo:CGImageAlphaInfo.premultipliedLast.rawValue)!;c.setBlendMode(.copy);c.draw(cg,in:CGRect(x:0,y:0,width:w,height:h))}
 try Data(output).write(to:root.appendingPathComponent("bloom-\(Int(radius))-\(Int(amount)).rgba"))
}
print("3 independent Core Image bloom references generated")
