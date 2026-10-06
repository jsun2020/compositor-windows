import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import zlib from 'node:zlib';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { projectVersion } from './release-metadata.mjs';

export const sha256 = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
export const PORTABLE_FILES = ['Compositor.exe','CompositorRaw.exe','LICENSE-Compositor.txt','README.txt','msvcp140.dll','vcruntime140.dll','vcruntime140_1.dll','LibRaw-notices/COPYRIGHT','LibRaw-notices/LICENSE.CDDL','LibRaw-notices/LICENSE.LGPL','LibRaw-notices/provenance.json'];
export function crc32(data) {
  let crc = 0xffffffff;
  for (const byte of data) {
    crc ^= byte;
    for (let bit = 0; bit < 8; bit++) crc = (crc >>> 1) ^ ((crc & 1) ? 0xedb88320 : 0);
  }
  return (crc ^ 0xffffffff) >>> 0;
}

export function inspectZip(buffer) {
  if (buffer.length < 22) throw new Error('ZIP end record absent');
  let end = buffer.length - 22;
  for (; end >= Math.max(0, buffer.length - 65557); end--) {
    if (buffer.readUInt32LE(end) === 0x06054b50 && end + 22 + buffer.readUInt16LE(end + 20) === buffer.length) break;
  }
  if (end < Math.max(0, buffer.length - 65557)) throw new Error('ZIP end record absent');
  assert.equal(buffer.readUInt32LE(end + 4), 0, 'Multi-volume ZIP refused');
  const count = buffer.readUInt16LE(end + 10), entries = [];
  assert.equal(count, PORTABLE_FILES.length, 'Portable must include exactly the app, RAW helper, runtimes and notices');
  assert.equal(buffer.readUInt16LE(end + 8), count);
  let at = buffer.readUInt32LE(end + 16);
  const centralEnd = at + buffer.readUInt32LE(end + 12);
  assert.equal(centralEnd, end, 'Invalid central directory size');
  for (let i = 0; i < count; i++) {
    assert.equal(buffer.readUInt32LE(at), 0x02014b50, 'Invalid ZIP central record');
    const flags = buffer.readUInt16LE(at + 8), method = buffer.readUInt16LE(at + 10);
    assert.equal(flags & 1, 0, 'Encrypted ZIP refused');
    const expected = buffer.readUInt32LE(at + 16), compressed = buffer.readUInt32LE(at + 20), size = buffer.readUInt32LE(at + 24);
    assert(size > 0 && size <= 128 * 1024 * 1024, 'Invalid portable entry size');
    const nameLength = buffer.readUInt16LE(at + 28), extra = buffer.readUInt16LE(at + 30), comment = buffer.readUInt16LE(at + 32), local = buffer.readUInt32LE(at + 42);
    const name = buffer.toString('utf8', at + 46, at + 46 + nameLength).replaceAll('\\','/');
    assert.equal(buffer.readUInt32LE(local), 0x04034b50, 'Invalid ZIP local record');
    assert.equal(buffer.readUInt16LE(local + 8), method);
    const localNameLength = buffer.readUInt16LE(local + 26), localExtra = buffer.readUInt16LE(local + 28);
    assert.equal(buffer.toString('utf8', local + 30, local + 30 + localNameLength).replaceAll('\\','/'), name, 'ZIP entry names differ');
    const start = local + 30 + localNameLength + localExtra;
    assert(start + compressed <= buffer.readUInt32LE(end + 16), 'ZIP payload overlaps directory');
    const raw = buffer.subarray(start, start + compressed);
    const bytes = method === 0 ? raw : method === 8 ? zlib.inflateRawSync(raw, { maxOutputLength: size }) : null;
    assert(bytes && bytes.length === size && crc32(bytes) === expected, 'ZIP CRC/length failure: ' + name);
    entries.push({ name, bytes });
    at += 46 + nameLength + extra + comment;
  }
  assert.equal(at, centralEnd);
  assert.deepEqual(entries.map(e => e.name).sort(), [...PORTABLE_FILES].sort());
  return entries;
}

function checkReceipt(directory) {
  const receiptBytes = fs.readFileSync(path.join(directory, 'release-manifest.json'));
  const receipt = JSON.parse(receiptBytes);
  assert.equal(receipt.schema, 1);
  assert.equal(receipt.sourceCommit, process.env.GITHUB_SHA, 'Downloaded artifact source differs');
  assert.equal(receipt.sourceDirty, false, 'Published artifacts require a clean source checkout');
  assert.equal(receipt.version, process.env.RELEASE_VERSION);
  assert.equal(path.basename(receipt.zip.name), receipt.zip.name);
  const zip = fs.readFileSync(path.join(directory, receipt.zip.name));
  assert.equal(zip.length, receipt.zip.bytes);
  assert.equal(sha256(zip), receipt.zip.sha256);
  const entries = inspectZip(zip);
  assert.deepEqual(entries.map(e => ({ name: e.name, bytes: e.bytes.length, sha256: sha256(e.bytes) })), receipt.entries);
  assert.equal(fs.readFileSync(path.join(directory, 'SHA256SUMS.txt'), 'utf8'), `${receipt.zip.sha256}  ${receipt.zip.name}\n${sha256(receiptBytes)}  release-manifest.json\n`);
  assert.deepEqual(fs.readdirSync(directory).sort(), [receipt.zip.name, 'release-manifest.json', 'SHA256SUMS.txt'].sort());
  console.log('Downloaded ZIP, CRCs, complete hashes and source receipt verified.');
}

function prepare(zipPath, output) {
  const versions = projectVersion();
  assert(versions.pkg === versions.tauri && versions.pkg === versions.cargo, 'Version mismatch');
  const version = versions.pkg, name = path.basename(zipPath), stamp = name.match(/^Compositor-portable-\d+\.\d+\.\d+-(\d{8}-\d{4})\.zip$/)?.[1];
  assert(stamp && name === `Compositor-portable-${version}-${stamp}.zip`, 'ZIP version/name mismatch');
  const marker = `COMPOSITOR_BUILD_${version}_${stamp}`;
  const zip = fs.readFileSync(zipPath), entries = inspectZip(zip);
  const stage = zipPath.slice(0, -4);
  for (const entry of entries) assert(entry.bytes.equals(fs.readFileSync(path.join(stage, entry.name))), 'Archive/stage bytes differ');
  for (const name of ['Compositor.exe','CompositorRaw.exe','msvcp140.dll','vcruntime140.dll','vcruntime140_1.dll']) assert.equal(entries.find(e=>e.name===name).bytes.toString('ascii',0,2),'MZ',`Invalid native component: ${name}`);
  const raw = JSON.parse(entries.find(e=>e.name==='LibRaw-notices/provenance.json').bytes.toString('utf8').replace(/^\uFEFF/,''));
  assert.equal(raw.version,'0.22.2');assert.equal(raw.license,'CDDL-1.0');
  assert.equal(raw.sourceSHA256,'AC64FA12BB00A7581332D4C6AB918C0533FB3F119D6B668D47A6875410DCA948');
  assert.equal(raw.helperSHA256.toLowerCase(),sha256(entries.find(e=>e.name==='CompositorRaw.exe').bytes));
  assert.deepEqual(raw.msvcRuntime.map(e=>e.name).sort(),['msvcp140.dll','vcruntime140.dll','vcruntime140_1.dll'].sort());
  for(const dll of raw.msvcRuntime)assert.equal(dll.sha256.toLowerCase(),sha256(entries.find(e=>e.name===dll.name).bytes));
  assert(entries.find(e => e.name === 'README.txt').bytes.toString('utf8').includes(`Compositor for Windows ${version} (${marker})`));
  assert(entries.find(e => e.name === 'LICENSE-Compositor.txt').bytes.equals(fs.readFileSync('engine/native/LICENSE-Compositor.txt')));
  const sourceCommit = process.env.GITHUB_SHA ?? execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
  const sourceDirty=Boolean(execFileSync('git',['status','--porcelain','--untracked-files=no'],{encoding:'utf8'}).trim());
  if(process.env.GITHUB_ACTIONS==='true')assert.equal(sourceDirty,false,'CI portable source must be clean');
  assert(/^[a-f0-9]{40}$/.test(sourceCommit), 'Invalid source commit');
  const assetNames = fs.readdirSync('app/dist/assets').filter(n => /^compositor_engine_bg-.*\.wasm$/.test(n));
  assert.equal(assetNames.length, 1);
  const wasm = fs.readFileSync(path.join('app/dist/assets', assetNames[0]));
  assert(wasm.equals(fs.readFileSync('app/src/engine/pkg/compositor_engine_bg.wasm')), 'Production/source WASM differs');
  const receipt = { schema: 1, version, marker, sourceCommit, sourceDirty, zip: { name, bytes: zip.length, sha256: sha256(zip) }, wasm: { bytes: wasm.length, sha256: sha256(wasm) }, entries: entries.map(e => ({ name: e.name, bytes: e.bytes.length, sha256: sha256(e.bytes) })) };
  assert(!fs.existsSync(output), 'Preserve existing release output');
  fs.mkdirSync(output, { recursive: true });
  const receiptBytes = Buffer.from(JSON.stringify(receipt, null, 2) + '\n');
  fs.writeFileSync(path.join(output, name), zip);
  fs.writeFileSync(path.join(output, 'release-manifest.json'), receiptBytes);
  fs.writeFileSync(path.join(output, 'SHA256SUMS.txt'), `${receipt.zip.sha256}  ${name}\n${sha256(receiptBytes)}  release-manifest.json\n`);
  console.log(JSON.stringify({ version, marker, sourceCommit, zipSha256: receipt.zip.sha256, crcVerifiedEntries: entries.length }));
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    if (process.argv[2] === '--receipt') checkReceipt(process.argv[3]);
    else prepare(process.argv[2], process.argv[3]);
  } catch (error) { console.error(error.message); process.exitCode = 1; }
}
