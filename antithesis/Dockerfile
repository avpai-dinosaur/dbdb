# Build context is this directory: antithesis/ is self-contained and names no
# implementation language anywhere.
#
# IMPL_IMAGE is any image satisfying CONTRACT.md -- it carries /out/bin/dbdb and
# /out/symbols/. Build the implementation first, then pass it in:
#
#   make -C antithesis            # defaults to dbdb-cpp
#   make -C antithesis IMPL=dbdb-rust
ARG IMPL_IMAGE
FROM ${IMPL_IMAGE} AS impl

FROM debian:stable-slim
ENV NO_COLOR=1
ENV DBDB_PATH=/data/dbdb.db
RUN apt-get update \
 && apt-get install -y --no-install-recommends python3 python3-pip \
 && rm -rf /var/lib/apt/lists/*
# The Antithesis Python SDK, which the test commands use for assertions.
# --break-system-packages: Debian marks its python3 as externally managed
# (PEP 668), and a virtualenv would buy nothing in a single-purpose image.
RUN pip3 install --no-cache-dir --break-system-packages antithesis
RUN mkdir -p /data
# The implementation's binary dlopens this at startup. This is the stub;
# Antithesis substitutes the real instrumentation library at test time.
COPY lib/libvoidstar.so /usr/lib/libvoidstar.so
# Everything the implementation publishes. These two lines are the entire
# coupling between the harness and the database it tests.
COPY --from=impl /out/bin/     /usr/local/bin/
COPY --from=impl /out/symbols/ /symbols/
COPY test/main/ /opt/antithesis/test/v1/main/
RUN chmod +x /usr/local/bin/dbdb* /opt/antithesis/test/v1/main/*.py
COPY setup-complete.sh /opt/antithesis/setup-complete.sh
CMD ["bash", "-c", "/opt/antithesis/setup-complete.sh && exec sleep infinity"]
