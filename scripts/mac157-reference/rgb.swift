import Foundation
import CoreGraphics
import Metal
import CoreImage
enum ExportError: Error {case render}
let root=URL(fileURLWithPath:CommandLine.arguments[1],isDirectory:true)
try FileManager.default.createDirectory(at:root,withIntermediateDirectories:true)
let width=64,height=48
var input=[UInt8](repeating:0,count:width*height*4)
for y in 0..<height {for x in 0..<width {
 let a:UInt8=(x<6 || y<4) ? 0:255
 let at=(y*width+x)*4
 input[at]=(x>24 && x<40) ? a:UInt8(Int(a)*x/width)
 input[at+1]=UInt8(Int(a)*y/height);input[at+2]=UInt8(Int(a)*(x+y)/(width+height));input[at+3]=a
}}
try Data(input).write(to:root.appendingPathComponent("rgb-input.rgba"))
let gpu=MetalDispersion.shared!
for (name,distance,blur,angle,radial) in [("linear",Float(8),Float(0),Float(0),false),("smear",0,24,0,false),("radial",12,8,0,true),("angled",10,12,Float.pi/4,false)] {
 var pixels=input
 try pixels.withUnsafeMutableBytes { bytes in
  let context=CGContext(data:bytes.baseAddress,width:width,height:height,bitsPerComponent:8,bytesPerRow:width*4,space:CGColorSpace(name:CGColorSpace.sRGB)!,bitmapInfo:CGImageAlphaInfo.premultipliedLast.rawValue)!
  let factor:Float=radial ? 2:1
  try gpu.run(context,MetalDispersion.Parameters(center:SIMD2(32,24),direction:SIMD2(cos(angle),-sin(angle)),distance:distance,blur:blur,reach:32,radial:radial ? 1:0,sides:0,spectral:distance>0 ? UInt32(min(32,max(3,Int(distance*factor)))):1,smear:blur>0 ? UInt32(min(24,max(2,Int(blur*factor/2)))):1))
 }
 try Data(pixels).write(to:root.appendingPathComponent("rgb-\(name).rgba"))
}
print("4 upstream Metal RGB Split references generated")
let device=MTLCreateSystemDefaultDevice()!,queue=device.makeCommandQueue()!
let library=try device.makeLibrary(source:"""
#include <metal_stdlib>
using namespace metal;
kernel void noise(device float *out [[buffer(0)]], uint2 p [[thread_position_in_grid]]) {
 if(p.x<64 && p.y<48) out[p.y*64+p.x]=fract(sin(dot(float2(p),float2(12.9898,78.233)))*43758.5453);
}
""",options:nil)
let pipeline=try device.makeComputePipelineState(function:library.makeFunction(name:"noise")!)
let buffer=device.makeBuffer(length:64*48*4,options:.storageModeShared)!,command=queue.makeCommandBuffer()!,encoder=command.makeComputeCommandEncoder()!
encoder.setComputePipelineState(pipeline);encoder.setBuffer(buffer,offset:0,index:0)
encoder.dispatchThreads(MTLSize(width:64,height:48,depth:1),threadsPerThreadgroup:MTLSize(width:16,height:16,depth:1));encoder.endEncoding();command.commit();command.waitUntilCompleted()
try Data(bytes:buffer.contents(),count:buffer.length).write(to:root.appendingPathComponent("rgb-jitter.f32"))
