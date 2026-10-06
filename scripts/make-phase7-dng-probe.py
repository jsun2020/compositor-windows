"""Make a tiny uncompressed Bayer DNG with no thumbnail/preview image.

The decoder must demosaic the sensor values. Outputs are generated acceptance
artifacts, not photographs or repository fixtures. Never overwrite an input.
"""
import argparse, json, math, struct
from pathlib import Path

def main():
    parser=argparse.ArgumentParser();parser.add_argument('output',type=Path);args=parser.parse_args()
    width,height=128,96
    samples=[]
    colors=[(50000,12000,8000),(10000,48000,14000),(9000,14000,50000),(28000,28000,28000)]
    for y in range(height):
        for x in range(width):
            channel=((y%2)*2+x%2);channel=[0,1,1,2][channel]
            color=colors[min(3,x//32)]
            level=color[channel]*(.3+.7*y/(height-1))
            samples.append(int(level))
    pixels=struct.pack('<'+'H'*len(samples),*samples)
    tags=[]
    def tag(key,kind,values):
        if isinstance(values,str):values=values.encode('ascii')+b'\0'
        if isinstance(values,bytes):count=len(values);data=values
        elif kind==3:count=len(values);data=struct.pack('<'+'H'*count,*values)
        elif kind==4:count=len(values);data=struct.pack('<'+'I'*count,*values)
        elif kind in (5,10):count=len(values);data=b''.join(struct.pack('<ii' if kind==10 else '<II',*v) for v in values)
        else:raise ValueError(kind)
        tags.append([key,kind,count,data])
    tag(256,4,[width]);tag(257,4,[height]);tag(258,3,[16]);tag(259,3,[1]);tag(262,3,[32803]);tag(271,2,'Compositor test');tag(272,2,'Bayer probe');tag(273,4,[0]);tag(274,3,[1]);tag(277,3,[1]);tag(278,4,[height]);tag(279,4,[len(pixels)]);tag(284,3,[1]);tag(33421,3,[2,2]);tag(33422,1,bytes([0,1,1,2]));tag(50706,1,bytes([1,4,0,0]));tag(50707,1,bytes([1,1,0,0]));tag(50708,2,'Compositor Bayer test');tag(50710,1,bytes([0,1,2]));tag(50711,3,[1]);tag(50714,5,[(0,1)]);tag(50717,4,[65535]);tag(50721,10,[(1,1),(0,1),(0,1),(0,1),(1,1),(0,1),(0,1),(0,1),(1,1)]);tag(50728,5,[(1,2),(1,1),(2,3)]);tag(50778,3,[21]);tag(50829,4,[0,0,height,width])
    tags.sort(key=lambda t:t[0]);offset=8+2+len(tags)*12+4;extra=bytearray();entries=[]
    for key,kind,count,data in tags:
        if len(data)>4:
            field=struct.pack('<I',offset+len(extra));extra.extend(data)
            if len(extra)%2:extra.append(0)
        else:field=data.ljust(4,b'\0')
        entries.append([key,struct.pack('<HHI',key,kind,count)+field])
    pixel_offset=offset+len(extra)
    for row in entries:
        if row[0]==273:row[1]=row[1][:8]+struct.pack('<I',pixel_offset)
    result=b'II'+struct.pack('<HIH',42,8,len(tags))+b''.join(row[1] for row in entries)+b'\0'*4+extra+pixels
    args.output.parent.mkdir(parents=True,exist_ok=True)
    with args.output.open('xb') as f:f.write(result)
    print(json.dumps({'path':str(args.output),'sensorSize':[width,height],'embeddedPreview':False,'bytes':len(result)}))
if __name__=='__main__':main()
