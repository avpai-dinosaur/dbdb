# dbdb

A key-value store built on an append-only file, after
[DBDB: Dog Bed Database](https://aosabook.org/en/500L/dbdb-dog-bed-database.html),
together with an Antithesis test harness for it.

## Layout

| directory | what it is |
|---|---|
| [`dbdb-cpp/`](dbdb-cpp/) | the C++ implementation — build and usage instructions live there |
| [`dbdb-rust/`](dbdb-rust/) | the Rust implementation, a port of the same design |
| [`dbdb-python/`](dbdb-python/) | the original [500 Lines](https://aosabook.org/en/500L/dbdb-dog-bed-database.html) Python implementation, wrapped to satisfy the same contract |
| [`antithesis/`](antithesis/) | the Antithesis test harness, which names no implementation language |

The harness tests *a* dbdb, not *this* dbdb. It drives the CLI's line protocol
and knows nothing else about what is underneath, so the storage format, the tree,
and the locking strategy can all change without touching it.

[`antithesis/CONTRACT.md`](antithesis/CONTRACT.md) is the entire interface
between the two: a command-line protocol, and two paths in a container image. A
second implementation is a new `dbdb-<lang>/` directory that satisfies it — no
file under `antithesis/` changes.

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

---

## Bug Catalog

### Read Isolation Violated

*Date:* `10/05/2026`

*Hash:* `889d275a215d6c8411d9f0f65cc15544487656cb`

Process 54 reads `accountA` at root 271 and `accountB` at root 375 while process 51 commits a transfer in between. Process 54 sees both accounts `full` and fails `ALWAYS(count == 1)` on a state that never existed on disk.

- **Run:** `ccb1fd8faeec545d6830be3e5c2ca4a8-63-5`
- **Logs:** [search at vtime 13.381](https://honey-whale.antithesis.com/search?search=v6veyJxIjp7Im4iOnsiciI6eyJoIjpbeyJoIjpbeyJjIjoibW9tZW50LnZ0aW1lIiwiZiI6ImdlbmVyYWwuY3VzdG9tIiwibyI6Im1hdGNoZXMiLCJ2IjoiMTMuMzgxMzA0OTQ4ODIzNTI3In1dLCJvIjoib3IifSx7ImgiOlt7ImMiOiJtb21lbnQuaW5wdXRfaGFzaCIsImYiOiJnZW5lcmFsLmN1c3RvbSIsIm8iOiJtYXRjaGVzIiwidiI6IjkwNjg3NDE0ODU0MzI2NjkzMzEifV0sIm8iOiJvciJ9XSwibyI6ImFuZCJ9LCJ0Ijp7ImciOmZhbHNlLCJtIjoiIn0sInkiOiJub25lIn19LCJzIjoiOGE2NmJmNmU5MmE0YjY0ODM2ZGE0N2NlMTg4ZmM4NTUtNjMtNSJ9)
- **Moment:** `Moment.from({ session_id: "8a66bf6e92a4b64836da47ce188fc855-63-5", input_hash: "9068741485432669331", vtime: 13.381304948823527 })`

| vtime | pid | event |
|---|---|---|
| 13.269 | 54 | `view_refresh reason="get_unlocked" old_root=271 new_root=271` |
| 13.275 | 51 | `commit_begin seq=1` |
| 13.295 | 51 | `node_commit addr=323 key="accountB" value="full" left=0 right=0` |
| 13.304 | 51 | `node_commit addr=375 key="accountA" value="empty" left=0 right=323` |
| 13.309 | 51 | `root_commit old_root=271 new_root=375` |
| 13.310 | 51 | `commit_end seq=1` |
| 13.330 | 54 | `view_refresh reason="get_unlocked" old_root=271 new_root=375` |
