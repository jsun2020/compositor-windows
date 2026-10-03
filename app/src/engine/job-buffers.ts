// Keep one transferred layer buffer for the next edit. Repeated 100 MP edits
// otherwise allocate and retire 400 MB on the UI thread each time, provoking GC
// during the following edit. Ownership still moves: retired callers are detached.
const MIN_BYTES = 4 * 1024 * 1024;
const MAX_BYTES = 100_000_000 * 4;
let spare: ArrayBuffer | null = null;
let timer: ReturnType<typeof setTimeout> | undefined;
type Movable = ArrayBuffer & { transfer?: (length?: number) => ArrayBuffer };

export function clearJobBuffers(): void {
  clearTimeout(timer); timer = undefined;
  (spare as Movable | null)?.transfer?.(0); spare = null;
}

// A discarded/bfcached page must not leave its 400 MB spare waiting for an idle
// timer that the browser may suspend. Resuming can allocate a fresh spare.
if(typeof window!=="undefined")window.addEventListener("pagehide",clearJobBuffers);

export function takeJobBuffer(size: number): ArrayBuffer {
  if (spare?.byteLength === size) {
    const buffer = spare; spare = null;
    clearTimeout(timer); timer = undefined;
    return buffer;
  }
  return new ArrayBuffer(size);
}

export function releaseJobBuffer(buffer: ArrayBuffer): void {
  const movable = buffer as Movable, size = buffer.byteLength;
  if (size < MIN_BYTES || !movable.transfer) return;
  if (size > MAX_BYTES || size < (spare?.byteLength ?? 0)) { movable.transfer(0); return; }
  clearJobBuffers();
  spare = movable.transfer();
  // An idle editor releases its spare instead of retaining a large image forever.
  timer = setTimeout(clearJobBuffers, 30_000);
}
