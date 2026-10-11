import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { fileURLToPath } from 'node:url';

export function verifyWasmOutput(directory) {
  const wasmPath = path.join(directory, 'compositor_engine_bg.wasm');
  const bytes = fs.readFileSync(wasmPath);
  assert(WebAssembly.validate(bytes), 'Invalid or truncated engine WASM: ' + wasmPath);
  const exports = new Set(WebAssembly.Module.exports(new WebAssembly.Module(bytes)).map(entry => entry.name));
  const glue = fs.readFileSync(path.join(directory, 'compositor_engine.js'), 'utf8');
  const calls = new Set([...glue.matchAll(/\bwasm\.([A-Za-z0-9_]+)\s*\(/g)].map(match => match[1]));
  assert(calls.size > 0, 'Engine JavaScript contains no WASM calls');
  for (const name of calls) assert(exports.has(name), 'WASM export required by engine JavaScript is missing: ' + name);
  for (const name of ['wasmengine_new', 'wasmengine_composite', 'wasmengine_render_plan']) {
    assert(exports.has(name), 'Required editor engine export is missing: ' + name);
  }
  return { bytes: bytes.length, sha256: crypto.createHash('sha256').update(bytes).digest('hex'), checkedGlueCalls: calls.size };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const directory = process.argv[2] ?? fileURLToPath(new URL('../app/src/engine/pkg/', import.meta.url));
  console.log('Engine WASM binary and JavaScript exports verified: ' + JSON.stringify(verifyWasmOutput(directory)));
}
