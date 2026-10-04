# dbdb

A C++ implementation of [DBDB: Dog Bed Database](https://aosabook.org/en/500L/dbdb-dog-bed-database.html).

The database is a simple key-value store built on an append-only file.

## Build

```bash
make             # builds ./dbdb
make run_tests   # builds and runs the unit tests
```

## Usage

```bash
./dbdb DBNAME set KEY VALUE
./dbdb DBNAME get KEY
./dbdb DBNAME delete KEY
```

## Guarantees

`dbdb` intends to provide full ACID guarantees through its implementation on top of an immutable data structure.

- **Atomicity:** a commit either fully happens or doesn't. We never modify data in place, only append new data to the end of the file. Concretely, a commit just means updating the address pointing to the "root" of the kv-store. Thus, until a commit occurs, new data is unreachable, old data is preserved, so a crash leaves the old kv-store intact.
- **Consistency:** any committed root points to a complete, valid kv-store. 
- **Isolation:** transactions don't interfere with each other. Because of our immutable structure, readers don't need any locking; they always see the kv-store corresponding to the last root address they loaded, which a concurrent commit can't mutate. Writers serialize themselves through a `flock`.
- **Durability:** once a commit returns, the data is on disk.

### Caveats

- **The file only grows.** Old nodes are never removed. Could be solved with a compaction mechanism.
- **Stale reads.** Any past root address still points to a
  complete tree, so you could read the database as of an earlier commit.
