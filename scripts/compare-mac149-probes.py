"""Retain exact pixel differences; no threshold relaxation or retries."""
import argparse,hashlib,json
from pathlib import Path

def main():
 p=argparse.ArgumentParser();p.add_argument('windows',type=Path);p.add_argument('mac',type=Path);p.add_argument('output',type=Path);a=p.parse_args();assert not a.output.exists(),'Preserve previous comparisons';a.output.mkdir(parents=True)
 cases=json.loads((a.windows/'cases.json').read_text(encoding='utf-8'))['cases'];rows=[]
 for name in ['input-copy',*[c['name']for c in cases],'quantize16']:
  original=(a.windows/'input.rgba')if name=='input-copy'else(a.windows/'Windows'/f'{name}.rgba');returned=a.mac/f'{name}.rgba'
  if not returned.exists():rows.append({'case':name,'status':'MISSING'});continue
  left,right=original.read_bytes(),returned.read_bytes();assert len(left)==len(right)==128*96*4
  delta=[abs(x-y)for x,y in zip(left,right)];channels=[delta[c::4]for c in range(4)]
  rows.append({'case':name,'status':'PASS'if left==right else'DIFF','windowsSHA256':hashlib.sha256(left).hexdigest(),'macSHA256':hashlib.sha256(right).hexdigest(),'differentBytes':sum(v>0for v in delta),'maxRGBA':[max(c)for c in channels],'meanRGBA':[sum(c)/len(c)for c in channels]})
 manual={name:(a.mac/name).exists()for name in ['installed-version.txt','RESULT.txt','effects-opaque.png','effects-alpha.png','export-300x150.png','export-300x150.jpg','export-300x150.pdf','ui-edited.comp']}
 receipt={'status':'INCOMPLETE','sourceStatus':'PASS'if all(r['status']=='PASS'for r in rows)else'INCOMPLETE','sourcePixelGate':'exact premultiplied RGBA bytes','cases':rows,'actualApplicationFiles':manual,'actualApplicationReview':'Requires separate PNG/project/RESULT validation; file presence alone is not acceptance'}
 (a.output/'comparison.json').write_text(json.dumps(receipt,indent=2),encoding='utf-8');print(json.dumps(receipt,indent=2))
if __name__=='__main__':main()
