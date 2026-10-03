import type { BlendMode } from "../../engine/types";

// Mirrors BlendMode in engine/src/blend.rs. Every draw arrives in its own mode, an adjustment layer's
// and a stack base's too (Compositor 1.4.5 blends them through Core Image where Core Graphics cannot).
export const BLEND_INDEX: Record<BlendMode, number> = { Normal: 0, Multiply: 1, Screen: 2, Overlay: 3, Darken: 4, Lighten: 5, Difference: 6, "Color Dodge": 7, "Color Burn": 8, Hue: 9, Saturation: 10, Color: 11, Luminosity: 12,
  "Linear Burn": 13, "Linear Dodge (Add)": 14, "Soft Light": 15, "Hard Light": 16, "Vivid Light": 17, "Linear Light": 18, "Pin Light": 19, "Hard Mix": 20, Exclusion: 21, Subtract: 22, Divide: 23 };
export const ADJUST_KIND: Record<string, number> = { identity: 0, tables: 1, gradientMap: 2, hsv: 3, grain: 4, invert: 5, blackWhite: 6, colorBalance: 7, addNoise: 8 };

const VERT_UNIT = `#version 300 es
in vec2 unit;
uniform mat3 unitToClip;
uniform vec4 uvRect;
uniform vec2 edgePadding;
uniform bool flipX;
uniform bool flipY;
out vec2 uv;
void main() {
  vec2 lo = edgePadding * vec2(uvRect.x <= 0.0 ? 1.0 : 0.0, uvRect.y <= 0.0 ? 1.0 : 0.0);
  vec2 hi = edgePadding * vec2(uvRect.x + uvRect.z >= 1.0 ? 1.0 : 0.0, uvRect.y + uvRect.w >= 1.0 ? 1.0 : 0.0);
  vec2 f = uvRect.xy - lo + unit * (uvRect.zw + lo + hi);
  vec2 lu = vec2(flipX ? 1.0 - f.x : f.x, flipY ? 1.0 - f.y : f.y);
  vec3 p = unitToClip * vec3(lu, 1.0);
  gl_Position = vec4(p.xy, 0.0, p.z);
  uv = unit + (-lo + unit * (lo + hi)) / uvRect.zw;
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
  if (mode == 13) return max(0.0, cb + cs - 1.0);
  if (mode == 14) return min(1.0, cb + cs);
  if (mode == 15) {   // W3C / Core Image, as blend.rs soft_light
    if (cs <= 0.5) return cb - (1.0 - 2.0 * cs) * cb * (1.0 - cb);
    float d = cb <= 0.25 ? ((16.0 * cb - 12.0) * cb + 4.0) * cb : sqrt(cb);
    return cb + (2.0 * cs - 1.0) * (d - cb);
  }
  if (mode == 16) { if (cs <= 0.5) return cb * 2.0 * cs; float s = 2.0 * cs - 1.0; return cb + s - cb * s; }
  if (mode == 17) {
    if (cs <= 0.5) { float s = 2.0 * cs; return cb >= 1.0 ? 1.0 : (s <= 0.0 ? 0.0 : 1.0 - min(1.0, (1.0 - cb) / s)); }
    float s = 2.0 * cs - 1.0; return cb <= 0.0 ? 0.0 : (s >= 1.0 ? 1.0 : min(1.0, cb / (1.0 - s)));
  }
  if (mode == 18) return clamp(cb + 2.0 * cs - 1.0, 0.0, 1.0);
  if (mode == 19) return cs <= 0.5 ? min(cb, 2.0 * cs) : max(cb, 2.0 * cs - 1.0);
  if (mode == 20) return cb + cs > 1.0 + 0.5 / 255.0 ? 1.0 : 0.0;   // HARD_MIX_MARGIN
  if (mode == 21) return cb + cs - 2.0 * cb * cs;
  if (mode == 22) return max(0.0, cb - cs);
  if (mode == 23) return cs <= 0.0 ? (cb > 0.0 ? 1.0 : 0.0) : min(1.0, cb / cs);
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
uniform vec4 uvRect;
uniform vec2 edgePadding;
uniform vec4 copyGrid;
uniform float copyHeight;
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
  vec4 sampled;
  if (edgePadding.x > 0.0 || edgePadding.y > 0.0) {
    vec2 f = uvRect.xy + uv * uvRect.zw;
    vec2 edge = clamp(min(f, vec2(1.0) - f) / max(fwidth(f), vec2(1e-8)) + 0.5, 0.0, 1.0);
    float edgeCoverage = floor(edge.x * edge.y * 255.0) / 255.0;
    // Interpolated UVs can land one float below a texel boundary at a half-pixel
    // origin. Derive copy coordinates from device pixels to keep those ties exact.
    vec2 pixel = (vec2(gl_FragCoord.x, copyHeight - gl_FragCoord.y) - copyGrid.xy) / copyGrid.zw;
    sampled = texelFetch(tex, clamp(ivec2(floor(pixel)), ivec2(0), textureSize(tex, 0) - 1), 0);
    // Covered source bytes are rounded before blending with the backdrop.
    sampled = floor(sampled * edgeCoverage * 255.0 + 0.5) / 255.0;
  } else sampled = texture(tex, uv);
  vec4 s = sampled * k;
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
// draw_stack in engine/src/compositor.rs: the whole surface opaque, a clear pixel opaque black (layer_unpremultiply_opaque).
const FRAG_OPAQUE = `#version 300 es
precision highp float;
uniform sampler2D src;
out vec4 color;
void main() { vec4 c = texelFetch(src, ivec2(gl_FragCoord.xy), 0); color = c.a > 0.0 ? vec4(c.rgb / c.a, 1.0) : vec4(0.0, 0.0, 0.0, 1.0); }`;
const FRAG_RESTORE = `#version 300 es
precision highp float;
uniform sampler2D src;
uniform sampler2D alpha;
out vec4 color;
void main() { ivec2 at = ivec2(gl_FragCoord.xy); float a = texelFetch(alpha, at, 0).r; vec4 c = texelFetch(src, at, 0); color = vec4(c.rgb * a, a); }`;
const FRAG_BLIT = `#version 300 es
precision highp float;
uniform sampler2D src;
uniform ivec2 offset;   // the frame's position within the view (GlRenderer.screenPass)
out vec4 color;
void main() { color = texelFetch(src, ivec2(gl_FragCoord.xy) + offset, 0); }`;
const FRAG_CHECKER = `#version 300 es
precision highp float;
in vec2 uv;
uniform vec2 sizePx;
uniform float cell;
out vec4 color;
void main() { vec2 p = floor(uv * sizePx / cell); float c = mod(p.x + p.y, 2.0) < 1.0 ? 0.80 : 0.95; color = vec4(c, c, c, 1.0); }`;

// Ports rgb_to_hsl/hsl_to_rgb/adjust_rgb from engine/src/adjust/hsv.rs and mix32/lattice/grain_field
// from engine/src/adjust/grain.rs. Every constant here must match those files exactly: the CPU
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
// hsv.rs adjusted_saturation (HueSaturation.swift:342-348): below 0 scales toward grey, above 0
// divides by what is left, +100 takes any colour all the way.
float adjustedSaturation(float s, float amount) {
  float a = clamp(amount / 100.0, -1.0, 1.0);
  if (a <= 0.0) return max(0.0, s * (1.0 + a));
  if (a >= 1.0) return s > 0.0 ? 1.0 : 0.0;
  return min(1.0, s / (1.0 - a));
}
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
}
// grain_field in engine/src/adjust/grain.rs (AdjustPixels.c:47-60).
float grainField(vec2 at, float scale, uint seed) {
  vec2 cell = floor(at / scale);
  vec2 t = at / scale - cell;
  t = t * t * (3.0 - 2.0 * t);
  int ix = int(cell.x), iy = int(cell.y);
  float n00 = lattice(ix, iy, seed), n10 = lattice(ix + 1, iy, seed);
  float n01 = lattice(ix, iy + 1, seed), n11 = lattice(ix + 1, iy + 1, seed);
  float top = n00 + (n10 - n00) * t.x, bottom = n01 + (n11 - n01) * t.x;
  return (top + (bottom - top) * t.y) * 1.6;
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
uniform vec3 bwLow;        // reds, yellows, greens weights (/ 100)
uniform vec3 bwHigh;       // cyans, blues, magentas weights (/ 100)
uniform vec3 bwTint;       // on (0 or 1), hue in degrees, saturation 0..1
uniform vec3 cbShadows;    // cyan-red, magenta-green, yellow-blue (/ 100)
uniform vec3 cbMidtones;
uniform vec3 cbHighlights;
uniform bool cbPreserve;
uniform vec3 noiseParams;  // spread (amount / 100 * 127.5), gaussian (0 or 1), monochromatic (0 or 1)
uniform uint noiseSeed;
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
    hsl.y = adjustedSaturation(hsl.y, sampled.y);
  }
  float amount = clamp(lightnessAmount, -1.0, 1.0);
  hsl.z = amount >= 0.0 ? hsl.z + (1.0 - hsl.z) * amount : hsl.z * (1.0 + amount);
  return hslToRgb(vec3(hsl.x, hsl.y, clamp(hsl.z, 0.0, 1.0)));
}
vec3 throughGrain(vec3 c, vec2 at) {
  float size = grain.x > 0.0 ? grain.x : 1.0;
  float rough = clamp(grain.y / 100.0, 0.0, 1.0);
  uint fineSeed = mix32(grainSeed ^ 0xA511E9B3u);
  float smoothNoise = grainField(at, size, grainSeed);
  float fine = grainField(at, max(0.5, size * 0.35), fineSeed);
  float noise = smoothNoise + (fine - smoothNoise) * rough;
  float level = min(1.0, dot(c, vec3(0.2126, 0.7152, 0.0722)));
  float delta = noise * grain.z * (0.4 + 2.4 * level * (1.0 - level)) / 255.0;
  return clamp(c + delta, 0.0, 1.0);
}
// black_white_rgb in engine/src/adjust/tonal.rs (AdjustPixels.c:110-152).
float bwWeight(int i) { return i == 0 ? bwLow.x : i == 1 ? bwLow.y : i == 2 ? bwLow.z : i == 3 ? bwHigh.x : i == 4 ? bwHigh.y : bwHigh.z; }
vec3 throughBlackWhite(vec3 c) {
  float r = min(c.r, 1.0), g = min(c.g, 1.0), b = min(c.b, 1.0);
  float mx = max(r, max(g, b)), mn = min(r, min(g, b)), md = r + g + b - mx - mn;
  int primary; int secondary;
  if (mx == r) { primary = 0; secondary = g >= b ? 1 : 5; }
  else if (mx == g) { primary = 2; secondary = r >= b ? 1 : 3; }
  else { primary = 4; secondary = g >= r ? 3 : 5; }
  float gray = clamp(mn + (md - mn) * bwWeight(secondary) + (mx - md) * bwWeight(primary), 0.0, 1.0);
  if (bwTint.x < 0.5 || bwTint.z <= 0.0) return vec3(gray);
  float chroma = (1.0 - abs(2.0 * gray - 1.0)) * bwTint.z;
  float hp = mod(bwTint.y, 360.0) / 60.0;   // tintHue is 0..360, where mod and fmod agree
  float x = chroma * (1.0 - abs(mod(hp, 2.0) - 1.0));
  vec3 rgb = hp < 1.0 ? vec3(chroma, x, 0.0) : hp < 2.0 ? vec3(x, chroma, 0.0) : hp < 3.0 ? vec3(0.0, chroma, x)
    : hp < 4.0 ? vec3(0.0, x, chroma) : hp < 5.0 ? vec3(x, 0.0, chroma) : vec3(chroma, 0.0, x);
  return clamp(rgb + (gray - chroma / 2.0), 0.0, 1.0);
}
// tonal_weights and color_balance_rgb in engine/src/adjust/tonal.rs (AdjustPixels.c:156-196).
vec3 tonalWeights(float v) {
  float s = clamp((v - 0.333) / -0.25 + 0.5, 0.0, 1.0);
  float h = clamp((v + 0.333 - 1.0) / 0.25 + 0.5, 0.0, 1.0);
  float m = clamp((v - 0.333) / 0.25 + 0.5, 0.0, 1.0) * clamp((v + 0.333 - 1.0) / -0.25 + 0.5, 0.0, 1.0);
  return vec3(s, m, h) * 0.7;
}
vec3 throughColorBalance(vec3 c) {
  c = min(c, vec3(1.0));
  float before = dot(c, vec3(0.299, 0.587, 0.114));
  vec3 outc;
  for (int i = 0; i < 3; i++) {
    vec3 w = tonalWeights(c[i]);
    outc[i] = clamp(c[i] + cbShadows[i] * w.x + cbMidtones[i] * w.y + cbHighlights[i] * w.z, 0.0, 1.0);
  }
  if (cbPreserve) {
    float after = dot(outc, vec3(0.299, 0.587, 0.114));
    if (after > 0.0001) outc = clamp(outc * (before / after), 0.0, 1.0);
  }
  return outc;
}
// noise_base and noise_offset in engine/src/adjust/filters.rs (NoisePixels.c:5-49); noise_hash is mix32.
float noiseUnit(uint key) { return float(mix32(key) >> 8u) * (1.0 / 16777216.0); }
vec3 throughNoise(vec3 c, vec2 at) {
  uint px = uint(int(floor(at.x))), py = uint(int(floor(at.y)));
  uint base = mix32(noiseSeed ^ mix32(px * 0x9e3779b9u ^ mix32(py * 0x85ebca6bu)));
  vec3 outc;
  for (int i = 0; i < 3; i++) {
    uint key = noiseParams.z > 0.5 ? base : base + uint(i) * 0x9e3779b9u;
    float n = noiseParams.y > 0.5
      ? sqrt(-2.0 * log(1.0 - noiseUnit(key))) * cos(6.2831853 * noiseUnit(key ^ 0x68e31da4u)) * noiseParams.x * (2.0 / 3.0)
      : (noiseUnit(key) * 2.0 - 1.0) * noiseParams.x;
    outc[i] = clamp(c[i] * 255.0 + n, 0.0, 255.0) / 255.0;
  }
  return outc;
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
  else if (kind == 5) adjusted = vec3(1.0) - original;
  else if (kind == 6) adjusted = throughBlackWhite(original);
  else if (kind == 7) adjusted = throughColorBalance(original);
  else if (kind == 8) { vec3 p = deviceToDoc * vec3(gl_FragCoord.xy, 1.0); adjusted = throughNoise(original, p.xy / p.z); }
  if (mode != 0) adjusted = clamp(blendRgb(mode, original, adjusted), 0.0, 1.0);
  vec3 mixed = mix(original, clamp(adjusted, 0.0, 1.0), k);
  color = vec4(mixed * d.a, d.a);
}`;

// Raster::halved: each pixel the average of the 2 x 2 block that exists.
const FRAG_HALVE = `#version 300 es
precision highp float;
uniform sampler2D src;
uniform ivec2 size;
out vec4 color;
void main() {
  ivec2 at = ivec2(gl_FragCoord.xy) * 2;
  vec4 sum = vec4(0.0); float n = 0.0;
  for (int dy = 0; dy < 2; dy++) for (int dx = 0; dx < 2; dx++) {
    ivec2 p = at + ivec2(dx, dy);
    if (p.x < size.x && p.y < size.y) { sum += texelFetch(src, p, 0); n += 1.0; }
  }
  color = sum / max(n, 1.0);
}`;
// gaussian_blur in engine/src/adjust/filters.rs: one axis per pass, taps beyond the raster count as
// transparent but stay in the sum; the second pass keeps colour within alpha.
const FRAG_GAUSSIAN = `#version 300 es
precision highp float;
uniform sampler2D src;
uniform float sigma;
uniform int radius;
uniform bool horizontal;
uniform bool last;
uniform ivec2 size;
out vec4 color;
void main() {
  ivec2 at = ivec2(gl_FragCoord.xy);
  vec4 acc = vec4(0.0); float sum = 0.0;
  for (int k = -radius; k <= radius; k++) {
    float w = exp(-float(k * k) / (2.0 * sigma * sigma));
    sum += w;
    ivec2 p = horizontal ? ivec2(at.x + k, at.y) : ivec2(at.x, at.y + k);
    if (p.x < 0 || p.y < 0 || p.x >= size.x || p.y >= size.y) continue;
    acc += texelFetch(src, p, 0) * w;
  }
  vec4 c = acc / sum;
  color = last ? vec4(min(c.rgb, vec3(c.a)), c.a) : c;
}`;
// motion_blur in engine/src/adjust/filters.rs: CIMotionBlur's Gaussian along the angle, one bilinear
// tap per whole pixel out to `radius` (ceil(3 sigma)) either side, weighted exp(-t^2 / 2 sigma^2),
// zero beyond the raster (sample_zero). dir is (cos a, sin a): these rows run bottom-up, the CPU's
// top-down.
const FRAG_MOTION = `#version 300 es
precision highp float;
uniform sampler2D src;
uniform vec2 dir;
uniform float sigma;
uniform int radius;
uniform ivec2 size;
out vec4 color;
vec4 fetchZero(ivec2 p) { return (p.x < 0 || p.y < 0 || p.x >= size.x || p.y >= size.y) ? vec4(0.0) : texelFetch(src, p, 0); }
vec4 bilinearZero(vec2 q) {
  vec2 f = q - 0.5; vec2 b = floor(f); vec2 t = f - b; ivec2 i = ivec2(b);
  return mix(mix(fetchZero(i), fetchZero(i + ivec2(1, 0)), t.x), mix(fetchZero(i + ivec2(0, 1)), fetchZero(i + ivec2(1, 1)), t.x), t.y);
}
void main() {
  vec4 acc = vec4(0.0); float sum = 0.0;
  for (int k = -radius; k <= radius; k++) {
    float w = exp(-float(k * k) / (2.0 * sigma * sigma));
    sum += w;
    acc += bilinearZero(gl_FragCoord.xy + dir * float(k)) * w;
  }
  vec4 c = acc / sum;
  color = vec4(min(c.rgb, vec3(c.a)), c.a);
}`;
// spatial_target in engine/src/compositor.rs: the blurred copy enlarged (spatial.rs `enlarged`),
// blended keeping the original alpha when the layer's own mode is not Normal (keepsAlpha,
// blended_keeping_alpha), moved toward by the layer's coverage (toward), and nothing written past
// the canvas.
const FRAG_SPATIAL_MIX = `#version 300 es
precision highp float;
uniform sampler2D original;
uniform sampler2D adjusted;
uniform sampler2D coverage;
uniform bool useCoverage;
uniform float opacity;
uniform int mode;
uniform bool keepsAlpha;
uniform int level;
uniform ivec2 adjustedSize;
uniform ivec2 beyond;   // frame columns from beyond.x, and GL rows below beyond.y, lie past the canvas
out vec4 color;
${BLEND_GLSL}
vec4 fetchClamped(ivec2 p) { return texelFetch(adjusted, clamp(p, ivec2(0), adjustedSize - 1), 0); }
vec4 enlarged(ivec2 at) {
  if (level == 0) return texelFetch(adjusted, at, 0);
  vec2 q = (vec2(at) + 0.5) / exp2(float(level)) - 0.5;
  vec2 b = floor(q); vec2 t = q - b; ivec2 i = ivec2(b);
  vec4 c = mix(mix(fetchClamped(i), fetchClamped(i + ivec2(1, 0)), t.x), mix(fetchClamped(i + ivec2(0, 1)), fetchClamped(i + ivec2(1, 1)), t.x), t.y);
  return vec4(min(c.rgb, vec3(c.a)), c.a);
}
vec3 opaqueOf(vec4 p) {
  uint a = uint(p.a * 255.0 + 0.5);
  if (a == 0u) return vec3(0.0);
  uvec3 c = uvec3(p.rgb * 255.0 + 0.5);
  return vec3(min((c * 255u + a / 2u) / a, uvec3(255u))) / 255.0;
}
void main() {
  ivec2 at = ivec2(gl_FragCoord.xy);
  vec4 o = texelFetch(original, at, 0);
  if (at.x >= beyond.x || at.y < beyond.y) { color = o; return; }
  float k = opacity * (useCoverage ? texelFetch(coverage, at, 0).r : 1.0);
  if (k <= 0.0) { color = o; return; }
  vec4 r = enlarged(at);
  if (keepsAlpha) {
    vec3 b = clamp(blendRgb(mode, opaqueOf(o), opaqueOf(r)), 0.0, 1.0);
    uint a = uint(o.a * 255.0 + 0.5);
    r = vec4(vec3((uvec3(b * 255.0 + 0.5) * a + 127u) / 255u) / 255.0, o.a);
  }
  vec4 m = k >= 1.0 ? r : mix(o, r, k);
  color = vec4(min(m.rgb, vec3(m.a)), m.a);
}`;

export interface Program { program: WebGLProgram; uniforms: Record<string, WebGLUniformLocation | null>; }
export interface Programs { layer: Program; coverage: Program; alphaOf: Program; opaque: Program; restore: Program; blit: Program; checker: Program; adjust: Program;
  halve: Program; gaussian: Program; motion: Program; spatialMix: Program; vao: WebGLVertexArrayObject; buffer: WebGLBuffer; }

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
    layer: compile(gl, VERT_UNIT, FRAG_LAYER, ["unitToClip", "uvRect", "edgePadding", "copyGrid", "copyHeight", "flipX", "flipY", "tex", "backdrop", "coverage", "useCoverage", "useBackdrop", "opacity", "mode"]),
    coverage: compile(gl, VERT_SCREEN, FRAG_COVERAGE, ["deviceToMask", "maskSize", "background", "mask"]),
    alphaOf: compile(gl, VERT_SCREEN, FRAG_ALPHA_OF, ["src"]),
    opaque: compile(gl, VERT_SCREEN, FRAG_OPAQUE, ["src"]),
    restore: compile(gl, VERT_SCREEN, FRAG_RESTORE, ["src", "alpha"]),
    blit: compile(gl, VERT_SCREEN, FRAG_BLIT, ["src", "offset"]),
    checker: compile(gl, VERT_UNIT, FRAG_CHECKER, ["unitToClip", "uvRect", "flipX", "flipY", "sizePx", "cell"]),
    adjust: compile(gl, VERT_SCREEN, FRAG_ADJUST, ["src", "coverage", "useCoverage", "lut", "response", "opacity", "mode", "kind", "deviceToDoc", "colorize", "colorizeAmounts", "grain", "grainSeed",
      "bwLow", "bwHigh", "bwTint", "cbShadows", "cbMidtones", "cbHighlights", "cbPreserve", "noiseParams", "noiseSeed"]),
    halve: compile(gl, VERT_SCREEN, FRAG_HALVE, ["src", "size"]),
    gaussian: compile(gl, VERT_SCREEN, FRAG_GAUSSIAN, ["src", "sigma", "radius", "horizontal", "last", "size"]),
    motion: compile(gl, VERT_SCREEN, FRAG_MOTION, ["src", "dir", "sigma", "radius", "size"]),
    spatialMix: compile(gl, VERT_SCREEN, FRAG_SPATIAL_MIX, ["original", "adjusted", "coverage", "useCoverage", "opacity", "mode", "keepsAlpha", "level", "adjustedSize", "beyond"]),
    vao, buffer,
  };
  for (const p of [programs.layer, programs.coverage, programs.alphaOf, programs.opaque, programs.restore, programs.blit, programs.checker, programs.adjust,
    programs.halve, programs.gaussian, programs.motion, programs.spatialMix]) {
    const loc = gl.getAttribLocation(p.program, "unit"); gl.enableVertexAttribArray(loc); gl.vertexAttribPointer(loc, 2, gl.FLOAT, false, 0, 0);
  }
  return programs;
}
export function disposePrograms(gl: WebGL2RenderingContext, p: Programs): void {
  for (const q of [p.layer, p.coverage, p.alphaOf, p.opaque, p.restore, p.blit, p.checker, p.adjust, p.halve, p.gaussian, p.motion, p.spatialMix]) gl.deleteProgram(q.program);
  gl.deleteVertexArray(p.vao); gl.deleteBuffer(p.buffer);
}
