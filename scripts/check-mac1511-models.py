#!/usr/bin/env python3
"""Exercise original Swift Codable declarations, isolated from unrelated app UI.
This checks model interchange, not a GUI project-open acceptance.
"""
import argparse, hashlib, json, pathlib, subprocess, tempfile
p=argparse.ArgumentParser();p.add_argument('--source',type=pathlib.Path,required=True);p.add_argument('--reports',type=pathlib.Path,required=True);p.add_argument('--verify-return',type=pathlib.Path);a=p.parse_args()
root=pathlib.Path(__file__).resolve().parents[1]; src=a.source/'Compositor/Document'
paths=['LayerAdjustment.swift','LensEffects.swift','ImageAdjustments.swift']
hashes={n:hashlib.sha256((src/n).read_bytes()).hexdigest() for n in paths}
assert subprocess.check_output(['git','rev-parse','HEAD'],cwd=a.source,text=True).strip()=='9959c32d77330ec8d7b42a49353d903283ecb8e6'
# Refuse locally edited source, including the exact Codable declarations.
for name in paths:
 original=subprocess.check_output(['git','show','HEAD:Compositor/Document/'+name],cwd=a.source)
 assert (src/name).read_bytes()==original,name
layer=(src/paths[0]).read_text();lens=(src/paths[1]).read_text();colors=(src/paths[2]).read_text()
# The synthesized Codable fields and default initializers are retained verbatim;
# remove only methods requiring FilterSettings/EditorSession and pixel rendering.
chunks=['import Foundation\nimport CoreGraphics\n']
color=colors[colors.index('nonisolated struct AdjustmentColor:'):]
# Keep declaration through the three stored components, independent of later methods.
lines=color.splitlines(); end=next(i for i,l in enumerate(lines) if 'var blue:' in l)
chunks.append('\n'.join(lines[:end+1])+'\n}\n')
chunks.append(lens[lens.index('nonisolated enum LensEffect:'):lens.index('    /// The effect on `pixels`')]+ '}\n')
chunks.append('enum ImageAdjustmentPixels { static func clamp(_ v:Double,_ r:ClosedRange<Double>,_ f:Double)->Double {v.isFinite ? min(r.upperBound,max(r.lowerBound,v)):f} }\n')
for name in ['FilterLayerSettings','VignetteLayerSettings','TonalLayerSettings','SharpenLayerSettings']:
 text=layer[layer.index('nonisolated struct '+name+':'):]; end=text.index('    init() {}')+len('    init() {}');chunks.append(text[:end]+'\n}\n')
chunks.append(r'''
let fixtureRoot=URL(fileURLWithPath:CommandLine.arguments[1],isDirectory:true)
let encoder=JSONEncoder();encoder.outputFormatting=[.prettyPrinted,.sortedKeys]
let decoder=JSONDecoder()
var v=VignetteLayerSettings();v.amount=72;v.color=AdjustmentColor(red:0.12,green:0.34,blue:0.56);v.roundness = -45
var t=TonalLayerSettings();t.amount=85;t.radius=31.5;t.shadows = -30;t.midtones=67;t.highlights=12
var s=SharpenLayerSettings();s.amount=230;s.radius=4.5
var lens=LensEffectSettings();lens.effect = .prism;lens.threshold=100;lens.prismBothWays=true;lens.prismBlur=41;lens.prismFringe=0
var cases=[FilterLayerSettings](repeating:FilterLayerSettings(),count:4)
cases[0].vignette=v;cases[1].tonal=t;cases[2].sharpen=s;cases[3].lens=lens
for (i,value) in cases.enumerated() {
 let name="model-filter-\(i).json"
 if CommandLine.arguments.count>2 {
  let returned=URL(fileURLWithPath:CommandLine.arguments[2],isDirectory:true).appendingPathComponent(name)
  let actual=try decoder.decode(FilterLayerSettings.self,from:Data(contentsOf:returned))
  precondition(actual==value,"Swift could not round-trip returned model \(i)")
 } else {try encoder.encode(value).write(to:fixtureRoot.appendingPathComponent(name))}
 print("PASS Swift Codable model \(i)")
}
''')
fixtures=root/'engine/tests/fixtures/mac1511';fixtures.mkdir(exist_ok=True,parents=True)
with tempfile.TemporaryDirectory(prefix='mac1511-codable-') as work:
 work=pathlib.Path(work);main=work/'main.swift';main.write_text('\n'.join(chunks));exe=work/'reference'
 subprocess.run(['swiftc',str(main),'-o',str(exe)],check=True)
 command=[str(exe),str(fixtures)]+([str(a.verify_return)]if a.verify_return else [])
 subprocess.run(command,check=True)
a.reports.mkdir(parents=True,exist_ok=True)
(a.reports/('model-return.json'if a.verify_return else'model-reference.json')).write_text(json.dumps({'referenceCommit':'9959c32d77330ec8d7b42a49353d903283ecb8e6','sourceSha256':hashes,'kind':'original Swift Codable model interchange; not GUI project acceptance','returned':bool(a.verify_return),'passed':True},indent=2)+'\n')
