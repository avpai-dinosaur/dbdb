// Scratch files for the unit tests.
//
// Every test wants a database nobody else is touching, so each call hands back
// a path of its own: the process id keeps two runs apart, and the counter keeps
// two tests in the same run apart, which matters because cargo runs them on
// several threads at once.

use std::path::PathBuf;
use std::process;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

static NEXT: AtomicU64 = AtomicU64::new(0);

pub fn temp_path(name: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::SeqCst);
    let mut path = PathBuf::from(std::env::temp_dir());
    path.push(format!("dbdb-rust-{}-{}-{}.db", name, process::id(), n));

    // A leftover file from an earlier run would look like a database with
    // things already in it, which is not what any of these tests want.
    let _ = std::fs::remove_file(&path);

    path.to_string_lossy().to_string()
}
