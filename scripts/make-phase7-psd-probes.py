"""Fresh, synthetic RGB8 PSD/PSB probes. No user documents are copied or changed."""
import argparse,json,struct
from pathlib import Path
U16=lambda n:struct.pack('>H',n)
U32=lambda n:struct.pack('>I',n)
F64=lambda n:struct.pack('>d',n)
def unicode(s):return U32(len(s.encode('utf-16-be'))//2)+s.encode('utf-16-be')
def identifier(s):return U32(0 if len(s)==4 else len(s))+s.encode('ascii')
def descriptor(fields):return unicode('')+identifier('null')+U32(len(fields))+b''.join(identifier(key)+kind.encode()+value for key,kind,value in fields)
def versioned(fields):return U32(16)+descriptor(fields)
def block(key,data):return b'8BIM'+key.encode()+U32(len(data))+data+b'\0'*(len(data)%2)
def plane(data,w,h,big):
    rows=[]
    for y in range(h):
        row=data[y*w:(y+1)*w];rows.append(b''.join(bytes([len(row[x:x+128])-1])+row[x:x+128] for x in range(0,w,128)))
    return U16(1)+b''.join((U32 if big else U16)(len(row)) for row in rows)+b''.join(rows)
def write(path,big,layers):
    w,h=512,320;length=lambda n:struct.pack('>Q',n) if big else U32(n)
    records=[];planes=[]
    for layer in layers:
        bounds=layer.get('bounds',(0,0,w,h));left,top,lw,lh=bounds;section=layer.get('section',0)
        pixel=layer.get('pixels',b'');channels=[]
        if pixel:
            for channel,id_ in [(3,-1),(0,0),(1,1),(2,2)]:channels.append((id_,plane(pixel[channel::4],lw,lh,big)))
        mask=layer.get('mask');extra=b''
        if mask is not None:
            channels.append((-2,plane(mask,lw,lh,big)));extra+=U32(20)+b''.join(U32(n) for n in [top,left,top+lh,left+lw])+bytes([255,0,0,0])
        else:extra+=U32(0)
        extra+=U32(0)+bytes(4)+block('luni',unicode(layer['name']))
        if section:extra+=block('lsct',U32(section))
        extra+=layer.get('extra',b'')
        record=b''.join(U32(n) for n in [top,left,top+lh,left+lw])+U16(len(channels))+b''.join(U16(id_&65535)+length(len(data)) for id_,data in channels)
        record+=b'8BIM'+layer.get('blend',b'pass' if section else b'norm')+bytes([layer.get('opacity',255),layer.get('clip',0),2 if layer.get('hidden') else 0,0])+U32(len(extra))+extra
        records.append(record);planes.extend(data for _,data in channels)
    layerinfo=U16(len(layers))+b''.join(records)+b''.join(planes);layerinfo+=b'\0'*(len(layerinfo)%2)
    section=length(len(layerinfo))+layerinfo+U32(0)
    header=b'8BPS'+U16(2 if big else 1)+bytes(6)+U16(3)+U32(h)+U32(w)+U16(8)+U16(3)+U32(0)+U32(0)
    merged=bytes([230])*(w*h*3)
    path.write_bytes(header+length(len(section))+section+U16(0)+merged)
def solid(w,h,color):return bytes(color)*(w*h)
def main():
    p=argparse.ArgumentParser();p.add_argument('output',type=Path);a=p.parse_args();a.output.mkdir(parents=True,exist_ok=False)
    background={'name':'Background','pixels':solid(512,320,[230,230,230,255])}
    mask=bytes(255 if x<150 else 80 for y in range(120) for x in range(300))
    base={'name':'Masked blue 基底','bounds':(40,50,300,120),'pixels':solid(300,120,[35,100,220,220]),'mask':mask}
    clip={'name':'Clipped orange','bounds':(100,100,270,120),'pixels':solid(270,120,[235,120,35,180]),'clip':1,'blend':b'mul ','opacity':200}
    # File order matches the Mac 1.4.5 oracle: bottom-to-top, divider before children/folder.
    layers=[background,{'name':'Hidden green','bounds':(20,20,90,90),'pixels':solid(90,90,[20,220,60,255]),'hidden':True},{'name':'Folder end','section':3},base,clip,{'name':'Folder','section':1}]
    for big in [False,True]:write(a.output/('01-groups-masks-clipping.'+('psb' if big else 'psd')),big,layers)
    color=descriptor([('Rd  ','doub',F64(240)),('Grn ','doub',F64(70)),('Bl  ','doub',F64(40))]);fill=block('SoCo',versioned([('Clr ','Objc',color)]))
    box=descriptor([(key,'UntF',b'#Pxl'+F64(n)) for key,n in [('Left',50),('Top ',50),('Rght',250),('Btom',180)]])
    origin=block('vogk',U32(1)+versioned([('keyOriginType','long',U32(1)),('keyOriginShapeBBox','Objc',box)]))
    engine=b'<< /ResourceDict << /FontSet [ << /Name (ArialMT) >> ] >> /EngineDict << /StyleRun << /RunArray [ << /StyleSheet << /StyleSheetData << /Font 0 /FontSize 28 /FillColor << /Values [ 1 0.1 0.1 0.1 ] >> >> >> >> ] >> /ParagraphRun << /RunArray [ << /ParagraphSheet << /Properties << /Justification 0 >> >> >> ] >> >> >>'
    tysh=U16(1)+b''.join(F64(n) for n in [1,0,0,1,50,260])+U16(50)+versioned([('Txt ','TEXT',unicode('Editable ABC\r')),('EngineData','tdta',U32(len(engine))+engine)])+U16(1)+versioned([])+b''.join(U32(n) for n in [50,230,260,270])
    # A real cached text raster helps inspect the first-style fallback; Pillow is only a generator dependency.
    from PIL import Image,ImageDraw,ImageFont
    image=Image.new('RGBA',(260,45));draw=ImageDraw.Draw(image);draw.text((0,0),'Editable ABC',font=ImageFont.truetype('C:/Windows/Fonts/arial.ttf',28),fill=(25,25,25,255))
    text={'name':'Editable text','bounds':(50,234,260,45),'pixels':image.tobytes(),'extra':block('TySh',tysh)}
    shape={'name':'Editable rectangle','bounds':(50,50,200,130),'pixels':solid(200,130,[240,70,40,255]),'extra':fill+origin}
    write(a.output/'02-editable-text-shape.psd',False,[background,shape,text])
    levels=U16(2)+b''.join(U16(n) for _ in range(29) for n in [0,255,0,255,100])
    write(a.output/'03-adjustment-conversions.psd',False,[background,{'name':'Levels','extra':block('levl',levels)},{'name':'Unsupported posterize','extra':block('post',U16(3))}])
    (a.output/'README.json').write_text(json.dumps({'size':[512,320],'files':{'01-groups-masks-clipping.psd':'groups, Unicode, clipping, visibility, opacity, mask, Multiply','01-groups-masks-clipping.psb':'same semantics, PSB 64-bit lengths and 32-bit PackBits rows','02-editable-text-shape.psd':'ArialMT point text and live rectangle','03-adjustment-conversions.psd':'editable neutral Levels and explicit skipped Posterize notice'}},indent=2),encoding='utf8')
    print('Created four synthetic Photoshop probes:',a.output)
if __name__=='__main__':main()
