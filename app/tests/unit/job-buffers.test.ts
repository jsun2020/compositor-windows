// @vitest-environment jsdom
import {afterEach,expect,it,vi} from "vitest";
import {clearJobBuffers,releaseJobBuffer,takeJobBuffer,takeSpareJobBuffer} from "../../src/engine/job-buffers";
const SIZE=4*1024*1024;
afterEach(()=>{clearJobBuffers();vi.useRealTimers();});

it("moves ownership out of retired callers and reuses the pixels for the next same-size edit",()=>{
  const original=new ArrayBuffer(SIZE);new Uint8Array(original)[17]=123;
  releaseJobBuffer(original);expect(original.byteLength).toBe(0);
  releaseJobBuffer(original); // Installation and its caller's finally may both retire it.
  const next=takeJobBuffer(SIZE);expect(next.byteLength).toBe(SIZE);
  expect(new Uint8Array(next)[17]).toBe(123);
  expect(takeJobBuffer(SIZE)).not.toBe(next);
});

it("keeps at most the largest spare and does not give a differently-sized copy stale pixels",()=>{
  const largest=new ArrayBuffer(SIZE*2);new Uint8Array(largest)[0]=77;releaseJobBuffer(largest);
  const smaller=new ArrayBuffer(SIZE);releaseJobBuffer(smaller);expect(smaller.byteLength).toBe(0);
  const copy=takeJobBuffer(SIZE);expect(new Uint8Array(copy)[0]).toBe(0);
  expect(new Uint8Array(takeJobBuffer(SIZE*2))[0]).toBe(77);
});

it("releases the spare after the editor has been idle",()=>{
  vi.useFakeTimers();const original=new ArrayBuffer(SIZE);new Uint8Array(original)[0]=55;
  releaseJobBuffer(original);vi.advanceTimersByTime(30_000);
  expect(new Uint8Array(takeJobBuffer(SIZE))[0]).toBe(0);
});

it("releases its spare when the page is hidden or discarded",()=>{
  const original=new ArrayBuffer(SIZE);new Uint8Array(original)[0]=55;
  releaseJobBuffer(original);window.dispatchEvent(new Event("pagehide"));
  expect(new Uint8Array(takeJobBuffer(SIZE))[0]).toBe(0);
});

it("loans an existing exact-size spare without allocating or consuming a different size",()=>{
  expect(takeSpareJobBuffer(SIZE)).toBeNull();
  const bytes=new ArrayBuffer(SIZE);new Uint8Array(bytes)[31]=61;releaseJobBuffer(bytes);
  expect(takeSpareJobBuffer(SIZE*2)).toBeNull();
  const loan=takeSpareJobBuffer(SIZE)!;
  expect(new Uint8Array(loan)[31]).toBe(61);
  expect(takeSpareJobBuffer(SIZE)).toBeNull();
});
