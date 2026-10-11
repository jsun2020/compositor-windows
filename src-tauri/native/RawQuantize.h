// Compositor v1.4.9 DitherPixels.c (0ab3ff5), MIT. Same fixed triangular-noise quantization.
#pragma once
#include <cmath>
#include <cstdint>
#include <cstddef>
static inline float raw_hash_noise(size_t x, size_t y, uint32_t draw) {
    uint32_t h = (uint32_t)x * 0x9E3779B1u ^ (uint32_t)y * 0x85EBCA77u ^ draw * 0xC2B2AE3Du;
    h ^= h >> 15; h *= 0x2C1B3C6Du; h ^= h >> 12; h *= 0x297A2D39u; h ^= h >> 15;
    return (float)(h >> 8) / (float)(1u << 24);
}
static inline uint8_t raw_quantize16(uint16_t wide,size_t x,size_t y,uint32_t channel) {
  float value = (float)wide * (255.0f / 65535.0f) + raw_hash_noise(x,y,channel) + raw_hash_noise(x,y,channel+3) - 1.0f;
  long rounded=lroundf(value);return (uint8_t)(rounded<0?0:rounded>255?255:rounded);
}
