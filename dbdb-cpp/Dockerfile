# Build context is this directory: dbdb-cpp/ is self-contained.
#
# The result is an artifact carrier, not a runnable image. It publishes
# /out/bin and /out/symbols for the Antithesis harness to copy out, and has no
# entrypoint, no shell, and no knowledge that Antithesis exists beyond
# instrumenting its binary. See ../antithesis/CONTRACT.md.

# clang, not g++: the Antithesis C++ SDK requires clang 16+ and C++20.
FROM debian:stable-slim AS build
ENV NO_COLOR=1
RUN apt-get update \
 && apt-get install -y --no-install-recommends clang make \
 && rm -rf /var/lib/apt/lists/*
# Fail the build loudly if the base image ever ships a clang older than the SDK
# supports, rather than at the #error buried in antithesis_sdk.h.
RUN v=$(clang -dumpversion | cut -d. -f1) \
 && [ "$v" -ge 16 ] || { echo "need clang >= 16 for the Antithesis C++ SDK, got $v"; exit 1; }
WORKDIR /src
COPY Makefile ./
COPY include ./include
COPY src ./src
COPY cli ./cli
COPY antithesis-sdk ./antithesis-sdk
# -> ./dbdb-instrumented, built with -fsanitize-coverage=trace-pc-guard. The
# harness drives this one, so it is the only source of coverage feedback.
RUN make ANTITHESIS=1
# -> ./dbdb, uninstrumented, for poking at the database by hand in the multiverse
# debugger. Kept distinct so inspecting state from a debug shell can't be
# mistaken for the workload exercising new code.
RUN make

FROM scratch
COPY --from=build /src/dbdb-instrumented /out/bin/dbdb
COPY --from=build /src/dbdb             /out/bin/dbdb-debug
# Unstripped copy of exactly the binary at /out/bin/dbdb, matched by build-id.
COPY --from=build /src/dbdb-instrumented /out/symbols/dbdb
