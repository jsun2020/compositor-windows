import type { BlendMode } from "../../engine/types";

export const BLEND_INDEX: Record<BlendMode, number> = { Normal: 0, Multiply: 1, Screen: 2, Overlay: 3, Darken: 4, Lighten: 5, Difference: 6, "Color Dodge": 7, "Color Burn": 8, Hue: 9, Saturation: 10, Color: 11, Luminosity: 12 };
export const ADJUST_KIND: Record<string, number> = { identity: 0, tables: 1, gradientMap: 2, hsv: 3, grain: 4 };

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

// Ports rgb_to_hsl/hsl_to_rgb/adjust_rgb from engine/src/adjust/hsv.rs and mix32/lattice from
// engine/src/adjust/grain.rs. Every constant here must match those files exactly: the CPU
// compositor is the single definition of the maths, and this is the shader's mirror of it.
const ADJUST_GLSL = `
vec3 rgbToHsl(vec3 c) {
  float high = max(c.r, max(c.g, c.b)), low = min(c.r, min(c.g, c.b));
  float lightness = (high + low) * 0.5, delta = high - low;
  if (delta <= 0.0) return vec3(0.0, 0.0, lightness);
  float saturation = min(1.0, delta / (1.0 - abs(2.0 * lightness - 1.0)));
  float hue = high == c.r ? (c.g - c.b) / delta : (high == c.g ? (c.b - c.r) / delta + 2.0 : (c.r - c.g) / delta + 4.0);
  hue *= 60.0;
  if (hue < 0.0) hue += 360.0;
  return vec3(hue, saturation, lightness);
}
// Rust's '%' truncates toward zero, GLSL's mod() floors. They agree for non-negative values and
// disagree for negative ones, and the colorize branch below CAN produce a negative hue (Task 4
// deliberately dropped its negative wrap to match the Mac's truncatingRemainder). The parity
// constraint makes the CPU the reference, so this mirrors Rust's operator, not GLSL's. Used
// wherever hsv.rs uses '%' on a value that can be negative; the non-colorize path wraps negatives
// itself, which makes it equal to a floored mod, so mod() stays correct there.
float rem(float x, float y) { return x - y * trunc(x / y); }
vec3 hslToRgb(vec3 hsl) {
  if (hsl.y <= 0.0) return vec3(hsl.z);
  float chroma = (1.0 - abs(2.0 * hsl.z - 1.0)) * hsl.y;
  float sector = hsl.x / 60.0;
  float second = chroma * (1.0 - abs(rem(sector, 2.0) - 1.0));
  float base = hsl.z - chroma * 0.5;
  // Dispatch on the TRUNCATED INTEGER, mirroring hsv.rs's 'match sector as i64 { 0..=4, _ }'. A
  // 'sector < 1.0' comparison chain would send every negative sector to arm 0, while Rust sends
  // anything at or below -1 to the catch-all arm; for a colorize hue of -300 that is (chroma,
  // second, 0) against (chroma, 0, second), two different colours. int(trunc(x)) truncates toward
  // zero exactly as 'as i64' does.
  int s = int(trunc(sector));
  vec3 rgb = s == 0 ? vec3(chroma, second, 0.0) : s == 1 ? vec3(second, chroma, 0.0)
    : s == 2 ? vec3(0.0, chroma, second) : s == 3 ? vec3(0.0, second, chroma)
    : s == 4 ? vec3(second, 0.0, chroma) : vec3(chroma, 0.0, second);
  return clamp(rgb + base, 0.0, 1.0);
}
uint mix32(uint x) {
  x ^= x >> 16; x *= 0x7feb352du;
  x ^= x >> 15; x *= 0x846ca68bu;
  x ^= x >> 16;
  return x;
}
float lattice(int ix, int iy, uint seed) {
  uint h = mix32(uint(ix) * 0x9E3779B1u ^ mix32(uint(iy) * 0x85EBCA77u ^ seed));
  return float(h & 0xFFFFu) / 65535.0 + float(h >> 16) / 65535.0 - 1.0;
}`;

const FRAG_ADJUST = `#version 300 es
precision highp float;
uniform sampler2D src;
uniform sampler2D coverage;
uniform sampler2D lut;
uniform sampler2D response;
uniform bool useCoverage;
uniform bool colorize;
uniform float opacity;
uniform int mode;
uniform int kind;
uniform mat3 deviceToDoc;
uniform vec3 colorizeAmounts;   // hue, saturation, lightness of the SELECTED range (not Master)
uniform vec3 grain;             // size, roughness, strength
uniform uint grainSeed;
out vec4 color;
${BLEND_GLSL}
${ADJUST_GLSL}
vec3 throughTables(vec3 c) {
  return vec3(texture(lut, vec2((c.r * 255.0 + 0.5) / 256.0, 0.5)).r,
              texture(lut, vec2((c.g * 255.0 + 0.5) / 256.0, 0.5)).g,
              texture(lut, vec2((c.b * 255.0 + 0.5) / 256.0, 0.5)).b);
}
vec3 throughGradientMap(vec3 c) {
  int level = (2126 * int(floor(c.r * 255.0 + 0.5)) + 7152 * int(floor(c.g * 255.0 + 0.5)) + 722 * int(floor(c.b * 255.0 + 0.5)) + 5000) / 10000;
  return texture(lut, vec2((float(min(level, 255)) + 0.5) / 256.0, 0.5)).rgb;
}
vec3 throughHsl(vec3 c) {
  vec3 hsl = rgbToHsl(c);
  float lightnessAmount;
  if (colorize) {
    // rem(), not mod(): hsv.rs's colorize branch is a bare 'selected.hue % 360.0' with no
    // subsequent positive wrap (unlike the non-colorize branch below), so a negative hue must
    // stay negative here and reach hslToRgb's catch-all arm exactly as Rust's does.
    hsl.x = rem(colorizeAmounts.x, 360.0);
    hsl.y = clamp(colorizeAmounts.y / 100.0, 0.0, 1.0);
    lightnessAmount = colorizeAmounts.z / 100.0;
  } else {
    vec4 sampled = texelFetch(response, ivec2(clamp(int(floor(hsl.x + 0.5)), 0, 360), 0), 0);
    lightnessAmount = sampled.z / 100.0;
    hsl.x = mod(hsl.x + sampled.x, 360.0);
    if (hsl.x < 0.0) hsl.x += 360.0;
    hsl.y = clamp(hsl.y * (1.0 + sampled.y / 100.0), 0.0, 1.0);
  }
  float amount = clamp(lightnessAmount, -1.0, 1.0);
  hsl.z = amount >= 0.0 ? hsl.z + (1.0 - hsl.z) * amount : hsl.z * (1.0 + amount);
  return hslToRgb(vec3(hsl.x, hsl.y, clamp(hsl.z, 0.0, 1.0)));
}
vec3 throughGrain(vec3 c, vec2 at) {
  float size = grain.x > 0.0 ? grain.x : 1.0;
  float rough = clamp(grain.y / 100.0, 0.0, 1.0);
  uint fineSeed = mix32(grainSeed ^ 0xA511E9B3u);
  vec2 cell = floor(at / size);
  vec2 t = at / size - cell;
  t = t * t * (3.0 - 2.0 * t);
  int ix = int(cell.x), iy = int(cell.y);
  float n00 = lattice(ix, iy, grainSeed), n10 = lattice(ix + 1, iy, grainSeed);
  float n01 = lattice(ix, iy + 1, grainSeed), n11 = lattice(ix + 1, iy + 1, grainSeed);
  float top = n00 + (n10 - n00) * t.x, bottom = n01 + (n11 - n01) * t.x;
  float smoothNoise = (top + (bottom - top) * t.y) * 1.6;
  float fine = lattice(int(floor(at.x)), int(floor(at.y)), fineSeed);
  float noise = smoothNoise + (fine - smoothNoise) * rough;
  float level = min(1.0, dot(c, vec3(0.2126, 0.7152, 0.0722)));
  float delta = noise * grain.z * (0.4 + 2.4 * level * (1.0 - level)) / 255.0;
  return clamp(c + delta, 0.0, 1.0);
}
void main() {
  ivec2 at = ivec2(gl_FragCoord.xy);
  vec4 d = texelFetch(src, at, 0);
  if (d.a <= 0.0) { color = d; return; }
  float k = opacity * (useCoverage ? texelFetch(coverage, at, 0).r : 1.0);
  if (k <= 0.0) { color = d; return; }
  vec3 original = clamp(d.rgb / d.a, 0.0, 1.0);
  vec3 adjusted = original;
  if (kind == 1) adjusted = throughTables(original);
  else if (kind == 2) adjusted = throughGradientMap(original);
  else if (kind == 3) adjusted = throughHsl(original);
  else if (kind == 4) { vec3 p = deviceToDoc * vec3(gl_FragCoord.xy, 1.0); adjusted = throughGrain(original, p.xy / p.z); }
  if (mode != 0) adjusted = clamp(blendRgb(mode, original, adjusted), 0.0, 1.0);
  vec3 mixed = mix(original, clamp(adjusted, 0.0, 1.0), k);
  color = vec4(mixed * d.a, d.a);
}`;

export interface Program { program: WebGLProgram; uniforms: Record<string, WebGLUniformLocation | null>; }
export interface Programs { layer: Program; coverage: Program; alphaOf: Program; opaque: Program; restore: Program; blit: Program; checker: Program; adjust: Program; vao: WebGLVertexArrayObject; buffer: WebGLBuffer; }

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
    adjust: compile(gl, VERT_SCREEN, FRAG_ADJUST, ["src", "coverage", "useCoverage", "lut", "response", "opacity", "mode", "kind", "deviceToDoc", "colorize", "colorizeAmounts", "grain", "grainSeed"]),
    vao, buffer,
  };
  for (const p of [programs.layer, programs.coverage, programs.alphaOf, programs.opaque, programs.restore, programs.blit, programs.checker, programs.adjust]) {
    const loc = gl.getAttribLocation(p.program, "unit"); gl.enableVertexAttribArray(loc); gl.vertexAttribPointer(loc, 2, gl.FLOAT, false, 0, 0);
  }
  return programs;
}
export function disposePrograms(gl: WebGL2RenderingContext, p: Programs): void {
  for (const q of [p.layer, p.coverage, p.alphaOf, p.opaque, p.restore, p.blit, p.checker, p.adjust]) gl.deleteProgram(q.program);
  gl.deleteVertexArray(p.vao); gl.deleteBuffer(p.buffer);
}
