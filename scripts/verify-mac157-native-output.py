"""Independently inspect files saved by the production WebView2 UI."""
import sys,json
from pathlib import Path
from PIL import Image,ImageCms
import fitz
out,fixtures=map(Path,sys.argv[1:]);rows=[]
def package(name):
    p=out/(name+'.comp');m=json.loads((p/'manifest.json').read_text(encoding='utf-8-sig'));return p,m
p,m=package('pages');assert (m['width'],m['height'],m['resolution'])==(300,200,300);assert [x['name'] for x in m['layers']]==['Page 2','Page 1'];rows.append('PDF page stack and print resolution')
p,m=package('heif');assert m['version']==12
images=list((p/'images').glob('*.png'));assert len(images)==1;b=images[0].read_bytes();assert b[:8]==b'\x89PNG\r\n\x1a\n' and b[24]==16;assert Image.open(images[0]).size==(48,32);rows.append('HEIF saved PNG header has 16-bit samples')
p,m=package('cmyk-psd');assert (m['width'],m['height'])==(48,32);assert len(m['layers'])==1
icc=(fixtures/'test-print.icc').read_bytes();profile=ImageCms.ImageCmsProfile(str(fixtures/'test-print.icc'));srgb=ImageCms.createProfile('sRGB');rgb=Image.open(fixtures/'rgb.png').convert('RGB');expected=ImageCms.profileToProfile(rgb,srgb,profile,outputMode='CMYK',renderingIntent=1)
for ext in ['tif','jpg']:
    im=Image.open(out/f'print.{ext}');assert im.mode=='CMYK' and im.size==(48,32);assert im.info['icc_profile']==icc;assert all(abs(d-72)<.1 for d in im.info['dpi']);diff=[]
    for x in [3,11,19,27,35,43]:
        for y in range(8,24):diff.extend(abs(a-b) for a,b in zip(im.getpixel((x,y)),expected.getpixel((x,y))))
    assert max(diff)<=12 and sum(diff)/len(diff)<=3,(ext,max(diff),sum(diff)/len(diff));rows.append({'format':ext,'maxInkDifference':max(diff),'meanInkDifference':sum(diff)/len(diff)})
pdf=fitz.open(out/'print.pdf');assert len(pdf)==1 and abs(pdf[0].rect.width-48)<.001 and abs(pdf[0].rect.height-32)<.001;assert icc in (out/'print.pdf').read_bytes();assert pdf[0].get_images()[0][2:5]==(48,32,8);assert pdf[0].get_pixmap().width==48;rows.append('CMYK PDF embedded ICC and physical size')
p,m=package('animation');assert m['version']==17;gif=Image.open(out/'animation.gif');assert gif.n_frames==4;delays=[]
for i in range(gif.n_frames):gif.seek(i);delays.append(gif.info['duration'])
assert delays==[100,450,450,300],delays;rows.append({'format':'gif','durationsMs':delays})
(out/'independent-reader.json').write_text(json.dumps({'passed':True,'rows':rows},indent=2),encoding='utf-8');print(json.dumps(rows))
