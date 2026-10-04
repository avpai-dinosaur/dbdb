CXX = g++
ROOT = ../..
CXXFLAGS = -std=c++17 -Wall -Wextra -g -O0 -I$(ROOT)/include
LIB = $(ROOT)/build/libdbdb.a

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
