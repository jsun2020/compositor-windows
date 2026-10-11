import fs from 'node:fs';
import crypto from 'node:crypto';
import assert from 'node:assert/strict';

const sha = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const resource = fs.readFileSync('engine/native/CameraRawTables.bin');
assert.equal(sha(resource), 'e67ff4dbcce35cb7765752acff4405be5c934d913ef1396b2ca3fc6dc8a85190', 'Fixed Mac 1.4.9 LUT changed');
assert.equal(resource.toString('ascii',0,4),'CRT3');
assert.deepEqual([resource.readUInt16LE(4),resource.readUInt16LE(6),resource.readUInt16LE(8)],[17,763,3]);
assert.match(fs.readFileSync('engine/native/LICENSE-Compositor.txt','utf8'), /Permission is hereby granted/);
const files = ['engine/native/AdjustPixels.c','engine/native/AdjustPixels.h','engine/native/DitherPixels.c','engine/native/DitherPixels.h','engine/native/CameraRawTables.bin','src-tauri/native/RawQuantize.h'];
console.log(JSON.stringify({baseline:'Mac Compositor 1.4.9',upstream:'0ab3ff58a3dcf3d52a455fe3ab0336ac95923edd',fixedResourceVerified:true,files:files.map(name=>({name,sha256:sha(fs.readFileSync(name))}))},null,2));
