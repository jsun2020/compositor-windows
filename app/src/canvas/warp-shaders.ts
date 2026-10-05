// Dedicated-context warp passes. Row zero represents the document's top row;
// no browser image conversion, premultiplication or texture filtering is used.
export const WARP_VERTEX = `#version 300 es
void main() {
  vec2 p = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2));
  gl_Position = vec4(p * 2.0 - 1.0, 0.0, 1.0);
}`;

const HEADER = `#version 300 es
precision highp float;
precision highp int;
uniform ivec2 origin;
uniform ivec2 planeSize;
`;
const SOURCE = `
uniform highp sampler2DArray original;
uniform int sourceSide;
uniform int sourceColumns;
vec4 sourceAt(ivec2 p) {
  if (any(lessThan(p, ivec2(0))) || any(greaterThanEqual(p, planeSize))) return vec4(0.0);
  ivec2 tile = p / sourceSide;
  return texelFetch(original, ivec3(p % sourceSide, tile.y * sourceColumns + tile.x), 0);
}
vec4 sourceBilinear(vec2 p) {
  p = clamp(p, vec2(0.0), vec2(planeSize - 1));
  ivec2 lo = ivec2(floor(p)); ivec2 hi = min(lo + 1, planeSize - 1);
  vec2 f = p - vec2(lo);
  vec4 a = sourceAt(lo) * 255.0;
  vec4 b = sourceAt(ivec2(hi.x, lo.y)) * 255.0;
  vec4 c = sourceAt(ivec2(lo.x, hi.y)) * 255.0;
  vec4 d = sourceAt(hi) * 255.0;
  vec4 upper = a + (b - a) * f.x;
  vec4 lower = c + (d - c) * f.x;
  return upper + (lower - upper) * f.y;
}`;
const TIP = `
uniform ivec2 center;
uniform float radius;
uniform float hardness;
uniform float strength;
float weightAt(ivec2 p) {
  vec2 delta = vec2(p - center);
  float u = sqrt(dot(delta, delta)) * (1.0 / radius);
  if (u >= 1.0) return 0.0;
  if (u <= hardness) return 1.0;
  float t = (1.0 - u) / (1.0 - hardness);
  return t * t * (3.0 - 2.0 * t);
}`;

export const WARP_COPY = HEADER + SOURCE + `
out vec4 color;
void main() { color = sourceAt(origin + ivec2(gl_FragCoord.xy)); }`;

export const WARP_PICKUP = HEADER + `
uniform highp sampler2D under;
out vec4 color;
void main() { color = floor(texelFetch(under, ivec2(gl_FragCoord.xy), 0) * 255.0 + 0.5); }`;

export const WARP_CARRY = HEADER + TIP + `
uniform highp sampler2D under;
uniform highp sampler2D carried;
out vec4 color;
void main() {
  ivec2 tip = ivec2(gl_FragCoord.xy), p = origin + tip;
  vec4 old = texelFetch(carried, tip, 0);
  float w = weightAt(p);
  if (w <= 0.0 || any(lessThan(p, ivec2(0))) || any(greaterThanEqual(p, planeSize))) { color = old; return; }
  vec4 base = floor(texelFetch(under, tip, 0) * 255.0 + 0.5);
  // Keep fractional painted colors; only the canvas rounds each dab.
  color = base + (old - base) * w * strength;
}`;

export const WARP_STAMP = HEADER + TIP + `
uniform highp sampler2D carried;
uniform ivec2 tipOrigin;
out vec4 color;
void main() {
  ivec2 p = origin + ivec2(gl_FragCoord.xy);
  if (weightAt(p) <= 0.0) discard;
  color = clamp(floor(texelFetch(carried, p - tipOrigin, 0) + 0.5), 0.0, 255.0) / 255.0;
}`;

export const WARP_OFFSET = HEADER + TIP + `
uniform highp sampler2D snapshot;
uniform ivec2 dependencyOrigin;
uniform ivec2 dependencySize;
uniform vec2 movement;
out vec2 offset;
vec2 fieldAt(ivec2 p) { return texelFetch(snapshot, p, 0).rg; }
void main() {
  ivec2 p = origin + ivec2(gl_FragCoord.xy);
  float w = weightAt(p); if (w <= 0.0) discard;
  vec2 delta = movement * w;
  vec2 q = clamp(vec2(p - dependencyOrigin) - delta, vec2(0.0), vec2(dependencySize - 1));
  ivec2 lo = ivec2(floor(q)), hi = min(lo + 1, dependencySize - 1); vec2 f = q - vec2(lo);
  vec2 a = fieldAt(lo), b = fieldAt(ivec2(hi.x, lo.y));
  vec2 c = fieldAt(ivec2(lo.x, hi.y)), d = fieldAt(hi);
  vec2 upper = a + (b - a) * f.x, lower = c + (d - c) * f.x;
  offset = upper + (lower - upper) * f.y - delta;
}`;

export const WARP_LIQUIFY_COLOR = HEADER + SOURCE + TIP + `
uniform highp sampler2D offsets;
out vec4 color;
void main() {
  ivec2 local = ivec2(gl_FragCoord.xy), p = origin + local;
  if (weightAt(p) <= 0.0) discard;
  vec2 offset = texelFetch(offsets, local, 0).rg;
  color = clamp(floor(sourceBilinear(vec2(p) + offset) + 0.5), 0.0, 255.0) / 255.0;
}`;
