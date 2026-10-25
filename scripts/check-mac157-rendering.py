#!/usr/bin/env python3
"""Regenerate independent Mac 1.5.7 shape/Metal references on macOS, never from port code."""
import argparse, pathlib, tempfile, hashlib, subprocess, json
p=argparse.ArgumentParser();p.add_argument('--source',type=pathlib.Path,required=True);p.add_argument('--reports',type=pathlib.Path,required=True);args=p.parse_args()
root=pathlib.Path(__file__).resolve().parents[1];fixtures=root/'engine/tests/fixtures/mac157';drivers=root/'scripts/mac157-reference';args.reports.mkdir(parents=True,exist_ok=True)
sources={'Compositor/Document/ShapeTool.swift':'adaf9af7524c9ff90ec52d998040362d8f67ac0bc2e4df20ba40505990fe7ec3','Compositor/Rendering/MetalDispersion.swift':'459deaa8642cf1b5eb50818bb0efb8b7b3d2439105135ea9b44c9575aaa101a0'}
sources.update({'Compositor/Document/Filters.swift': '61a51b2026c590327bac52e93d1d9bbf84b850ca28d819d65bd7abd2420a5920', 'Compositor/Document/PixelAdjust.swift': '697d624c80520ecb5acd43a6a9f9e059606493379951f5f08116f1bc28ab11ae'})
for name,digest in sources.items():assert hashlib.sha256((args.source/name).read_bytes()).hexdigest()==digest, f'Not frozen upstream: {name}'
with tempfile.TemporaryDirectory(prefix='mac157-reference-') as work:
 work=pathlib.Path(work);main=work/'main.swift'
 shape=(args.source/'Compositor/Document/ShapeTool.swift').read_text().split('/// What a shape layer draws')[0]
 main.write_text(shape+'\n'+(drivers/'shapes.swift').read_text());exe=work/'shapes'
 subprocess.run(['swiftc',str(main),'-o',str(exe)],check=True);subprocess.run([str(exe),str(fixtures)],check=True)
 main.write_text((drivers/'rgb.swift').read_text());exe=work/'rgb'
 subprocess.run(['swiftc',str(args.source/'Compositor/Rendering/MetalDispersion.swift'),str(main),'-o',str(exe)],check=True);subprocess.run([str(exe),str(fixtures)],check=True)
 subprocess.run(['swift',str(drivers/'filters.swift'),str(fixtures)],check=True)
receipt={'referenceCommit':'9537736da6c324b948cdcba24adf74760498ea14','sourceSha256':sources,'host':'macOS Core Graphics / Metal, independent upstream reference generation; not Windows acceptance','fixtures':{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(fixtures.iterdir()) if p.name.startswith(('shape-','shapes.','rgb-','bloom-'))}}
(args.reports/'rendering-reference.json').write_text(json.dumps(receipt,indent=2)+'\n')
