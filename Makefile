CXX = g++
CXXFLAGS = -std=c++17 -Wall -Wextra -g -O0

# Library code shared by the CLI and the tests.
SRCS = binaryTree.cpp fileStorage.cpp
HDRS = $(wildcard *.hpp)

dbdb: main.cpp $(SRCS) $(HDRS)
	$(CXX) $(CXXFLAGS) -o dbdb main.cpp $(SRCS)

test: $(SRCS) $(HDRS) $(wildcard tests/*.cpp tests/*.h tests/*.hpp)
	$(CXX) $(CXXFLAGS) -o run_tests $(SRCS) $(wildcard tests/*.cpp)

run_tests: test
	./run_tests

clean:
	rm -f dbdb run_tests

.PHONY: test clean
