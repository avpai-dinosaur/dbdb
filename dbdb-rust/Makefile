# cargo does the building; this exists so nobody has to remember the
# instrumentation flags, which are the one thing in here that is easy to get
# wrong and impossible to notice afterwards -- a run with an uninstrumented
# binary looks perfectly healthy while the fuzzer searches blind.

CARGO = cargo

# Always build for an explicit target, even though it is the host one. Without
# --target, cargo hands RUSTFLAGS to build scripts as well, and build.rs would
# then be compiled with the coverage flags and fail to link against the very
# shim it is supposed to be producing.
TARGET ?= $(shell rustc -vV | sed -n 's/^host: //p')

# ANTITHESIS=1 turns on LLVM sanitizer coverage, so Antithesis can see which
# branches a test exercised. Rust has no -fsanitize-coverage of its own, so the
# flags go to LLVM directly.
ANTITHESIS ?= 0
ifeq ($(ANTITHESIS),1)
  # --build-id stamps the binary so Antithesis can match it to the unstripped
  # copy in /symbols. One codegen unit keeps the instrumentation from being
  # split across several guard regions.
  RUST_FLAGS = -Cpasses=sancov-module \
               -Cllvm-args=-sanitizer-coverage-level=3 \
               -Cllvm-args=-sanitizer-coverage-trace-pc-guard \
               -Clink-arg=-Wl,--build-id \
               -Ccodegen-units=1
  # Its own target directory, so flipping ANTITHESIS does not make cargo rebuild
  # the world each time, and its own name, so the two can sit in one image.
  TARGET_DIR = target-antithesis
  CLI = dbdb-instrumented
else
  RUST_FLAGS =
  TARGET_DIR = target
  CLI = dbdb
endif

SRCS = $(wildcard src/*.rs) build.rs Cargo.toml Cargo.lock \
       antithesis-sdk/instrumentation.c antithesis-sdk/antithesis_instrumentation.h

# Default goal: ./dbdb normally, ./dbdb-instrumented under ANTITHESIS=1.
$(CLI): $(SRCS)
	RUSTFLAGS="$(RUST_FLAGS)" $(CARGO) build --release --locked --offline \
	    --target $(TARGET) --target-dir $(TARGET_DIR)
	cp $(TARGET_DIR)/$(TARGET)/release/dbdb $(CLI)

# Uninstrumented, because the coverage flags have nothing to do with whether the
# database is correct and only slow the tests down.
test:
	$(CARGO) test --locked --offline --target $(TARGET)

# What the contract says to check: if this symbol is missing, the binary cannot
# reach libvoidstar and the run reports no coverage.
verify: dbdb-instrumented
	@nm dbdb-instrumented | grep -q antithesis_load_libvoidstar \
	  && echo "dbdb-instrumented: instrumented" \
	  || { echo "dbdb-instrumented: NOT instrumented"; exit 1; }

clean:
	rm -rf target target-antithesis dbdb dbdb-instrumented

.PHONY: test verify clean
