# Note: build context is the repo root

# Build libdbdb.a and link every driver against it.
FROM debian:stable-slim AS build
ENV NO_COLOR=1
RUN apt-get update \
 && apt-get install -y --no-install-recommends g++ make \
 && rm -rf /var/lib/apt/lists/*
WORKDIR /src
COPY Makefile ./
COPY include ./include
COPY src ./src
COPY antithesis/drivers ./antithesis/drivers
RUN make -C antithesis/drivers

# Runtime: the drivers as test commands, idle until Antithesis runs them.
FROM debian:stable-slim AS drivers
ENV NO_COLOR=1
ENV DBDB_PATH=/data/dbdb.db
RUN mkdir -p /data
COPY --from=build /src/antithesis/drivers/bin/ /opt/antithesis/test/v1/main/
COPY antithesis/drivers/setup-complete.sh /opt/antithesis/setup-complete.sh
CMD ["bash", "-c", "/opt/antithesis/setup-complete.sh && exec sleep infinity"]
