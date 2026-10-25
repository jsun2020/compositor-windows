"""Independent Pillow/LittleCMS and PDF checks for native Windows CMYK exports."""
from pathlib import Path
from PIL import Image,ImageCms
import sys,json,hashlib
out,fixtures=map(Path,sys.argv[1:]);icc=(fixtures/'test-print.icc').read_bytes();rgb=Image.open(fixtures/'rgb.png').convert('RGB');srgb=ImageCms.createProfile('sRGB');profile=ImageCms.ImageCmsProfile(str(fixtures/'test-print.icc'))
expected=ImageCms.profileToProfile(rgb,srgb,profile,outputMode='CMYK',renderingIntent=1)
rows=[]
for ext in ['tif','jpg']:
    im=Image.open(out/f'export.{ext}');assert im.mode=='CMYK' and im.size==rgb.size;assert im.info['icc_profile']==icc
    assert all(abs(d-300)<.1 for d in im.info['dpi']);diff=[]
    for x in [3,11,19,27,35,43]:
        for y in range(8,24):diff.extend(abs(a-b) for a,b in zip(im.getpixel((x,y)),expected.getpixel((x,y))))
    maximum=max(diff);mean=sum(diff)/len(diff);assert maximum<=12 and mean<=3,(ext,maximum,mean)
    rows.append({'format':ext,'maxInkDifference':maximum,'meanInkDifference':mean})
# PyMuPDF independently parses the PDF object tree, image channels and print size.
import fitz
pdf=fitz.open(out/'export.pdf');assert len(pdf)==1;page=pdf[0];assert abs(page.rect.width-48*72/300)<.001 and abs(page.rect.height-32*72/300)<.001
images=page.get_images(full=True);assert len(images)==1;image=images[0];assert image[2:5]==(48,32,8);assert image[5] in ('ICCBased','DeviceCMYK')
raw=(out/'export.pdf').read_bytes();assert icc in raw and b'/N 4' in raw
render=page.get_pixmap(matrix=fitz.Matrix(300/72,300/72),alpha=False);assert render.width==48 and render.height==32
rows.append({'format':'pdf','pagePoints':[page.rect.width,page.rect.height],'embeddedICC':hashlib.sha256(icc).hexdigest(),'renderedPixels':[render.width,render.height]})
# ICC proof lookup uses exactly the same blue/green/red nesting as the renderer.
lut=(out/'proof-lut.rgb').read_bytes();assert len(lut)==33**3*3
points=[(round(r*255/32),round(g*255/32),round(b*255/32)) for b in range(33) for g in range(33) for r in range(33)]
image=Image.new('RGB',(33**3,1));image.putdata(points)
inks=ImageCms.profileToProfile(image,srgb,profile,outputMode='CMYK',renderingIntent=1)
reference=ImageCms.profileToProfile(inks,profile,srgb,outputMode='RGB',renderingIntent=1).tobytes()
diff=[abs(a-b) for a,b in zip(lut,reference)];assert max(diff)<=16 and sum(diff)/len(diff)<=3,('proof lookup',max(diff),sum(diff)/len(diff))
rows.append({'format':'proof','maxRGBDifference':max(diff),'meanRGBDifference':sum(diff)/len(diff)})
(out/'independent-reader.json').write_text(json.dumps({'passed':True,'rows':rows},indent=2),encoding='utf-8');print(json.dumps(rows))
