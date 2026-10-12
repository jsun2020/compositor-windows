import { describe, expect, it } from "vitest";
import { createPrograms, disposePrograms } from "../../src/canvas/gl/programs";

function driver(linkOkay: boolean) {
  const shaders: object[] = [], programs: object[] = [], deletedShaders: object[] = [], deletedPrograms: object[] = [];
  const vertices: object[] = [], buffers: object[] = [], deletedVertices: object[] = [], deletedBuffers: object[] = [];
  const gl = {
    VERTEX_SHADER: 1, FRAGMENT_SHADER: 2, LINK_STATUS: 3,
    ARRAY_BUFFER: 4, STATIC_DRAW: 5, FLOAT: 6,
    createShader: () => { const s = {}; shaders.push(s); return s; },
    shaderSource: () => {}, compileShader: () => {},
    createProgram: () => { const p = {}; programs.push(p); return p; },
    attachShader: () => {}, detachShader: () => {}, linkProgram: () => {},
    getProgramParameter: () => linkOkay, getProgramInfoLog: () => "driver rejected shader",
    getUniformLocation: () => null, getAttribLocation: () => 0,
    createVertexArray: () => { const v = {}; vertices.push(v); return v; }, bindVertexArray: () => {},
    createBuffer: () => { const b = {}; buffers.push(b); return b; }, bindBuffer: () => {}, bufferData: () => {},
    enableVertexAttribArray: () => {}, vertexAttribPointer: () => {},
    deleteShader: (s: object) => { deletedShaders.push(s); },
    deleteProgram: (p: object) => { deletedPrograms.push(p); },
    deleteVertexArray: (v: object) => { deletedVertices.push(v); },
    deleteBuffer: (b: object) => { deletedBuffers.push(b); },
  } as unknown as WebGL2RenderingContext;
  return { gl, shaders, programs, deletedShaders, deletedPrograms, vertices, buffers, deletedVertices, deletedBuffers };
}

describe("canvas program resource lifetime", () => {
  it("a rejected program leaves no shader, program, vertex array or buffer behind", () => {
    const d = driver(false);
    expect(() => createPrograms(d.gl)).toThrow("driver rejected shader");
    expect(new Set(d.deletedShaders)).toEqual(new Set(d.shaders));
    expect(new Set(d.deletedPrograms)).toEqual(new Set(d.programs));
    expect(d.deletedVertices).toEqual(d.vertices);
    expect(d.deletedBuffers).toEqual(d.buffers);
  });

  it("successful programs release temporary shaders and remain owned until renderer disposal", () => {
    const d = driver(true);
    const programs = createPrograms(d.gl);
    expect(new Set(d.deletedShaders)).toEqual(new Set(d.shaders));
    expect(d.deletedPrograms).toHaveLength(0);
    disposePrograms(d.gl, programs);
    expect(new Set(d.deletedPrograms)).toEqual(new Set(d.programs));
    expect(d.deletedPrograms).toHaveLength(d.programs.length);
    expect(d.deletedVertices).toEqual(d.vertices);
    expect(d.deletedBuffers).toEqual(d.buffers);
  });
});
