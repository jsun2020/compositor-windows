import type { BlendMode } from "../../engine/types";

export const BLEND_INDEX: Record<BlendMode, number> = { Normal: 0, Multiply: 1, Screen: 2, Overlay: 3, Darken: 4, Lighten: 5, Difference: 6, "Color Dodge": 7, "Color Burn": 8, Hue: 9, Saturation: 10, Color: 11, Luminosity: 12 };

const VERT_UNIT = `#version 300 es
in vec2 unit;
uniform mat3 unitToClip;
uniform vec4 uvRect;
uniform bool flipX;
uniform bool flipY;
out vec2 uv;
void main() {
  vec2 f = uvRect.xy + unit * uvRect.zw;
  vec2 lu = vec2(flipX ? 1.0 - f.x : f.x, flipY ? 1.0 - f.y : f.y);
  vec3 p = unitToClip * vec3(lu, 1.0);
  gl_Position = vec4(p.xy, 0.0, p.z);
  uv = unit;
}`;
const VERT_SCREEN = `#version 300 es
in vec2 unit;
out vec2 uv;
void main() { gl_Position = vec4(unit * 2.0 - 1.0, 0.0, 1.0); uv = unit; }`;

const BLEND_GLSL = `
float lum(vec3 c) { return dot(c, vec3(0.3, 0.59, 0.11)); }
vec3 clipColor(vec3 c) {
  float l = lum(c); float n = min(c.r, min(c.g, c.b)); float x = max(c.r, max(c.g, c.b));
  if (n < 0.0) c = l + (c - l) * l / max(l - n, 1e-6);
  if (x > 1.0) c = l + (c - l) * (1.0 - l) / max(x - l, 1e-6);
  return c;
}
vec3 setLum(vec3 c, float l) { return clipColor(c + (l - lum(c))); }
float sat(vec3 c) { return max(c.r, max(c.g, c.b)) - min(c.r, min(c.g, c.b)); }
vec3 setSat(vec3 c, float s) {
  float mx = max(c.r, max(c.g, c.b)); float mn = min(c.r, min(c.g, c.b));
  return mx > mn ? (c - mn) * s / (mx - mn) : vec3(0.0);
}
float sep(int mode, float cb, float cs) {
  if (mode == 1) return cb * cs;
  if (mode == 2) return cb + cs - cb * cs;
  if (mode == 3) { if (cb <= 0.5) return cs * 2.0 * cb; float s = 2.0 * cb - 1.0; return cs + s - cs * s; }
  if (mode == 4) return min(cb, cs);
  if (mode == 5) return max(cb, cs);
  if (mode == 6) return abs(cb - cs);
  if (mode == 7) return cb <= 0.0 ? 0.0 : (cs >= 1.0 ? 1.0 : min(1.0, cb / (1.0 - cs)));
  if (mode == 8) return cb >= 1.0 ? 1.0 : (cs <= 0.0 ? 0.0 : 1.0 - min(1.0, (1.0 - cb) / cs));
  return cs;
}
vec3 blendRgb(int mode, vec3 cb, vec3 cs) {
  if (mode == 9) return setLum(setSat(cs, sat(cb)), lum(cb));
  if (mode == 10) return setLum(setSat(cb, sat(cs)), lum(cb));
  if (mode == 11) return setLum(cs, lum(cb));
  if (mode == 12) return setLum(cb, lum(cs));
  return vec3(sep(mode, cb.r, cs.r), sep(mode, cb.g, cs.g), sep(mode, cb.b, cs.b));
}
vec4 compose(vec4 dst, vec4 src, int mode) {
  float ad = dst.a; float as = src.a;
  if (as <= 0.0) return dst;
  float outA = as + ad * (1.0 - as);
  if (mode == 0 || ad <= 0.0) return vec4(src.rgb + dst.rgb * (1.0 - as), outA);
  vec3 cb = dst.rgb / ad; vec3 cs = src.rgb / as;
  vec3 b = clamp(blendRgb(mode, cb, cs), 0.0, 1.0);
  return vec4(clamp(src.rgb * (1.0 - ad) + dst.rgb * (1.0 - as) + as * ad * b, 0.0, 1.0), outA);
}`;

const FRAG_LAYER = `#version 300 es
precision highp float;
in vec2 uv;
uniform sampler2D tex;
uniform sampler2D backdrop;
uniform sampler2D coverage;
uniform bool useCoverage;
uniform bool useBackdrop;
uniform float opacity;
uniform int mode;
out vec4 color;
${BLEND_GLSL}
void main() {
  ivec2 at = ivec2(gl_FragCoord.xy);
  float k = opacity * (useCoverage ? texelFetch(coverage, at, 0).r : 1.0);
  vec4 s = texture(tex, uv) * k;
  // Without a backdrop the target is a cleared buffer, so the destination is known to be zero.
  // Fetching it anyway would read outside the 1x1 placeholder, which GLSL ES 3.00 leaves
  // undefined; the value feeds compose() and would corrupt clipping coverage on any backend
  // that does not happen to return zero.
  vec4 d = useBackdrop ? texelFetch(backdrop, at, 0) : vec4(0.0);
  color = compose(d, s, mode);
}`;
const FRAG_COVERAGE = `#version 300 es
precision highp float;
uniform mat3 deviceToMask;
uniform vec2 maskSize;
uniform float background;
uniform sampler2D mask;
out vec4 color;
void main() {
  vec3 p = deviceToMask * vec3(gl_FragCoord.xy, 1.0);
  vec2 m = p.xy / p.z;
  float v = (m.x < 0.0 || m.y < 0.0 || m.x >= maskSize.x || m.y >= maskSize.y) ? background : texture(mask, m / maskSize).r;
  color = vec4(v, v, v, 1.0);
}`;
const FRAG_ALPHA_OF = `#version 300 es
precision highp float;
uniform sampler2D src;
out vec4 color;
void main() { float a = texelFetch(src, ivec2(gl_FragCoord.xy), 0).a; color = vec4(a, a, a, 1.0); }`;
const FRAG_OPAQUE = `#version 300 es
precision highp float;
uniform sampler2D src;
out vec4 color;
void main() { vec4 c = texelFetch(src, ivec2(gl_FragCoord.xy), 0); color = c.a > 0.0 ? vec4(c.rgb / c.a, 1.0) : vec4(0.0); }`;
const FRAG_RESTORE = `#version 300 es
precision highp float;
uniform sampler2D src;
uniform sampler2D alpha;
out vec4 color;
void main() { ivec2 at = ivec2(gl_FragCoord.xy); float a = texelFetch(alpha, at, 0).r; vec4 c = texelFetch(src, at, 0); color = vec4(c.rgb * a, a); }`;
const FRAG_BLIT = `#version 300 es
precision highp float;
uniform sampler2D src;
out vec4 color;
void main() { color = texelFetch(src, ivec2(gl_FragCoord.xy), 0); }`;
const FRAG_CHECKER = `#version 300 es
precision highp float;
in vec2 uv;
uniform vec2 sizePx;
uniform float cell;
out vec4 color;
void main() { vec2 p = floor(uv * sizePx / cell); float c = mod(p.x + p.y, 2.0) < 1.0 ? 0.80 : 0.95; color = vec4(c, c, c, 1.0); }`;

export interface Program { program: WebGLProgram; uniforms: Record<string, WebGLUniformLocation | null>; }
export interface Programs { layer: Program; coverage: Program; alphaOf: Program; opaque: Program; restore: Program; blit: Program; checker: Program; vao: WebGLVertexArrayObject; buffer: WebGLBuffer; }

function compile(gl: WebGL2RenderingContext, vert: string, frag: string, uniforms: string[]): Program {
  const make = (type: number, src: string) => { const s = gl.createShader(type)!; gl.shaderSource(s, src); gl.compileShader(s); if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(s) ?? "shader"); return s; };
  const program = gl.createProgram()!;
  gl.attachShader(program, make(gl.VERTEX_SHADER, vert)); gl.attachShader(program, make(gl.FRAGMENT_SHADER, frag)); gl.linkProgram(program);
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(program) ?? "link");
  const u: Record<string, WebGLUniformLocation | null> = {};
  for (const name of uniforms) u[name] = gl.getUniformLocation(program, name);
  return { program, uniforms: u };
}

export function createPrograms(gl: WebGL2RenderingContext): Programs {
  const vao = gl.createVertexArray()!; gl.bindVertexArray(vao);
  const buffer = gl.createBuffer()!; gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
  gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([0, 0, 1, 0, 0, 1, 1, 1]), gl.STATIC_DRAW);
  const programs: Programs = {
    layer: compile(gl, VERT_UNIT, FRAG_LAYER, ["unitToClip", "uvRect", "flipX", "flipY", "tex", "backdrop", "coverage", "useCoverage", "useBackdrop", "opacity", "mode"]),
    coverage: compile(gl, VERT_SCREEN, FRAG_COVERAGE, ["deviceToMask", "maskSize", "background", "mask"]),
    alphaOf: compile(gl, VERT_SCREEN, FRAG_ALPHA_OF, ["src"]),
    opaque: compile(gl, VERT_SCREEN, FRAG_OPAQUE, ["src"]),
    restore: compile(gl, VERT_SCREEN, FRAG_RESTORE, ["src", "alpha"]),
    blit: compile(gl, VERT_SCREEN, FRAG_BLIT, ["src"]),
    checker: compile(gl, VERT_UNIT, FRAG_CHECKER, ["unitToClip", "uvRect", "flipX", "flipY", "sizePx", "cell"]),
    vao, buffer,
  };
  for (const p of [programs.layer, programs.coverage, programs.alphaOf, programs.opaque, programs.restore, programs.blit, programs.checker]) {
    const loc = gl.getAttribLocation(p.program, "unit"); gl.enableVertexAttribArray(loc); gl.vertexAttribPointer(loc, 2, gl.FLOAT, false, 0, 0);
  }
  return programs;
}
export function disposePrograms(gl: WebGL2RenderingContext, p: Programs): void {
  for (const q of [p.layer, p.coverage, p.alphaOf, p.opaque, p.restore, p.blit, p.checker]) gl.deleteProgram(q.program);
  gl.deleteVertexArray(p.vao); gl.deleteBuffer(p.buffer);
}
