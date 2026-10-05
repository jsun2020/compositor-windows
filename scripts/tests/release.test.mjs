import { test } from 'node:test';
import assert from 'node:assert/strict';
import { validateVersions, releaseExists, projectVersion } from '../release-metadata.mjs';
import { inspectZip, crc32 } from '../verify-portable-release.mjs';

test('matching main/version tag release metadata', () => {
  assert.deepEqual(validateVersions('0.8.0', '0.8.0', '0.8.0', 'branch', 'main'), { version: '0.8.0', tag: 'v0.8.0' });
  assert.deepEqual(validateVersions('0.8.0', '0.8.0', '0.8.0', 'tag', 'v0.8.0'), { version: '0.8.0', tag: 'v0.8.0' });
});
test('version mismatch and unrelated ref cannot publish', () => {
  for (const values of [['0.8.0', '0.7.0', '0.8.0', 'branch', 'main'], ['0.8.0', '0.8.0', '0.7.0', 'branch', 'main'], ['0.8.0', '0.8.0', '0.8.0', 'tag', 'v0.5.0'], ['0.8.0', '0.8.0', '0.8.0', 'branch', 'codex/phase6']]) assert.throws(() => validateVersions(...values));
});
test('unstable/unsafe versions are refused', () => {
  for (const version of ['0.8', '0.8.0-beta', '0.8.0\n', '0.8.0;echo']) assert.throws(() => validateVersions(version, version, version, 'branch', 'main'));
});
test('current project metadata matches', () => {
  const v = projectVersion();
  assert.deepEqual(validateVersions(v.pkg, v.tauri, v.cargo, 'branch', 'main'), { version: v.pkg, tag: `v${v.pkg}` });
});
test('only an explicit HTTP 404 means not yet released', async () => {
  assert.equal(await releaseExists(new Response('', { status: 404 })), false);
  for (const status of [401, 403, 429, 500]) await assert.rejects(releaseExists(new Response('', { status })));
});
test('published releases are immutable; drafts require inspection', async () => {
  assert.equal(await releaseExists(Response.json({ draft: false })), true);
  await assert.rejects(releaseExists(Response.json({ draft: true })));
});

// Independent small ZIP fixture, with stored payloads and an explicit central directory.
function zip(names = ['Compositor.exe', 'LICENSE-Compositor.txt', 'README.txt']) {
  const locals = [], central = []; let offset = 0;
  for (const name of names) {
    const n = Buffer.from(name), bytes = Buffer.from(name), crc = crc32(bytes);
    const l = Buffer.alloc(30); l.writeUInt32LE(0x04034b50); l.writeUInt32LE(crc, 14); l.writeUInt32LE(bytes.length, 18); l.writeUInt32LE(bytes.length, 22); l.writeUInt16LE(n.length, 26);
    const c = Buffer.alloc(46); c.writeUInt32LE(0x02014b50); c.writeUInt32LE(crc, 16); c.writeUInt32LE(bytes.length, 20); c.writeUInt32LE(bytes.length, 24); c.writeUInt16LE(n.length, 28); c.writeUInt32LE(offset, 42);
    locals.push(l, n, bytes); central.push(c, n); offset += l.length + n.length + bytes.length;
  }
  const directory = Buffer.concat(central), end = Buffer.alloc(22); end.writeUInt32LE(0x06054b50); end.writeUInt16LE(names.length, 8); end.writeUInt16LE(names.length, 10); end.writeUInt32LE(directory.length, 12); end.writeUInt32LE(offset, 16);
  return Buffer.concat([...locals, directory, end]);
}
test('standard CRC and complete three-entry archive', () => {
  assert.equal(crc32(Buffer.from('123456789')), 0xcbf43926);
  assert.equal(inspectZip(zip()).length, 3);
});
test('tampered payload, duplicate entries, traversal and truncation fail', () => {
  const changed = zip(); changed[30 + 'Compositor.exe'.length] ^= 1;
  for (const b of [changed, zip(['Compositor.exe', 'Compositor.exe', 'README.txt']), zip(['../Compositor.exe', 'LICENSE-Compositor.txt', 'README.txt']), zip().subarray(0, 20)]) assert.throws(() => inspectZip(b));
});
