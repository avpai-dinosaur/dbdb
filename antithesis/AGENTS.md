This directory contains files relevant to running tests in Antithesis.

Use the `antithesis-setup` skill to scaffold and manage this directory. Use the `antithesis-research` skill to analyze the system and build a property catalog. Use the `antithesis-workload` skill to implement assertions and test commands. Use the `antithesis-launch` skill to build, validate, and submit Antithesis runs — do not run `snouty launch` directly.

**snouty launch**
Use `snouty launch --json --webhook basic_test --config antithesis/config` to start an Antithesis run. Always run `make -C antithesis` first to ensure images are up to date.

**snouty validate**
Use this command to quickly validate changes to the Antithesis scaffolding. See `snouty validate --help` for details.

**setup-complete.sh**
Inject this script into a Dockerfile to notify Antithesis that setup is complete. This script should only run once the system under test is ready for testing. Antithesis will not run any test commands until it receives this event.

**CONTRACT.md**
The interface between this directory and the database it tests: a command-line
protocol, and two paths in a container image. Read it before changing anything
here. Nothing in this directory may assume an implementation language — the
per-language SDKs and build systems live in the `dbdb-<lang>/` directories.

**Dockerfile / Makefile**
`make -C antithesis` builds the implementation image (default `IMPL=dbdb-cpp`),
then builds the harness image `FROM` it, copying out `/out/bin/` and
`/out/symbols/`. Test another implementation with `make -C antithesis IMPL=dbdb-rust`.
The dependency points one way: the harness knows implementations exist, they do
not know it exists.

**lib**
Holds `libvoidstar.so`, the stub instrumentation library. The harness image copies
it to `/usr/lib/libvoidstar.so`, where an instrumented binary dlopens it;
Antithesis substitutes the real implementation at test time. It lives here rather
than in an implementation directory because every language needs the same copy.

**instrumentation**
The test commands are Python and are not instrumented. Coverage comes entirely
from the `dbdb` binary they drive, which the implementation is responsible for
instrumenting — see `CONTRACT.md`. If a run reports no coverage, that is the
first thing to check; the run will otherwise look healthy while the fuzzer
searches blind.

**config**
This directory contains the `docker-compose.yaml` file used to bring up this system within the Antithesis environment, along with any closely related config files. Snouty will push tagged images, consume this config directory, and launch the run.

**scratchbook**
This directory is the Antithesis scratchbook for the codebase. It contains documents such as system analysis, property catalogs, topology plans, per-property evidence files (in `scratchbook/properties/`), property relationship maps, and other persistent integration notes. Keep it up to date as Antithesis-related decisions change.

**test**
This directory contains test templates. `test/main/` maps to
`/opt/antithesis/test/v1/main/` in the image. The commands are Python and talk to
the database over the CLI's line protocol via `helper_dbdb.py` (specified in
`CONTRACT.md`), so the workload is not tied to any implementation language. A test template is a directory containing test command executable files. Each test command must have a valid prefix: `parallel_driver_, singleton_driver_, serial_driver_, first_, eventually_, finally_, anytime_`. Prefixes constrain when and how commands are composed in a single timeline. Files or subdirectories prefixed with `helper_` are ignored by Antithesis and can be used for helper scripts kept alongside the commands.
