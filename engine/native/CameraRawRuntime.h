// Rust-owned, bounded allocations let void upstream kernels report failure.
// Pixel algorithms and their operation order remain unchanged.
#include <stddef.h>
void *phase7_malloc(size_t size);
void *phase7_calloc(size_t count, size_t size);
void phase7_free(void *pointer);
// Keep the Mac compiler's fused Glow accumulation on native and WASM alike.
float phase7_mul_addf(float a, float b, float c);
#undef malloc
#undef free
#undef calloc
#define malloc phase7_malloc
#define calloc phase7_calloc
#define free phase7_free
