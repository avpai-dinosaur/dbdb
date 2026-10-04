CXX = g++
CXXFLAGS = -std=c++17 -Wall -Wextra -g -O0

# Library code shared by the CLI and the tests.
SRCS = src/binaryTree.cpp src/fileStorage.cpp
HDRS = $(wildcard src/*.hpp)

dbdb: src/main.cpp $(SRCS) $(HDRS)
	$(CXX) $(CXXFLAGS) -o dbdb src/main.cpp $(SRCS)

test: $(SRCS) $(HDRS) $(wildcard tests/*.cpp tests/*.h tests/*.hpp)
	$(CXX) $(CXXFLAGS) -o run_tests $(SRCS) $(wildcard tests/*.cpp)

run_tests: test
	./run_tests

clean:
	rm -f dbdb run_tests

.PHONY: test clean
