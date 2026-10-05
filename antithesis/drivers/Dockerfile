# Note: build context is the repo root

# Build libdbdb.a and link every driver against it.
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
COPY antithesis/sdk ./antithesis/sdk
COPY antithesis/drivers ./antithesis/drivers
RUN make -C antithesis/drivers
# The dbdb CLI, for poking at the database by hand in the multiverse debugger.
# Built without ANTITHESIS=1 on purpose: an uninstrumented binary neither needs
# libvoidstar nor reports coverage, so inspecting state from a debug shell can't
# be mistaken for the workload exercising new code. Separate build dir
# (build/ vs build-antithesis/), so this does not disturb the drivers' library.
RUN make dbdb

# Runtime: the drivers as test commands, idle until Antithesis runs them.
FROM debian:stable-slim AS drivers
ENV NO_COLOR=1
ENV DBDB_PATH=/data/dbdb.db
RUN mkdir -p /data
# The instrumented drivers dlopen this at startup. This is the stub; Antithesis
# substitutes the real instrumentation library at test time.
COPY antithesis/lib/libvoidstar.so /usr/lib/libvoidstar.so
COPY --from=build /src/antithesis/drivers/bin/ /opt/antithesis/test/v1/main/
# Unstripped copies of the same binaries, so Antithesis can symbolize coverage.
COPY --from=build /src/antithesis/drivers/bin/ /symbols/
# On PATH, not in the test command directory -- Antithesis runs everything under
# /opt/antithesis/test/v1/main/ as a test command, and this is a debugging tool.
COPY --from=build /src/dbdb /usr/local/bin/dbdb
COPY antithesis/drivers/setup-complete.sh /opt/antithesis/setup-complete.sh
CMD ["bash", "-c", "/opt/antithesis/setup-complete.sh && exec sleep infinity"]
