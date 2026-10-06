// Rust-owned, bounded allocations let void upstream kernels report failure.
// Pixel algorithms and their operation order remain unchanged.
#include <stddef.h>
void *phase7_malloc(size_t size);
void phase7_free(void *pointer);
#undef malloc
#undef free
#define malloc phase7_malloc
#define free phase7_free
