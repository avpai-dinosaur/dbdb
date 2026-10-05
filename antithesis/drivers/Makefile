CXX = clang++
ROOT = ../..
BUILD = $(ROOT)/build-antithesis
LIB = $(BUILD)/libdbdb.a

CXXFLAGS = -std=c++20 -Wall -Wextra -g -O0 -I$(ROOT)/include -I$(ROOT)/antithesis/sdk \
           -fsanitize-coverage=trace-pc-guard
# --build-id stamps each binary with a unique ID, which is how Antithesis matches
# a running binary to the unstripped copy shipped in /symbols.
#
# -fno-sanitize-link-runtime: -fsanitize-coverage makes clang's driver auto-link
# libclang_rt.ubsan_standalone, which carries compiler-rt's own
# __sanitizer_cov_trace_pc_guard. We don't want it -- support/instrumentation.cpp
# defines those callbacks and forwards them to libvoidstar. Dropping it also
# avoids depending on libclang-rt-dev, which Debian's clang package omits.
LDFLAGS = -Wl,--build-id -fno-sanitize-link-runtime

# The sanitizer-coverage callbacks must be defined in exactly one translation
# unit per binary. Linking this into every driver means new drivers get
# instrumentation without having to remember the include.
SUPPORT = $(ROOT)/antithesis/sdk/instrumentation.cpp

SRCS = $(wildcard *.cpp)
BINS = $(patsubst %.cpp,bin/%,$(SRCS))

all: lib $(BINS)

# Let the top-level Makefile decide whether the library needs rebuilding.
lib:
	$(MAKE) -C $(ROOT) ANTITHESIS=1 build-antithesis/libdbdb.a

bin/%: %.cpp $(SUPPORT) $(LIB)
	@mkdir -p bin
	$(CXX) $(CXXFLAGS) $(LDFLAGS) -o $@ $< $(SUPPORT) -L$(BUILD) -ldbdb

clean:
	rm -rf bin

.PHONY: all lib clean
