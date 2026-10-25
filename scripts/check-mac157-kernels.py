#!/usr/bin/env python3
"""Independent upstream reference generation; mutates only mac157 fixtures and the requested report directory."""
import ctypes as c,subprocess,json,hashlib,pathlib,math
import argparse, tempfile
parser=argparse.ArgumentParser(description="Regenerate Mac 1.5.7 native reference fixtures on macOS; provide the frozen upstream source checkout.")
parser.add_argument('--source',type=pathlib.Path,required=True)
parser.add_argument('--reports',type=pathlib.Path,required=True)
args=parser.parse_args()
root=pathlib.Path(__file__).resolve().parents[1]
src=args.source.resolve()/'Compositor/Rendering'
args.reports.mkdir(parents=True,exist_ok=True)
work=tempfile.TemporaryDirectory(prefix='compositor-kernel157-');out=pathlib.Path(work.name)
expected='76578539bc45a32817aff669c4e41523d7040f3c691ad35adf0723a70a89a705'
assert hashlib.sha256((src.parent/'Resources/CameraRawShadows.bin').read_bytes()).hexdigest()==expected, 'not the frozen 1.5.7 resource'

for kind,folder in [('reference',src),('windows',root/'engine/native')]:
 subprocess.run(['clang','-dynamiclib','-fblocks','-O2',str(folder/'PatchPixels.c'),str(folder/'HealPixels.c'),'-o',str(out/(kind+'.dylib'))],check=True)
libs=[c.CDLL(str(out/(k+'.dylib'))) for k in ['reference','windows']]
for lib in libs:
 lib.patch_fill.argtypes=[c.c_void_p,c.c_size_t,c.c_void_p,c.c_size_t,c.c_int,c.c_int,c.c_uint32]
 lib.spot_heal.argtypes=[c.c_void_p,c.c_void_p,c.c_size_t,c.c_size_t,c.c_size_t,c.c_float,c.c_int,c.c_uint32]
results=[]
for case in ['stripes','ridge','alpha','empty','all-selected']:
 w,h=96,64; data=bytearray(); mask=bytearray(w*h)
 for y in range(h):
  for x in range(w):
   hole=(x-48)**2+(y-32)**2<8**2
   if case=='empty':hole=False
   if case=='all-selected':hole=True
   mask[y*w+x]=255 if hole else 0
   gray=100 if x%4<2 else 112
   rgb=[gray]*3
   if case=='ridge':rgb=[120+y//2,160+y//3,230] if y<32+8*math.sin(x/13) else [60+x%7,110+x%7,50+x%7]
   if hole:rgb=[230,20,20]
   alpha=0 if case=='alpha' and x<8 else 255
   data.extend(rgb+[alpha] if alpha else [0,0,0,0])
 for op in ['fill',0,1,2]:
  outputs=[];codes=[]
  for lib in libs:
   pixels=(c.c_ubyte*len(data)).from_buffer_copy(data);coverage=(c.c_ubyte*len(mask)).from_buffer_copy(mask)
   code=lib.patch_fill(pixels,w*4,coverage,w,w,h,0x6d2b79f5) if op=='fill' else lib.spot_heal(pixels,coverage,w,h,w*4,1.,op,7)
   outputs.append(bytes(pixels));codes.append(code)
  fixtures=root/'engine/tests/fixtures/mac157';fixtures.mkdir(parents=True,exist_ok=True)
  (fixtures/(case+'-input.rgba')).write_bytes(data)
  (fixtures/(case+'-mask.gray')).write_bytes(mask)
  (fixtures/(case+'-'+str(op)+'.rgba')).write_bytes(outputs[0])
  different=sum(a!=b for a,b in zip(*outputs));passed=different==0 and codes[0]==codes[1]
  results.append({'case':case,'operation':op,'codes':codes,'differentBytes':different,'passed':passed,'sha256':hashlib.sha256(outputs[1]).hexdigest()})
  print(case,op,passed,different,flush=True)
report={'referenceCommit':'9537736da6c324b948cdcba24adf74760498ea14','host':'macOS C reference vs portable C; not Windows/WASM acceptance','tests':results,'passed':all(t['passed'] for t in results)}
(args.reports/'kernel-reference.json').write_text(json.dumps(report,indent=2)+'\n')
assert report['passed']
# Camera Raw's region Shadows pass, called directly from the frozen upstream C.
subprocess.run(['clang','-dynamiclib','-fblocks','-O2',str(src/'AdjustPixels.c'),str(src/'LensPixels.c'),'-o',str(out/'shadows.dylib')],check=True)
lib=c.CDLL(str(out/'shadows.dylib'))
lib.adjust_camera_raw_shadows.argtypes=[c.c_void_p,c.c_size_t,c.c_size_t,c.c_size_t,c.c_void_p,c.c_int,c.c_float,c.c_float,c.c_int,c.c_float,c.c_float]
import struct
blob=(src.parent/'Resources/CameraRawShadows.bin').read_bytes()
sets,n,xc,kc,terms=struct.unpack_from('<5H',blob,4);normal,xlow,xstep,klow,kstep=struct.unpack_from('<5f',blob,14)
amounts=struct.unpack_from('<'+str(n)+'f',blob,34);per=xc*kc*terms
coeff=struct.unpack_from('<'+str(sets*n*per)+'f',blob,34+n*4)
f32=lambda v:c.c_float(v).value
points=sorted([(v,i) for i,v in enumerate(amounts)]+[(0,None)])
source=bytearray()
for y in range(64):
 for x in range(96):
  alpha=[0,64,128,255][(x//24)%4]; tone=(x*13+y*7)%256
  source.extend([round(v*alpha/255) for v in [tone,(tone*3)%256,(tone*7)%256]]+[alpha])
fixtures=root/'engine/tests/fixtures/mac157';(fixtures/'shadows-input.rgba').write_bytes(source)
shadow_results=[]
for set_ in range(2):
 for amount in [-100,-50,-12.5,25,50,75,100]:
  value=max(points[0][0],min(points[-1][0],amount));index=0
  while index<len(points)-2 and value>points[index+1][0]:index+=1
  lo,li=points[index];hi,ui=points[index+1];blend=f32((value-lo)/(hi-lo))
  at=lambda a,i: 0 if a is None else coeff[(set_*n+a)*per+i]
  values=[f32(f32(at(li,i)*f32(1-blend))+f32(at(ui,i)*blend)) for i in range(per)]
  coefficients=(c.c_float*per)(*values);pixels=(c.c_ubyte*len(source)).from_buffer_copy(source)
  lib.adjust_camera_raw_shadows(pixels,96,64,96*4,coefficients,xc,xlow,xstep,kc,klow,kstep)
  name=f'shadows-{set_}-{amount}.rgba';(fixtures/name).write_bytes(bytes(pixels))
  shadow_results.append({'file':name,'amount':amount,'scale':1 if set_==0 else .05,'sha256':hashlib.sha256(bytes(pixels)).hexdigest()})
(args.reports/'shadows-reference.json').write_text(json.dumps(shadow_results,indent=2)+'\n')
print('Generated 14 independent upstream Shadows references.')
