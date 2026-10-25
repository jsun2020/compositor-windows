// Driver only. The geometry is compiled verbatim from frozen upstream ShapeTool.swift.
let root=URL(fileURLWithPath:CommandLine.arguments[1],isDirectory:true)
try FileManager.default.createDirectory(at:root,withIntermediateDirectories:true)
var cases=[[String:Any]]()
func render(_ spec:[String:Any],bounds:CGRect,_ draw:(CGContext)->Void) throws {
 let w=Int(bounds.width),h=Int(bounds.height);var data=[UInt8](repeating:0,count:w*h*4)
 data.withUnsafeMutableBytes { bytes in
  let ctx=CGContext(data:bytes.baseAddress,width:w,height:h,bitsPerComponent:8,bytesPerRow:w*4,space:CGColorSpace(name:CGColorSpace.sRGB)!,bitmapInfo:CGImageAlphaInfo.premultipliedLast.rawValue)!
  ctx.translateBy(x:0,y:CGFloat(h));ctx.scaleBy(x:1,y:-1)
  ctx.setFillColor(CGColor(srgbRed:1,green:1,blue:1,alpha:1));ctx.setStrokeColor(CGColor(srgbRed:1,green:1,blue:1,alpha:1));draw(ctx)
 }
 let name="shape-\(cases.count).rgba";try Data(data).write(to:root.appendingPathComponent(name))
 cases.append(["spec":spec,"bounds":[bounds.minX,bounds.minY,bounds.width,bounds.height],"width":w,"height":h,"file":name])
}
for (w,h,r) in [(80.0,60.0,12.0),(100,40,18),(41,41,50),(24,20,1)] {
 let rect=CGRect(x:0,y:0,width:w,height:h)
 try render(["kind":"Rectangle","rect":["x":0,"y":0,"width":w,"height":h],"cornerRadius":r,"smoothing":true],bounds:rect){ctx in ctx.addPath(ShapeKind.rectangle.path(in:rect,cornerRadius:r,smoothing:true));ctx.fillPath()}
}
for cap in ShapeLineCap.allCases {for style in ShapeLineStyle.allCases {for width in [1.0,5.0] {for (x,y) in [(60.0,0.0),(51.0,31.0),(0.0,0.0)] {
 let a=CGPoint(x:10.5,y:20.5),b=CGPoint(x:10.5+x,y:20.5+y)
 let reach=cap.reach(from:a,to:b,width:width)
 let bounds=CGRect(x:a.x-reach,y:a.y-reach,width:x+reach*2,height:y+reach*2)
 try render(["kind":"Line","start":[a.x,a.y],"end":[b.x,b.y],"width":width,"lineStyle":style.rawValue,"lineCap":cap.rawValue],bounds:bounds){ctx in
  ctx.strokeShapeLine(from:CGPoint(x:a.x-bounds.minX,y:a.y-bounds.minY),to:CGPoint(x:b.x-bounds.minX,y:b.y-bounds.minY),width:width,style:style,cap:cap)
 }
}}}}
try JSONSerialization.data(withJSONObject:cases,options:[.prettyPrinted,.sortedKeys]).write(to:root.appendingPathComponent("shapes.json"))
print("\(cases.count) upstream Core Graphics shape references generated")
