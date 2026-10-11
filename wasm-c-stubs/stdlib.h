// Freestanding libc stubs for wasm32-unknown-unknown: only declarations.
// All symbols are provided at link time by Rust (dlmalloc for heap,
// compiler_builtins for memset/memcpy/memmove).
#ifndef WASM_C_STUBS_STDLIB_H
#define WASM_C_STUBS_STDLIB_H
#include <stddef.h>
void *malloc(size_t size);
void *calloc(size_t nmemb, size_t size);
void *realloc(void *ptr, size_t size);
void free(void *ptr);
void abort(void) __attribute__((noreturn));
void exit(int status) __attribute__((noreturn));
int abs(int x);
#endif
