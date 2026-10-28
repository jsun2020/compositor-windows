#!/usr/bin/env python3
"""Pinned native kernels and original Core Image expressions for finishing filters."""
import argparse,hashlib,json,pathlib,subprocess,tempfile
p=argparse.ArgumentParser();p.add_argument('--source',type=pathlib.Path,required=True);p.add_argument('--reports',type=pathlib.Path,required=True);a=p.parse_args()
root=pathlib.Path(__file__).resolve().parents[1]
sources={'Document/Filters.swift':'dcc83802aaf7914d1793656638f9131244c32a27e0a84340dacea795ccb0fbaf','Rendering/AdjustPixels.c':'f909779259a9b621d7573fd6bf48d0197262978094c6e0884b4bd7116adbdedb','Rendering/AdjustPixels.h':'aa7a9f1093e8cd8feeaca57b7cf489625d343c9f3abb2a1b0587f72855b3dadb','Rendering/LensPixels.c':'06851d9d50f7d7a163cc97a5b889caba1ad0b44ef7b10accd8799405bae72d3c','Rendering/LensPixels.h':'81d60f653de4f5d36d23cfe442b0c23ced1dfb500c7bbcb0c7fabed36904c73f'}
for n,h in sources.items():assert hashlib.sha256((a.source/'Compositor'/n).read_bytes()).hexdigest()==h,n
fixtures=root/'engine/tests/fixtures/mac1511';fixtures.mkdir(parents=True,exist_ok=True)
with tempfile.TemporaryDirectory(prefix='mac1511-finishing-') as work:
 work=pathlib.Path(work);objects=[]
 for name in ['AdjustPixels','LensPixels']:
  out=work/(name+'.o');subprocess.run(['clang','-O2','-fblocks','-c',str(a.source/'Compositor/Rendering'/(name+'.c')),'-o',str(out)],check=True);objects.append(str(out))
 main=work/'main.swift';main.write_text((root/'scripts/mac1511-reference/finishing.swift').read_text());exe=work/'reference'
 subprocess.run(['swiftc',str(main),*objects,'-o',str(exe)],check=True);subprocess.run([str(exe),str(fixtures)],check=True)
a.reports.mkdir(parents=True,exist_ok=True)
receipt={'referenceCommit':'9959c32d77330ec8d7b42a49353d903283ecb8e6','sourceSha256':sources,'host':'macOS native upstream C + Core Image; not Windows acceptance','fixtures':{f.name:hashlib.sha256(f.read_bytes()).hexdigest() for f in sorted(fixtures.iterdir()) if f.name.startswith(('sharpen-','tonal-','vignette-'))}}
(a.reports/'finishing-reference.json').write_text(json.dumps(receipt,indent=2)+'\n')
