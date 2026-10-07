# Vendored Antithesis SDK

Vendored from the Antithesis C++ SDK, version **0.5.0** — the same copy as
[`../../dbdb-cpp/antithesis-sdk/`](../../dbdb-cpp/antithesis-sdk/).

- `antithesis_instrumentation.h` — the sanitizer-coverage callbacks, forwarding
  to `/usr/lib/libvoidstar.so`. It is written to work in C as well as C++, which
  is what makes it usable from a Rust build.
- `instrumentation.c` — the single translation unit that includes it.
  [`../build.rs`](../build.rs) compiles this and links it into the binary.
- `../../antithesis/lib/libvoidstar.so` — the stub library. The harness image
  copies it to `/usr/lib/libvoidstar.so`; Antithesis substitutes the real one at
  test time.

Only the instrumentation header is vendored here. The rest of the C++ SDK is
assertions for a workload to call, and the workload for this database is the
Python under [`../../antithesis/test/`](../../antithesis/test/), which uses the
Antithesis Python SDK instead.

These are vendored rather than fetched during the image build so the build stays
hermetic — `dbdb-rust/` has no dependencies at all and never reaches the network.
To update, replace the header from
<https://github.com/antithesishq/antithesis-sdk-cpp> and bump the version above.
