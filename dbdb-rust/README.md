# dbdb-rust

The Rust implementation of dbdb: a key-value store on an append-only file, with
a CLI that speaks the line protocol in
[`../antithesis/CONTRACT.md`](../antithesis/CONTRACT.md). See
[the root README](../README.md) for the guarantees it aims at and the bugs found
so far.

It is a port of the same design as [`../dbdb-cpp/`](../dbdb-cpp/) — immutable
nodes, a root pointer that only moves on commit, writers serialized by a file
lock — so the two should behave the same way, including where they are wrong.

## Build

```bash
make                 # -> ./dbdb
make test            # runs the unit and protocol tests
make ANTITHESIS=1    # -> ./dbdb-instrumented, with sanitizer coverage
make verify          # checks that ./dbdb-instrumented really is instrumented
```

The two variants have separate target directories (`target/` and
`target-antithesis/`) and separate output names, so neither disturbs the other.
Only `./dbdb-instrumented` reports coverage to Antithesis; see
[`../antithesis/CONTRACT.md`](../antithesis/CONTRACT.md) for why that matters.

The crate has **no dependencies**. Everything it needs — files, `flock`, a
process id — is in `std`, so the build never reaches the network and `cargo`
runs `--offline --locked`.

## Usage

One operation per invocation, each a complete transaction:

```bash
./dbdb DBNAME set KEY VALUE
./dbdb DBNAME get KEY         # exits 2 if the key is not there
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

## How it fits together

| file | what it does |
|---|---|
| `src/main.rs` | argument handling and the session loop — the only module that knows the protocol exists |
| `src/db.rs` | when a transaction starts, what a read sees, what commit means |
| `src/tree.rs` | the binary search tree, whose nodes are never modified once written |
| `src/storage.rs` | the append-only file, the root pointer, and the lock |
| `src/bytes.rs` | the little-endian encoding everything on disk is made of |
| `src/trace.rs` | `[dbdb]: …` lines on stderr, for reading a timeline afterwards |

A read outside a transaction looks at whatever has been committed by the time it
runs. The first write of a session takes the file lock and keeps it until
`commit`, which writes the new nodes, moves the root pointer, and releases the
lock. A session that goes away mid-transaction — `exit`, end of input, or being
killed — loses its writes, because they were never anywhere but in its own
memory, and drops its lock when its file descriptor closes.

## Instrumentation

Rust has no `-fsanitize-coverage` of its own, so `make ANTITHESIS=1` passes the
flags to LLVM directly and `build.rs` compiles the vendored shim in
[`antithesis-sdk/`](antithesis-sdk/), which is what defines the coverage
callbacks and forwards them to `libvoidstar`. The shim is C rather than Rust
because it must not be instrumented itself — a callback that called itself on
every branch would recurse forever.

That is also why the Makefile always passes `--target` even for a native build:
without it, cargo hands `RUSTFLAGS` to `build.rs` as well, and the build script
would be compiled with the coverage flags and fail to link against the very shim
it is supposed to be producing.

## Tests

```bash
make test
```

`src/*.rs` carry unit tests for the encoding, the tree, and the transaction
rules. `tests/protocol.rs` is the one that matters for the harness: it spawns the
built binary and drives it over a pipe exactly as `helper_dbdb.py` does, checking
that every command gets one reply, that replies are flushed rather than buffered,
that `set` and `delete` do not commit, and that `delete` on a missing key still
answers `OK`.
