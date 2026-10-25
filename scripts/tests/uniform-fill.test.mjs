import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';

const source = fs.readFileSync(new URL('../../engine-wasm/src/uniform.rs', import.meta.url), 'utf8');
const helper = source.match(/export function repeatUniformBytes\([\s\S]*?\n}\r?\n/)?.[0];
assert(helper, 'Actual WASM inline helper must be present');
const fill = new Function(helper.replace('export ', '') + '; return repeatUniformBytes;')();

test('actual uniform helper preserves exact RGBA bytes and guards for aligned and unaligned spans', () => {
  for (const length of [4, 8, 52, 4096, 4194304, 4194356]) {
    for (const offset of [0, 1, 4, 19]) {
      for (const pixel of [[0, 0, 0, 0], [255, 255, 255, 255], [17, 31, 43, 97], [255, 0, 128, 1]]) {
        const memory = { buffer: new ArrayBuffer(length + 64) };
        const actual = new Uint8Array(memory.buffer); actual.fill(165);
        const expected = new Uint8Array(length + 64); expected.fill(165);
        // Independent scalar byte oracle; no native-word packing or doubling.
        for (let at = 0; at < length; at++) expected[offset + at] = pixel[at % 4];
        fill(memory, offset, length, ...pixel);
        assert.deepEqual(actual, expected, `offset=${offset} length=${length} pixel=${pixel}`);
      }
    }
  }
});

test('actual helper acquires the current WASM memory after growth and preserves adjacent pixels', () => {
  const memory = new WebAssembly.Memory({ initial: 1 });
  fill(memory, 0, 52, 17, 31, 43, 97);
  memory.grow(1);
  fill(memory, 65536, 52, 255, 0, 128, 1);
  const bytes = new Uint8Array(memory.buffer);
  for (let at = 0; at < 52; at++) {
    assert.equal(bytes[at], [17, 31, 43, 97][at % 4]);
    assert.equal(bytes[65536 + at], [255, 0, 128, 1][at % 4]);
  }
  assert.equal(bytes[52], 0); assert.equal(bytes[65535], 0); assert.equal(bytes[65588], 0);
});
