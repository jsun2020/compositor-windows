"""Verify sensor development against the synthetic DNG with no embedded preview."""
import argparse,hashlib,json,struct,subprocess
from pathlib import Path

def main():
    p=argparse.ArgumentParser();p.add_argument('helper',type=Path);p.add_argument('dng',type=Path);p.add_argument('output',type=Path);a=p.parse_args();a.output.mkdir(parents=True,exist_ok=False)
    source_hash=hashlib.sha256(a.dng.read_bytes()).hexdigest()
    def execute(args):
        result=subprocess.run([str(a.helper.resolve()),*map(str,args)],capture_output=True,timeout=180,check=True)
        return json.loads(result.stdout)
    info=execute(['--inspect',a.dng.resolve()]);temp=info['asShotTemperature'];tint=info['asShotTint'];assert (info['width'],info['height'])==(128,96);assert info['sensorDecoded'] is False
    cases=[('as-shot',0,temp,tint,1),('as-shot-independent',0,temp,tint,1),('exposure-plus1',1,temp,tint,1),('temperature-4000',0,4000,tint,1),('tint-80',0,temp,80,1),('tone-zero',0,temp,tint,0)]
    images={};receipts=[]
    for name,exposure,temperature,tint,tone in cases:
        output=a.output/f'{name}.rgba';meta=execute(['--develop',a.dng.resolve(),output.resolve(),exposure,temperature,tint,tone,0]);data=output.read_bytes();assert data[:4]==b'RAW7';w,h,reserved=struct.unpack('<III',data[4:16]);assert (w,h,reserved)==(128,96,0);pixels=data[16:];assert len(pixels)==w*h*4;assert set(pixels[3::4])=={255};assert meta['sensorDecoded'] is True;images[name]=pixels
        means=[sum(pixels[c::4])/(w*h) for c in range(3)];receipts.append({'case':name,'metadata':meta,'sha256':hashlib.sha256(pixels).hexdigest(),'channelMeans':means})
        try:
            from PIL import Image
            Image.frombytes('RGBA',(w,h),pixels).save(a.output/f'{name}.png')
        except ImportError:pass
    assert images['as-shot']==images['as-shot-independent'],'As Shot must be deterministic'
    for name in ['exposure-plus1','temperature-4000','tint-80','tone-zero']:assert images[name]!=images['as-shot'],name
    assert sum(images['exposure-plus1'][0::4])>sum(images['as-shot'][0::4]),'positive exposure must brighten'
    invalid=a.output/'invalid.dng';invalid.write_bytes(b'not a camera RAW');result=subprocess.run([str(a.helper.resolve()),'--inspect',str(invalid.resolve())],capture_output=True,timeout=180);assert result.returncode!=0
    assert hashlib.sha256(a.dng.read_bytes()).hexdigest()==source_hash,'Source file changed'
    receipt={'status':'PASS','decoder':info['decoder'],'inputSHA256':source_hash,'helperSHA256':hashlib.sha256(a.helper.read_bytes()).hexdigest(),'sensorSize':[128,96],'inputHasEmbeddedPreview':False,'cases':receipts,'invalidFileExit':result.returncode}
    (a.output/'receipt.json').write_text(json.dumps(receipt,indent=2),encoding='utf-8');print(f"PASS: six sensor developments, repeatable As Shot, all controls, invalid input and unchanged source. {a.output}")
if __name__=='__main__':main()
