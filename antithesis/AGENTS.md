This directory contains files relevant to running tests in Antithesis.

Use the `antithesis-setup` skill to scaffold and manage this directory. Use the `antithesis-research` skill to analyze the system and build a property catalog. Use the `antithesis-workload` skill to implement assertions and test commands. Use the `antithesis-launch` skill to build, validate, and submit Antithesis runs — do not run `snouty launch` directly.

**snouty launch**
Use `snouty launch --json --webhook basic_test --config antithesis/config` to start an Antithesis run. Always run `compose build` first to ensure images are up to date.

**snouty validate**
Use this command to quickly validate changes to the Antithesis scaffolding. See `snouty validate --help` for details.

**setup-complete.sh**
Inject this script into a Dockerfile to notify Antithesis that setup is complete. This script should only run once the system under test is ready for testing. Antithesis will not run any test commands until it receives this event.

**sdk**
This directory holds the vendored Antithesis C++ SDK headers (see `sdk/README.md`).
`antithesis_instrumentation.h` supplies the sanitizer-coverage callbacks;
`antithesis_sdk.h` supplies assertions for the workload. Both require clang 16+
and C++20, which is why the project builds with `clang++ -std=c++20`.

**lib**
Holds `libvoidstar.so`, the stub instrumentation library. The drivers image copies
it to `/usr/lib/libvoidstar.so`, where the SDK dlopens it; Antithesis substitutes
the real implementation at test time.

**instrumentation**
The test commands are Python and are not themselves instrumented. Coverage comes
entirely from the `dbdb` CLI they drive, so that binary must stay instrumented:
`make ANTITHESIS=1` builds `./dbdb-instrumented` with
`-fsanitize-coverage=trace-pc-guard`, linking the instrumented `libdbdb.a` from
`build-antithesis/` plus `sdk/instrumentation.cpp`. Plain `make` builds an
uninstrumented `./dbdb` for ordinary development, shipped as `dbdb-debug` for
debug shells. Verify with `nm <binary> | grep antithesis_load_libvoidstar`.
If the workload ever stops reporting coverage, check that the drivers are
spawning `dbdb` and not `dbdb-debug`.

**drivers**
Holds the image definition only (`Dockerfile`, `setup-complete.sh`). The test
commands themselves live in `test/`.

**config**
This directory contains the `docker-compose.yaml` file used to bring up this system within the Antithesis environment, along with any closely related config files. Snouty will push tagged images, consume this config directory, and launch the run.

**scratchbook**
This directory is the Antithesis scratchbook for the codebase. It contains documents such as system analysis, property catalogs, topology plans, per-property evidence files (in `scratchbook/properties/`), property relationship maps, and other persistent integration notes. Keep it up to date as Antithesis-related decisions change.

**test**
This directory contains test templates. `test/main/` maps to
`/opt/antithesis/test/v1/main/` in the image. The commands are Python and talk to
the database over the CLI's line protocol via `helper_dbdb.py` (see the REPL
section of the top-level README), so the workload is not tied to C++. A test template is a directory containing test command executable files. Each test command must have a valid prefix: `parallel_driver_, singleton_driver_, serial_driver_, first_, eventually_, finally_, anytime_`. Prefixes constrain when and how commands are composed in a single timeline. Files or subdirectories prefixed with `helper_` are ignored by Antithesis and can be used for helper scripts kept alongside the commands.
