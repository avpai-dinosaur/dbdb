CXX = g++
ROOT = ../..
CXXFLAGS = -std=c++17 -Wall -Wextra -g -O0 -I$(ROOT)/include
LIB = $(ROOT)/build/libdbdb.a

# Every drivers/<name>.cpp becomes bin/<name>. Name sources with an Antithesis
# test command prefix (parallel_driver_, singleton_driver_, ...) so the binaries
# can be dropped straight into a test template directory.
SRCS = $(wildcard *.cpp)
BINS = $(patsubst %.cpp,bin/%,$(SRCS))

all: lib $(BINS)

# Let the top-level Makefile decide whether the library needs rebuilding.
lib:
	$(MAKE) -C $(ROOT) build/libdbdb.a

bin/%: %.cpp $(LIB)
	@mkdir -p bin
	$(CXX) $(CXXFLAGS) -o $@ $< -L$(ROOT)/build -ldbdb

clean:
	rm -rf bin

.PHONY: all lib clean
