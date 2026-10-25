#!/usr/bin/env python3
"""Compile pinned upstream LensEffects and MetalBokeh unchanged to generate independent references."""
import argparse, hashlib, json, pathlib, subprocess, tempfile

p = argparse.ArgumentParser()
p.add_argument('--source', type=pathlib.Path, required=True)
p.add_argument('--reports', type=pathlib.Path, required=True)
args = p.parse_args()
root = pathlib.Path(__file__).resolve().parents[1]
fixtures = root / 'engine/tests/fixtures/mac157'
sources = {
    'Compositor/Document/LensEffects.swift': 'a0159703290821eeca49159e45655fa34224500a49b62689119934cf5f209d72',
    'Compositor/Rendering/MetalBokeh.swift': '034a259b3d2726be684184df88635684ca5e992bd32f623184d0423dbe1474e5',
    'Compositor/Document/PixelAdjust.swift': '697d624c80520ecb5acd43a6a9f9e059606493379951f5f08116f1bc28ab11ae',
}
for name, digest in sources.items():
    assert hashlib.sha256((args.source / name).read_bytes()).hexdigest() == digest, f'Not frozen upstream: {name}'
with tempfile.TemporaryDirectory(prefix='mac157-lens-') as work:
    work = pathlib.Path(work)
    main = work / 'main.swift'
    main.write_text((root / 'scripts/mac157-reference/lens.swift').read_text())
    exe = work / 'reference'
    subprocess.run(['swiftc', str(args.source / 'Compositor/Document/LensEffects.swift'),
                    str(args.source / 'Compositor/Rendering/MetalBokeh.swift'), str(main), '-o', str(exe)], check=True)
    subprocess.run([str(exe), str(fixtures), str(root / 'engine/native/LensToneCurves.bin')], check=True)
args.reports.mkdir(parents=True, exist_ok=True)
receipt = {'referenceCommit': '9537736da6c324b948cdcba24adf74760498ea14', 'sourceSha256': sources,
           'portableToneCurveSha256': hashlib.sha256((root / 'engine/native/LensToneCurves.bin').read_bytes()).hexdigest(),
           'host': 'macOS original Swift / Core Image / Metal; not Windows acceptance',
           'fixtures': {f.name: hashlib.sha256(f.read_bytes()).hexdigest() for f in sorted(fixtures.glob('lens-*'))}}
(args.reports / 'lens-reference.json').write_text(json.dumps(receipt, indent=2) + '\n')
