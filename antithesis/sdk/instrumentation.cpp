// Defines the LLVM sanitizer-coverage callbacks that `-fsanitize-coverage=trace-pc-guard`
// emits calls to, forwarding them to libvoidstar so Antithesis can observe coverage.
//
// The header must be included in exactly one translation unit per binary. Linking this
// file into every driver (see the sibling Makefile) means new drivers get it for free.
// Outside Antithesis the dlopen fails harmlessly and the callbacks become no-ops, so
// instrumented drivers still run normally on a developer machine.
#include "antithesis_instrumentation.h"
