// Compiles the vendored Antithesis instrumentation shim and links it into the
// binary.
//
// Building with the sanitizer-coverage flags in the Makefile makes the compiler
// put a call to __sanitizer_cov_trace_pc_guard at every branch. Nothing in Rust
// defines those callbacks, so without this the link fails outright -- which is
// the good case, because the alternative would be a binary that builds fine and
// quietly reports no coverage at all.
//
// The shim is compiled here rather than written in Rust because it must not be
// instrumented itself: a callback that called itself on every branch would
// recurse forever. Cargo compiles build scripts for the host and leaves
// RUSTFLAGS out of them when --target is passed, which is why the Makefile
// always passes it.

use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    let out_dir = env::var("OUT_DIR").expect("cargo always sets OUT_DIR");
    let object = Path::new(&out_dir).join("instrumentation.o");
    let archive = Path::new(&out_dir).join("libdbdb_instrumentation.a");

    // cc is clang or gcc depending on the machine; either one can compile the
    // shim, which is plain C.
    let cc = env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let status = Command::new(&cc)
        .arg("-c")
        .arg("-O2")
        .arg("-fPIC")
        .arg("antithesis-sdk/instrumentation.c")
        .arg("-o")
        .arg(&object)
        .status()
        .unwrap_or_else(|e| panic!("could not run {}: {}", cc, e));
    if !status.success() {
        panic!("{} failed to compile the instrumentation shim", cc);
    }

    let ar = env::var("AR").unwrap_or_else(|_| "ar".to_string());
    // r replaces, c creates quietly, s writes the index. The archive is rebuilt
    // from scratch each time, so there is nothing stale to replace.
    let _ = std::fs::remove_file(&archive);
    let status = Command::new(&ar)
        .arg("rcs")
        .arg(&archive)
        .arg(&object)
        .status()
        .unwrap_or_else(|e| panic!("could not run {}: {}", ar, e));
    if !status.success() {
        panic!("{} failed to archive the instrumentation shim", ar);
    }

    println!("cargo:rustc-link-search=native={}", out_dir);
    println!("cargo:rustc-link-lib=static=dbdb_instrumentation");
    println!("cargo:rerun-if-changed=antithesis-sdk/instrumentation.c");
    println!("cargo:rerun-if-changed=antithesis-sdk/antithesis_instrumentation.h");
    println!("cargo:rerun-if-env-changed=CC");
    println!("cargo:rerun-if-env-changed=AR");
}
