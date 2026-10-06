# Build context is this directory: dbdb-python/ is self-contained.
#
# The result is an artifact carrier, not a runnable image. It publishes
# /out/bin and /out/symbols for the Antithesis harness to copy out.
# See ../antithesis/CONTRACT.md.

FROM debian:stable-slim AS build
ENV NO_COLOR=1
RUN apt-get update \
 && apt-get install -y --no-install-recommends python3 python3-pip \
 && rm -rf /var/lib/apt/lists/*
WORKDIR /build
COPY code/dbdb ./app/dbdb
# portalocker is the one runtime dependency (dbdb/physical.py). code/requirements.txt
# pins 0.4, from 2014, which no longer installs on a current Python; the
# lock()/unlock() API this uses is unchanged across 2.x and 3.x. That file is
# left alone so the upstream source stays untouched.
RUN pip3 install --no-cache-dir --break-system-packages --target ./app 'portalocker>=2,<4'
RUN printf 'import sys\nfrom dbdb.tool import main\nsys.exit(main(sys.argv))\n' > ./app/__main__.py
# The contract copies /out/bin and nothing else, so for an interpreted
# implementation the package tree has to travel *inside* the executable. A
# zipapp is exactly that: one file with a shebang, importing straight out of the
# zip. Everything bundled here is pure Python, so there is nothing to compile.
RUN mkdir -p /out/bin /out/symbols \
 && python3 -m zipapp app -p '/usr/bin/env python3' -o /out/bin/dbdb \
 && chmod +x /out/bin/dbdb
# Fail the build here, rather than at test time, if the archive cannot run.
RUN /out/bin/dbdb 2>&1 | grep -q 'Usage:' || { echo "zipapp is not runnable"; exit 1; }

FROM scratch
COPY --from=build /out/bin/ /out/bin/
# Nothing to symbolize: there is no compiled binary. The directory still has to
# exist, and this implementation reports no coverage at all -- see
# ../antithesis/CONTRACT.md, which is the reason to keep dbdb-cpp as the
# implementation the fuzzer actually searches against.
COPY --from=build /out/symbols/ /out/symbols/
