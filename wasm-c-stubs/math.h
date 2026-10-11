// Freestanding stubs — see stdlib.h header note. libxm uses sqrtf, fabs,
// fabsf, sinf, powf; wasm lowers these to LLVM intrinsics / compiler_builtins.
#ifndef WASM_C_STUBS_MATH_H
#define WASM_C_STUBS_MATH_H
double fabs(double x);
float fabsf(float x);
double sqrt(double x);
float sqrtf(float x);
double sin(double x);
float sinf(float x);
double cos(double x);
float cosf(float x);
double pow(double x, double y);
float powf(float x, float y);
double exp(double x);
float expf(float x);
double log(double x);
float logf(float x);
int abs(int x);
#endif
