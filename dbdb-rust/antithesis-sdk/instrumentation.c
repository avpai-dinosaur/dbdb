/*
Defines the sanitizer-coverage callbacks that a build with
-sanitizer-coverage-trace-pc-guard emits calls to, and forwards them to
libvoidstar so Antithesis can see which branches a timeline reached.

The header has to be included in exactly one translation unit per binary, and
this file is that unit. build.rs compiles it and links the result into the
binary; see ../build.rs for why it is C and not Rust.

Outside Antithesis the dlopen fails harmlessly and the callbacks become
no-ops, so an instrumented binary still runs normally on a laptop.
*/
#include "antithesis_instrumentation.h"
