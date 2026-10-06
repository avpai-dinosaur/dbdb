#!/usr/bin/env python3
"""Generate compile_commands.json for clangd from the Makefiles themselves.

Runs `make -Bn` (dry run) over the plain and instrumented builds and turns the
printed compile lines into a compile database, so the flags IntelliSense uses
are by construction the flags the build uses. Nothing is compiled.

Also records the compiler's real system include paths, which clangd cannot work
out for itself under a nix cc-wrapper (see system_includes). That makes the
database self-contained: no .vscode settings are needed, and it works however the
editor was launched.

Re-run after adding a source file or changing CXXFLAGS:

    make compile_commands.json
"""
import json, os, re, shlex, shutil, subprocess, sys

ROOT = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else ".")
SRC_EXT = (".cpp", ".cc", ".cxx", ".c")


def dry_run(cwd, targets):
    r = subprocess.run(["make", "-Bnw"] + targets, cwd=cwd,
                       stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    return r.stdout.splitlines()


_SYSTEM_INCLUDES = {}


def system_includes(cxx):
    """Ask the driver where its system headers actually live.

    Needed because of how the nix cc-wrapper works: it is a shell script that
    bakes in -nostdlibinc and then re-supplies libc++ via -cxx-isystem and glibc
    via -idirafter, with the dev shell adding more through NIX_CFLAGS_COMPILE.
    clangd parses the recorded command rather than executing the wrapper, so none
    of that reaches it and every standard header comes back "file not found".

    Recording the real search list in the database keeps IntelliSense working
    with no editor configuration at all, which matters here: .vscode is
    gitignored, so settings kept there do not survive a `git clean`. Same
    principle as the compile flags themselves -- read them off the build rather
    than maintain a second copy by hand.
    """
    if cxx in _SYSTEM_INCLUDES:
        return _SYSTEM_INCLUDES[cxx]
    flags = []
    try:
        r = subprocess.run([cxx, "-E", "-v", "-x", "c++", os.devnull],
                           stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
    except OSError:
        r = None
    if r is not None and r.returncode == 0:
        keep, dirs = False, []
        for line in r.stderr.splitlines():
            if line.startswith("#include <...> search starts here:"):
                keep = True
            elif line.startswith("End of search list."):
                break
            elif keep:
                d = line.strip()
                if d and os.path.isdir(d):
                    dirs.append(d)
        if dirs:
            # Drop the defaults and re-add exactly what the driver reports, in
            # its order. Correct for a plain toolchain too: there the query just
            # returns the standard directories we removed.
            flags = ["-nostdlibinc"] + [f for d in dirs for f in ("-isystem", d)]
    if not flags:
        print(f"warning: could not read system include paths from {cxx}; "
              "clangd may not find standard headers", file=sys.stderr)
    _SYSTEM_INCLUDES[cxx] = flags
    return flags


ENTER = re.compile(r"^make(?:\[\d+\])?: Entering directory ['\"](.*)['\"]$")
LEAVE = re.compile(r"^make(?:\[\d+\])?: Leaving directory ['\"](.*)['\"]$")


def entries(lines, cwd):
    """Walk the dry-run output, following recursive sub-makes so each compile
    line is attributed to the directory make was actually running in."""
    out = []
    dirs = [cwd]
    for line in lines:
        line = line.strip()
        m = ENTER.match(line)
        if m:
            dirs.append(m.group(1))
            continue
        if LEAVE.match(line):
            if len(dirs) > 1:
                dirs.pop()
            continue
        cwd = dirs[-1]
        if not line or "clang++" not in line and "g++" not in line:
            continue
        try:
            argv = shlex.split(line)
        except ValueError:
            continue
        if not argv or os.path.basename(argv[0]) not in ("clang++", "g++", "c++"):
            continue
        srcs = [a for a in argv[1:] if a.endswith(SRC_EXT)]
        if not srcs:
            continue
        # Strip link-only noise and the output flag; keep one entry per source.
        base, skip = [], False
        for a in argv[1:]:
            if skip:
                skip = False
                continue
            if a == "-o":
                skip = True
                continue
            if a == "-c":
                continue
            if a in srcs or a.startswith("-L") or a.startswith("-l") or a.startswith("-Wl,"):
                continue
            base.append(a)
        # Record the compiler by absolute path. Under a nix dev shell the
        # driver is a wrapper that supplies the libc++/glibc include paths;
        # clangd must be able to find it even when the editor was launched
        # with a PATH that lacks the dev shell.
        cxx = shutil.which(argv[0]) or argv[0]
        for s in srcs:
            out.append({
                "directory": cwd,
                "file": os.path.normpath(os.path.join(cwd, s)),
                "arguments": [cxx] + system_includes(cxx) + base + ["-c", s],
            })
    return out


db, seen = [], set()
passes = [
    (ROOT, ["dbdb", "test"]),
    # The instrumented build, so clangd understands antithesis-sdk/instrumentation.cpp
    # and the -Iantithesis-sdk include path. ANTITHESIS=1 is a command-line
    # variable assignment, which make accepts in the same position as a target.
    (ROOT, ["ANTITHESIS=1", "dbdb-instrumented"]),
]
for cwd, targets in passes:
    if not os.path.isdir(cwd):
        continue
    for e in entries(dry_run(cwd, targets), cwd):
        if e["file"] in seen:
            continue
        seen.add(e["file"])
        db.append(e)

db.sort(key=lambda e: e["file"])
path = os.path.join(ROOT, "compile_commands.json")
with open(path, "w") as f:
    json.dump(db, f, indent=2)
    f.write("\n")
print(f"wrote {len(db)} entries to {path}")
for e in db:
    print("  ", os.path.relpath(e["file"], ROOT))

# A compile database is only useful for the files it covers, and make goes
# quiet rather than erroring when it has nothing to do (an up-to-date target,
# or a pattern rule whose prerequisites are missing). Compare against the
# sources actually on disk so a silently partial db is visible here.
PRUNE = {".git", "build", "build-antithesis", "bin", "node_modules"}
on_disk = set()
for dirpath, dirnames, filenames in os.walk(ROOT):
    dirnames[:] = [d for d in dirnames if d not in PRUNE]
    for fn in filenames:
        if fn.endswith(SRC_EXT):
            on_disk.add(os.path.join(dirpath, fn))

missing = sorted(on_disk - seen)
if missing:
    print("\nwarning: no compile command found for these sources -- clangd will"
          "\nfall back to guessed flags and may report bogus errors in them:",
          file=sys.stderr)
    for m in missing:
        print("  ", os.path.relpath(m, ROOT), file=sys.stderr)
    print("Build once (`make dbdb test && make ANTITHESIS=1`), "
          "then re-run.", file=sys.stderr)
    sys.exit(1)
