"""Validate the production RAW8/RAW7 protocol with actual sensor decoding."""
import argparse,array,hashlib,json,struct,subprocess,sys
from pathlib import Path

def main():
    p=argparse.ArgumentParser();p.add_argument('helper',type=Path);p.add_argument('input',type=Path);p.add_argument('output',type=Path);a=p.parse_args();a.output.mkdir(parents=True,exist_ok=False)
    original=hashlib.sha256(a.input.read_bytes()).hexdigest()
    def run(args):
        r=subprocess.run([str(a.helper.resolve()),*map(str,args)],capture_output=True,timeout=180,check=True);return json.loads(r.stdout)
    info=run(['--inspect',a.input.resolve()]);rows=[];hashes={};means={}
    cases=[('as-shot',0,info['asShotTemperature'],info['asShotTint'],1,0,0,0),('repeat',0,info['asShotTemperature'],info['asShotTint'],1,0,0,0),('exposure',1,info['asShotTemperature'],info['asShotTint'],1,0,0,0),('temperature',0,4000,info['asShotTint'],1,0,0,0),('tint',0,info['asShotTemperature'],80,1,0,0,0),('tone-zero',0,info['asShotTemperature'],info['asShotTint'],0,0,0,0),('preview',0,info['asShotTemperature'],info['asShotTint'],1,1,0,0),('recovery-denoise',0,info['asShotTemperature'],info['asShotTint'],1,0,.5,1)]
    for name,exp,temp,tint,tone,preview,noise,recovery in cases:
        file=a.output/(name+'.rgba');meta=run(['--develop',a.input.resolve(),file.resolve(),exp,temp,tint,tone,preview,noise,recovery]);data=file.read_bytes();magic=data[:4];w,h,bits=struct.unpack('<III',data[4:16]);assert w>0 and h>0;assert meta['sensorDecoded'] is True
        if preview:
            assert magic==b'RAW7' and bits==0 and len(data)==16+w*h*4;assert set(data[19::4])=={255};means[name]=sum(data[16::4])/(w*h)
        else:
            assert magic==b'RAW8' and bits==16 and len(data)==16+w*h*8;assert (w,h)==(info['width'],info['height'])
            samples=array.array('H');samples.frombytes(data[16:]);
            if sys.byteorder!='little':samples.byteswap()
            assert min(samples[3::4])==max(samples[3::4])==65535
            assert any(v%257 for v in samples[0::4]),'Full import was expanded from eight bits'
            means[name]=sum(samples[0::4])/(w*h)/257
            del samples
        hashes[name]=hashlib.sha256(data[16:]).hexdigest();rows.append({'case':name,'width':w,'height':h,'bits':8 if preview else 16,'sha256':hashes[name],'metadata':meta,'redMean8Scale':means[name]})
        del data
    for name in ['tint','tone-zero']:assert hashes[name]!=hashes['as-shot'],name
    invalid=a.output/'invalid.dng';invalid.write_bytes(b'not a camera RAW');bad=subprocess.run([str(a.helper.resolve()),'--inspect',str(invalid.resolve())],capture_output=True,timeout=180);assert bad.returncode!=0
    assert hashes['as-shot']==hashes['repeat'];assert hashes['exposure']!=hashes['as-shot'];assert hashes['temperature']!=hashes['as-shot'];assert means['exposure']>means['as-shot']
    assert hashlib.sha256(a.input.read_bytes()).hexdigest()==original
    receipt={'status':'PASS','inputSHA256':original,'helperSHA256':hashlib.sha256(a.helper.read_bytes()).hexdigest(),'cases':rows,'scope':'Independent binary reader, real sensor development, no original-file writes'}
    (a.output/'receipt.json').write_text(json.dumps(receipt,indent=2),encoding='utf-8');print(json.dumps(receipt))
if __name__=='__main__':main()
