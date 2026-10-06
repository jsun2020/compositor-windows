// The original kernels' C runtime boundary on wasm32, provided by Rust.
// Algorithm bodies remain identical to Compositor v1.4.5.
#include <stddef.h>
#include <stdint.h>
#define malloc compositor_malloc
#define calloc compositor_calloc
#define free compositor_free
#define memcpy compositor_memcpy
#define memset compositor_memset
#define sqrt compositor_sqrt
#define log compositor_log
#define sin compositor_sin
#define cos compositor_cos
#define lround compositor_lround
#define labs compositor_labs
#define pow compositor_pow
#define exp2 compositor_exp2
#define fmod compositor_fmod
#define tanh compositor_tanh
#define hypot compositor_hypot
#define floor __builtin_floor
#define ceil __builtin_ceil
#define fabs __builtin_fabs
#define fabsf __builtin_fabsf
#define fmax __builtin_fmax
#define fmaxf __builtin_fmaxf
#define fmin __builtin_fmin
#define fminf __builtin_fminf
#define round __builtin_round
#define roundf __builtin_roundf
#define isfinite(x) __builtin_isfinite(x)
#define INFINITY __builtin_inff()
#define M_PI 3.14159265358979323846
void *compositor_malloc(size_t n);
void *compositor_calloc(size_t n, size_t size);
void compositor_free(void *p);
void *compositor_memcpy(void *target, const void *source, size_t n);
void *compositor_memset(void *target, int value, size_t n);
double compositor_sqrt(double x);
double compositor_log(double x);
double compositor_sin(double x);
double compositor_cos(double x);
long compositor_lround(double x);
long compositor_labs(long x);
double compositor_pow(double x, double y);
double compositor_exp2(double x);
double compositor_fmod(double x, double y);
double compositor_tanh(double x);
double compositor_hypot(double x, double y);
