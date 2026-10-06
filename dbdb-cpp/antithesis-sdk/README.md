# Vendored Antithesis C++ SDK

Vendored from the Antithesis C++ SDK, version **0.5.0**.

- `antithesis_instrumentation.h` — sanitizer-coverage callbacks that forward to
  `/usr/lib/libvoidstar.so`. Included by `instrumentation.cpp`.
- `antithesis_sdk.h` — assertions and lifecycle hooks, for use by the workload.
- `../lib/libvoidstar.so` — the stub instrumentation library. It is copied to
  `/usr/lib/libvoidstar.so` in the drivers image; Antithesis substitutes the real
  implementation at test time.

These are vendored rather than fetched during the image build so the build stays
hermetic. To update, replace the headers from
<https://github.com/antithesishq/antithesis-sdk-cpp> and bump the version above.

Both headers require **clang 16+** and **C++20** (`antithesis_sdk.h` enforces this
with `#error`); that is why the build uses `clang++ -std=c++20` rather than g++.
