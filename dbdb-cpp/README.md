# dbdb-cpp

The C++ implementation of dbdb: a key-value store on an append-only file, with a
CLI that speaks the line protocol in
[`../antithesis/CONTRACT.md`](../antithesis/CONTRACT.md). See
[the root README](../README.md) for the guarantees it aims at and the bugs found
in it so far.

## Build

```bash
make                 # -> ./dbdb
make run_tests       # builds and runs the unit tests
make ANTITHESIS=1    # -> ./dbdb-instrumented, with sanitizer coverage
```

The two variants have separate object directories (`build/` and
`build-antithesis/`) and separate output names, so neither disturbs the other.
Only `./dbdb-instrumented` reports coverage to Antithesis; see
`../antithesis/CONTRACT.md` for why that matters.

## Usage

One operation per invocation, each a complete transaction:

```bash
./dbdb DBNAME set KEY VALUE
./dbdb DBNAME get KEY
./dbdb DBNAME delete KEY
```

Or a session on stdin, where writes are held open until an explicit `commit`:

```bash
$ ./dbdb mydb.db repl
set accountA empty
OK
set accountB full
OK
commit
OK
get accountA
VALUE empty
exit
```

`../antithesis/CONTRACT.md` is the normative description of this protocol,
including the notes that matter when driving it from a program: reuse one
connection, read a reply for every command, and keep newlines out of keys and
values.

## IntelliSense

clangd reads `compile_commands.json`, which `tools/gen_compile_commands.py`
derives from this Makefile via `make -n` so it cannot drift from the real build:

```bash
make compile_commands.json
```

Re-run it after adding a source file or changing `CXXFLAGS`.
