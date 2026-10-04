CXX = g++
CXXFLAGS = -std=c++17 -Wall -Wextra -g -O0 -Iinclude

HDRS = $(wildcard include/dbdb/*.hpp)
LIB_SRCS = $(wildcard src/*.cpp)
LIB_OBJS = $(patsubst src/%.cpp,build/%.o,$(LIB_SRCS))
LIB = build/libdbdb.a

dbdb: cli/main.cpp $(LIB) $(HDRS)
	$(CXX) $(CXXFLAGS) -o dbdb cli/main.cpp -Lbuild -ldbdb

$(LIB): $(LIB_OBJS)
	ar rcs $@ $^

build/%.o: src/%.cpp $(HDRS)
	@mkdir -p build
	$(CXX) $(CXXFLAGS) -c $< -o $@

test: $(LIB) $(HDRS) $(wildcard tests/*.cpp tests/*.h)
	$(CXX) $(CXXFLAGS) -o run_tests $(wildcard tests/*.cpp) -Lbuild -ldbdb

run_tests: test
	./run_tests

clean:
	rm -rf build dbdb run_tests

.PHONY: test clean
