CXX = clang++

# ANTITHESIS=1 compiles the library with LLVM sanitizer coverage so the
# Antithesis fuzzer can observe which edges a test exercised. Instrumented
# objects go to their own directory.
ANTITHESIS ?= 0
ifeq ($(ANTITHESIS),1)
  BUILD = build-antithesis
  # -fno-sanitize-link-runtime keeps clang from auto-linking compiler-rt's
  # sanitizer runtime, whose coverage callbacks would duplicate the SDK's.
  ANT_CXXFLAGS = -fsanitize-coverage=trace-pc-guard -fno-sanitize-link-runtime -Iantithesis/sdk
  # Every instrumented *binary* needs exactly one TU defining the coverage
  # callbacks (the archive itself does not, which is why this is a link input).
  ANT_SUPPORT = antithesis/sdk/instrumentation.cpp
  # --build-id stamps the binary so Antithesis can match it to the unstripped
  # copy in /symbols.
  ANT_LDFLAGS = -Wl,--build-id
  # A distinct output name, so the two variants can coexist in one image and so
  # flipping ANTITHESIS actually relinks instead of finding the target "up to
  # date" from the other build.
  CLI = dbdb-instrumented
else
  BUILD = build
  ANT_CXXFLAGS =
  ANT_SUPPORT =
  ANT_LDFLAGS =
  CLI = dbdb
endif

CXXFLAGS = -std=c++20 -Wall -Wextra -g -O0 -Iinclude $(ANT_CXXFLAGS)

HDRS = $(wildcard include/dbdb/*.hpp)
LIB_SRCS = $(wildcard src/*.cpp)
LIB_OBJS = $(patsubst src/%.cpp,$(BUILD)/%.o,$(LIB_SRCS))
LIB = $(BUILD)/libdbdb.a

# Default goal: ./dbdb normally, ./dbdb-instrumented under ANTITHESIS=1.
$(CLI): cli/main.cpp $(LIB) $(HDRS)
	$(CXX) $(CXXFLAGS) $(ANT_LDFLAGS) -o $(CLI) cli/main.cpp $(ANT_SUPPORT) -L$(BUILD) -ldbdb

$(LIB): $(LIB_OBJS)
	ar rcs $@ $^

$(BUILD)/%.o: src/%.cpp $(HDRS)
	@mkdir -p $(BUILD)
	$(CXX) $(CXXFLAGS) -c $< -o $@

test: $(LIB) $(HDRS) $(wildcard tests/*.cpp tests/*.h)
	$(CXX) $(CXXFLAGS) -o run_tests $(wildcard tests/*.cpp) $(ANT_SUPPORT) -L$(BUILD) -ldbdb

run_tests: test
	./run_tests

# IntelliSense (clangd) reads compile_commands.json to learn each file's
# include paths. tools/gen_compile_commands.py derives it from these Makefiles
# via `make -n`, so it cannot drift from the real build. Re-run after adding a
# source file or changing CXXFLAGS.
compile_commands.json:
	python3 tools/gen_compile_commands.py .

clean:
	rm -rf build build-antithesis dbdb dbdb-instrumented run_tests

.PHONY: test clean compile_commands.json
