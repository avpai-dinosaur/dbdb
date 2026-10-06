# Note: build context is the repo root

FROM debian:stable-slim AS build
ENV NO_COLOR=1
RUN apt-get update \
 && apt-get install -y --no-install-recommends clang make \
 && rm -rf /var/lib/apt/lists/*

# Fail loudly if base image ships a clang older than what SDK supports
RUN v=$(clang -dumpversion | cut -d. -f1) \
 && [ "$v" -ge 16 ] || { echo "need clang >= 16 for the Antithesis C++ SDK, got $v"; exit 1; }

WORKDIR /src
COPY Makefile ./
COPY include ./include
COPY src ./src
COPY cli ./cli
COPY antithesis/sdk ./antithesis/sdk
RUN make ANTITHESIS=1
RUN make

FROM debian:stable-slim AS drivers
ENV NO_COLOR=1
ENV DBDB_PATH=/data/dbdb.db
RUN apt-get update \
 && apt-get install -y --no-install-recommends python3 python3-pip \
 && rm -rf /var/lib/apt/lists/*
RUN pip3 install --no-cache-dir --break-system-packages antithesis
RUN mkdir -p /data
COPY antithesis/lib/libvoidstar.so /usr/lib/libvoidstar.so
COPY --from=build /src/dbdb-instrumented /usr/local/bin/dbdb
COPY --from=build /src/dbdb-instrumented /symbols/dbdb
COPY --from=build /src/dbdb /usr/local/bin/dbdb-debug
COPY antithesis/test/main/ /opt/antithesis/test/v1/main/
RUN chmod +x /opt/antithesis/test/v1/main/*.py
COPY antithesis/drivers/setup-complete.sh /opt/antithesis/setup-complete.sh
CMD ["bash", "-c", "/opt/antithesis/setup-complete.sh && exec sleep infinity"]
