# Build context is this directory: dbdb-rust/ is self-contained.
#
# The result is an artifact carrier, not a runnable image. It publishes
# /out/bin and /out/symbols for the Antithesis harness to copy out, and has no
# entrypoint, no shell, and no knowledge that Antithesis exists beyond
# instrumenting its binary. See ../antithesis/CONTRACT.md.

FROM rust:1-slim-bookworm AS build
ENV NO_COLOR=1
# make drives cargo, and the instrumentation flags live in the Makefile rather
# than here so there is one copy of them. Nothing else has to be installed: the
# crate has no dependencies, and the cc and ar that build.rs needs for the
# instrumentation shim already come with the base image.
RUN apt-get update \
 && apt-get install -y --no-install-recommends make \
 && rm -rf /var/lib/apt/lists/*
# Fail the build loudly if the base image ever ships a toolchain older than
# std::fs::File::lock, rather than at a puzzling "no method named lock" in
# src/storage.rs.
RUN set -e; version=$(rustc --version | cut -d' ' -f2); \
    major=$(echo "$version" | cut -d. -f1); minor=$(echo "$version" | cut -d. -f2); \
    [ "$major" -gt 1 ] || [ "$minor" -ge 89 ] \
      || { echo "need rust >= 1.89 for File::lock, got $version"; exit 1; }
WORKDIR /src
COPY Makefile Cargo.toml Cargo.lock build.rs ./
COPY antithesis-sdk ./antithesis-sdk
COPY src ./src
COPY tests ./tests
# -> ./dbdb-instrumented, built with LLVM sanitizer coverage. The harness drives
# this one, so it is the only source of coverage feedback for the whole run.
RUN make ANTITHESIS=1
# The check CONTRACT.md asks for. Without this symbol the binary cannot reach
# libvoidstar, and a run would look perfectly healthy while the fuzzer searched
# blind -- which is why it fails the build here instead.
RUN make verify
# -> ./dbdb, uninstrumented, for poking at the database by hand in the multiverse
# debugger. Kept distinct so inspecting state from a debug shell can't be
# mistaken for the workload exercising new code.
RUN make
# tests/protocol.rs drives the binary over the same line protocol the harness
# uses, so a binary that cannot hold up its end of CONTRACT.md never ships.
RUN make test

FROM scratch
COPY --from=build /src/dbdb-instrumented /out/bin/dbdb
COPY --from=build /src/dbdb              /out/bin/dbdb-debug
# Unstripped copy of exactly the binary at /out/bin/dbdb, matched by build-id.
COPY --from=build /src/dbdb-instrumented /out/symbols/dbdb
