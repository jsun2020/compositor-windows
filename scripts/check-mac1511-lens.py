#!/usr/bin/env python3
"""Generate independent fixtures from the pinned Mac 1.5.11 implementation."""
import argparse, hashlib, json, pathlib, subprocess, tempfile
p=argparse.ArgumentParser()
p.add_argument('--source',type=pathlib.Path,required=True)
p.add_argument('--reports',type=pathlib.Path,required=True)
a=p.parse_args()
root=pathlib.Path(__file__).resolve().parents[1]
sources={
 'Compositor/Document/LensEffects.swift':'d71a9bf1d65968b11b85ff95725bba4dcb58368860121fc7a2d959e4648dfcf0',
 'Compositor/Rendering/MetalBokeh.swift':'7a319ffeaa59294c8fe47063c7ac408fa796cb912ce76d613a8918cb3e826787',
}
for name,digest in sources.items():
 assert hashlib.sha256((a.source/name).read_bytes()).hexdigest()==digest, name
fixtures=root/'engine/tests/fixtures/mac1511'
fixtures.mkdir(parents=True,exist_ok=True)
tables=root/'engine/native/LensDensityCurves.bin'
with tempfile.TemporaryDirectory(prefix='mac1511-reference-') as tmp:
 tmp=pathlib.Path(tmp); main=tmp/'main.swift'; exe=tmp/'reference'
 main.write_text((a.source/'Compositor/Document/LensEffects.swift').read_text()+'\n'+(root/'scripts/mac1511-reference/lens.swift').read_text())
 subprocess.run(['swiftc',str(a.source/'Compositor/Rendering/MetalBokeh.swift'),str(main),'-o',str(exe)],check=True)
 subprocess.run([str(exe),str(fixtures),str(tables)],check=True)
a.reports.mkdir(parents=True,exist_ok=True)
receipt={'referenceCommit':'9959c32d77330ec8d7b42a49353d903283ecb8e6','sourceSha256':sources,'host':'macOS original Swift/Core Image/Metal; not Windows acceptance','fixtures':{f.name:hashlib.sha256(f.read_bytes()).hexdigest() for f in sorted(fixtures.iterdir())},'densityTableSha256':hashlib.sha256(tables.read_bytes()).hexdigest()}
(a.reports/'lens-reference.json').write_text(json.dumps(receipt,indent=2)+'\n')
