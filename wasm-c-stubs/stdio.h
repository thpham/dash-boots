// Freestanding stubs — see stdlib.h header note. stdio paths (fprintf in
// debug blocks) are compiled out (XM_DEBUG=0) and gc-sections strips them.
#ifndef WASM_C_STUBS_STDIO_H
#define WASM_C_STUBS_STDIO_H
typedef struct _stub_FILE FILE;
extern FILE *stderr;
extern FILE *stdout;
int fprintf(FILE *stream, const char *fmt, ...);
#endif
