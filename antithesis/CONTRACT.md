# What the harness requires of an implementation

The harness in this directory tests *a* dbdb, not *this* dbdb. It knows exactly
two things about the system under test: a command-line protocol, and where to
find the binary that speaks it. Everything else — language, storage engine, build
system, how coverage instrumentation is wired — belongs to the implementation.

So `antithesis/` is a sibling of `dbdb-cpp/`, `dbdb-rust/`, `dbdb-python/`, and
the dependency points one way: the harness knows an implementation exists, the
implementation does not know the harness exists.

Changing the storage format, rewriting the tree, swapping the locking strategy —
none of that should require touching this directory. Changing the protocol
below does.

---

## 1. The command-line protocol

An implementation ships one executable. It is invoked two ways.

### One-shot

```
dbdb DBNAME get KEY        # prints the value, or exits 2 if absent
dbdb DBNAME set KEY VALUE
dbdb DBNAME delete KEY
```

Each is a complete transaction: connect, apply, commit, exit. Exit 0 on success,
2 for a missing key on `get`, non-zero otherwise.

### REPL — what the harness actually uses

```
dbdb DBNAME          # or: dbdb DBNAME repl
```

Reads one command per line on stdin, writes **exactly one response line** per
command on stdout, flushed immediately. A caller writes a line and blocks on the
reply, so a buffered response is a deadlock.

| command | response |
|---|---|
| `get KEY` | `VALUE <value>` or `NOTFOUND` |
| `set KEY VALUE` | `OK` |
| `delete KEY` | `OK` |
| `commit` | `OK` |
| `exit` | none; exits 0 |
| malformed, or a database error | `ERR <message>` |

Required behavior:

- **`set` and `delete` do not commit.** The first one opens a transaction, held
  until an explicit `commit`. This is what makes multi-write atomicity testable;
  an implementation that auto-commits each write cannot be tested for it.
- **`exit` or EOF discards uncommitted writes** and releases any lock held.
- **`ERR` is not fatal.** The process stays up and the stream stays usable.
- **stdout carries nothing but responses.** Diagnostics go to stderr.
- A reply is produced for every command, in order.

Encoding: a key is one token containing no spaces; a value is the rest of the
line and may contain spaces. Neither may contain a newline. Nothing is escaped.

Optional but useful: tracing to stderr. The C++ implementation emits
`[dbdb]: <event> pid=<n> ...` lines, which is what the bug catalog in the root
README is reconstructed from. The harness does not parse them.

---

## 2. The image layout

An implementation publishes a container image containing:

| path | contents |
|---|---|
| `/out/bin/dbdb` | the executable above. **Required.** |
| `/out/bin/*` | anything else that should land on `PATH`. Optional. |
| `/out/symbols/` | unstripped binaries, so Antithesis can symbolize coverage. Must exist; may hold nothing if the implementation has no symbols to ship. |

**This image is never run.** The harness image is built `FROM` it and copies
those paths out, so the implementation image needs no entrypoint, no Python, no
test commands, and no Antithesis SDK beyond whatever its own instrumentation
requires. That is the whole reason the dependency can point in one direction.

## 3. Coverage instrumentation

The test commands are Python and are not instrumented. **All coverage feedback
comes from `/out/bin/dbdb`.** If that binary is not instrumented, Antithesis gets
no signal about which code paths a timeline reached and the fuzzer searches
blind — while the run still looks healthy. This is the easiest thing to get
wrong in a new implementation.

How to instrument is language-specific (the C++ build uses
`-fsanitize-coverage=trace-pc-guard` plus the vendored SDK's callbacks). What is
common to all of them: the binary must dlopen `libvoidstar.so`, which the harness
image places at `/usr/lib/libvoidstar.so`. For compiled languages, verify with:

```
nm /out/bin/dbdb | grep antithesis_load_libvoidstar
```

An interpreted implementation has no equivalent, and will report no coverage.

---

## Adding an implementation

1. `mkdir dbdb-<lang>/` at the repo root, with a self-contained Docker build
   context — nothing outside that directory.
2. Write `dbdb-<lang>/Dockerfile` whose final stage contains `/out/bin/dbdb` and
   `/out/symbols/`.
3. Build it, then point the harness at it:

```
make -C antithesis IMPL=dbdb-<lang>
```

No file in `antithesis/` needs to change.
