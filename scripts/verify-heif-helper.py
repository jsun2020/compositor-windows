"""Independent HIF1 and PNG reader for the bundled decoder; no system HEVC codec."""
import argparse,array,hashlib,json,struct,subprocess,sys,zlib
from pathlib import Path

def png_rgba(path):
    data=path.read_bytes();assert data[:8]==b'\x89PNG\r\n\x1a\n';at=8;packed=b''
    while at<len(data):
        n=struct.unpack('>I',data[at:at+4])[0];kind=data[at+4:at+8];part=data[at+8:at+8+n];assert zlib.crc32(kind+part)==struct.unpack('>I',data[at+8+n:at+12+n])[0]
        if kind==b'IHDR':w,h,depth,color,compression,filtering,interlace=struct.unpack('>IIBBBBB',part);assert (depth,color,compression,filtering,interlace)==(8,6,0,0,0)
        if kind==b'IDAT':packed+=part
        at+=12+n
    raw=zlib.decompress(packed);stride=w*4;assert len(raw)==h*(stride+1);out=bytearray();previous=bytearray(stride)
    def paeth(a,b,c):
        p=a+b-c;d=[abs(p-a),abs(p-b),abs(p-c)];return [a,b,c][d.index(min(d))]
    for y in range(h):
        start=y*(stride+1);mode=raw[start];assert 0<=mode<=4;row=bytearray(raw[start+1:start+1+stride])
        for x in range(stride):
            a=row[x-4] if x>=4 else 0;b=previous[x];c=previous[x-4] if x>=4 else 0
            row[x]=(row[x]+[0,a,b,(a+b)//2,paeth(a,b,c)][mode])&255
        out+=row;previous=row
    return w,h,out

def read_pixels(path):
    b=path.read_bytes();magic,w,h,bits,icc,source,reserved=struct.unpack('<4s6I',b[:28]);assert magic==b'HIF1' and bits==16 and reserved==0
    assert 0<w<=30000 and 0<h<=30000 and len(b)==28+icc+w*h*8 and 8<=source<=16
    a=array.array('H');a.frombytes(b[28+icc:]);
    if sys.byteorder!='little':a.byteswap()
    return w,h,source,b[28:28+icc],a

def main():
    p=argparse.ArgumentParser();p.add_argument('helper',type=Path);p.add_argument('output',type=Path);a=p.parse_args();a.output.mkdir(parents=True,exist_ok=False)
    fixtures=Path(__file__).resolve().parent.parent/'src-tauri/native/heif/tests';rows=[]
    for name,expected in [('rgb8',(48,32,8)),('gradient10',(512,64,10)),('rotated',(32,48,8)),('alpha',(48,32,8))]:
        source=fixtures/(name+'.heic');original=hashlib.sha256(source.read_bytes()).hexdigest();output=a.output/(name+'.hif1')
        subprocess.run([str(a.helper.resolve()),str(source),str(output.resolve()),'100000000'],check=True,capture_output=True,timeout=180)
        w,h,depth,icc,pixels=read_pixels(output);assert (w,h,depth)==expected and not icc
        rw,rh,reference=png_rgba(fixtures/(name+'-reference.png'));assert (w,h)==(rw,rh)
        diffs=[abs((v+128)//257-ref) for v,ref in zip(pixels,reference)]
        # Apple and libheif use different chroma siting at 4:2:0 boundaries. Keep
        # those differences visible, and compare stripe interiors for color/orientation.
        # The 4:4:4 10-bit gradient has no chroma-subsampling boundary exclusion.
        if name=='alpha':
            diffs=[abs(((v+128)//257)*((pixels[i//4*4+3]+128)//257)/255-reference[i]*reference[i//4*4+3]/255) if i%4!=3 else abs((v+128)//257-reference[i]) for i,v in enumerate(pixels)]
        selected=diffs if depth>8 or name=='alpha' else [v for i,v in enumerate(diffs) if ((i//4%w if name=='rgb8' else i//4//w)%8) in (3,4)]
        maximum=max(selected);mean=sum(selected)/len(selected)
        assert maximum<=4 and mean<=1.5,(name,maximum,mean)
        if name=='alpha':assert min(pixels[3::4])<3*257 and max(pixels[3::4])>252*257 and len(set(pixels[3::4]))>32
        else:assert set(pixels[3::4])=={65535}
        if depth>8:
            colors=[v for i,v in enumerate(pixels) if i%4!=3];assert len(set(colors))>256;assert sum(v%257!=0 for v in colors)>len(colors)//2
        assert hashlib.sha256(source.read_bytes()).hexdigest()==original
        rows.append({'case':name,'size':[w,h],'sourceBits':depth,'outputBits':16,'maxReferenceDifference':maximum,'meanReferenceDifference':mean,'comparisonRegion':'all pixels (premultiplied RGB and alpha)' if name=='alpha' else 'all pixels' if depth>8 else 'two center pixels per stripe; color and orientation','fullImageMaxDifference':max(diffs),'fullImageMeanDifference':sum(diffs)/len(diffs),'sha256':hashlib.sha256(output.read_bytes()).hexdigest()})
    repeat=a.output/'repeat.hif1';subprocess.run([str(a.helper.resolve()),str(fixtures/'rgb8.heic'),str(repeat.resolve()),'100000000'],check=True,capture_output=True,timeout=180)
    assert repeat.read_bytes()==(a.output/'rgb8.hif1').read_bytes()
    invalid=a.output/'invalid.heic';invalid.write_bytes(b'not a HEIF image')
    for name,source,budget in [('invalid',invalid,100000000),('budget',fixtures/'rgb8.heic',10)]:
        output=a.output/(name+'.hif1');r=subprocess.run([str(a.helper.resolve()),str(source.resolve()),str(output.resolve()),str(budget)],capture_output=True,timeout=180);assert r.returncode!=0 and not output.exists()
    protected=a.output/'protected.hif1';protected.write_bytes(b'preserve me');r=subprocess.run([str(a.helper.resolve()),str(fixtures/'rgb8.heic'),str(protected.resolve()),'100000000'],capture_output=True,timeout=180);assert r.returncode!=0 and protected.read_bytes()==b'preserve me'
    receipt={'passed':True,'scope':'Bundled HEVC decoding without Windows codecs; Apple reference pixel checks, true 10-bit detail, orientation, determinism, budget and invalid-input rejection','helperSHA256':hashlib.sha256(a.helper.read_bytes()).hexdigest(),'rows':rows}
    (a.output/'result.json').write_text(json.dumps(receipt,indent=2));print(json.dumps(receipt))
if __name__=='__main__':main()
