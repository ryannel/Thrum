# Contributing to Thrum

Use stable Rust and Python 3.11+. `./dev` selects the stable rustup toolchain even if a standalone
Cargo comes first on PATH. PowerShell users can use `./dev.ps1` with the same commands. The workspace lockfile is committed. Run `rustup run stable cargo fetch --locked` once on a clean copy; checks then use locked offline Cargo commands.

The Bash `./dev` launcher defaults to an empty `RUSTC_WRAPPER`, overriding machine-wide Cargo wrapper configuration.
Set `RUSTC_WRAPPER` explicitly when opting into a compiler wrapper for a particular invocation.
Both launchers set `CARGO_INCREMENTAL=0` for `check`, including the xtask build and its child Cargo
commands. The pinned compiler has failed verification with unstable incremental trait-obligation
fingerprints after revision changes; the check therefore does not reuse that query cache.

| Command | What it does |
| --- | --- |
| `./dev help` | List implemented contributor commands |
| `./dev check` | The merge tier, which runs before every merge: repository checks and their tests, rustfmt, Clippy and V1 game Wasm Clippy for the whole workspace, then the Rust tests (including Mac Metal) of the packages the change since the merge base with `origin/main` can reach, side by side; it prints what it selected and why, runs every test where it cannot be sure, and ends with where its time went. [Tests and the check](#tests-and-the-check) says more |
| `./dev check-all` | The same with every Rust test of the workspace, whatever changed; the main-branch check `workspace-tests` runs it on main after merges, at most once an hour |
| `./dev long-tests` | The main tier's long tests and the tests behind a Cargo feature, from `.config/long-tests.txt`, with a GPU required; the main-branch check `long-tests` runs it on main after merges, at most every four hours |
| `./dev selection` | What `./dev check` would select for this checkout's change, and why |
| `./dev check-docs` | Local Markdown links/anchors, skill metadata, maturity hashes and dependency policy |
| `./dev check-web` | Lint/compile the V1 game, starter and kit test targets for `wasm32-unknown-unknown` |
| `./dev check-mobile-core` | Compile the portable core for ARM64 iOS device, iOS simulator and Android, plus the starter facade for iOS simulator |
| `./dev fmt` | Apply rustfmt |
| `./dev starter` | Cook and watch the [starter](examples/starter/README.md)'s assets and open its window (macOS) |
| `./dev scene <folder>` | The same for any scene package, such as your copy of the starter (macOS) |
| `./dev refresh-design-hashes` | Explicitly refresh current design digests after reviewing a design edit |

To rerun the offline finite-disk shadow diagnostic from the repository root, use
`python3 scripts/atmosphere_finite_disk.py`. It reports projected-chord and direct-ray coverage
on reconstructed terrain and caster cases, plus the work spent under each case's cap.

Before `./dev check`, run
`rustup target add --toolchain stable wasm32-unknown-unknown aarch64-apple-ios-sim`
to install the rule guard's cross-compilation targets, and `cargo install --locked cargo-nextest`:
`./dev check` runs its tests side by side through cargo-nextest 0.9.150 or newer, and stops before
any step with that command when it is missing or older.
For actual Chrome function: build with `sh examples/v1-game/web/build.sh`, then run
`node scripts/check_v1_browser.mjs --case all --output tmp/browser-check` (new output directory, Node 24, this M3 Max).
The material hook reload is compiled only into the development page: build it with `build.sh --dev`, then run
`--case material --web-root target/v1-game-web-dev`.
The compile alone proves no browser runtime behavior. The GitHub Actions
workflow is manual-only by owner decision; it describes Linux, macOS and Windows checks, plus Wasm
compilation. Run `./dev check` locally before committing. A configured job is not evidence it has run.
On macOS with Xcode, `./dev check-mobile-core` needs the pinned Rust targets
`aarch64-apple-ios`, `aarch64-apple-ios-sim` and `aarch64-linux-android` installed with `rustup target add --toolchain stable`.
It compiles only `thrum-core`; app packages, native activity/UI, devices and lifecycle remain separate checks.
`THRUM_PYTHON` can select the Python executable. Build products use `target/`, or an explicitly
owned `CARGO_TARGET_DIR`. `cargo clean` affects that build directory; inspect ownership before cleanup.

## Change the smallest complete interaction

Start from [the vision and scope document](docs/plans/VISION.md), your module in [the module list](docs/plans/ARCHITECTURE.md), the owning specification and affected contracts.
Describe the behavior, its state owner and observable result before adding APIs. Keep simulation rules
independent of rendering, transport and platform I/O. Local and remote inputs use the same validation
and authority; transport does not become a second gameplay implementation.

Use idiomatic Rust: modules express ownership, types distinguish identities/units/lifetimes where useful,
`Result` carries expected failure, and public APIs document invariants and error behavior. Avoid hidden
global state, unchecked assumptions at external boundaries and blanket lint suppression. Workspace lints
forbid unsafe code by default; a future necessary adapter must justify a narrow exception and its safety
invariants in review. The [Thrum engineering rules](.agents/skills/thrum-engineering/references/building.md) record how
to choose work, match rigour to state and shape code.

Add a crate only for a demonstrated boundary, reuse or platform/build need. Register it in
[architecture policy](.config/architecture.toml); all workspace crates inherit common package/lint settings.
The dependency guard checks direct workspace edges of every dependency kind, including renamed and
target-specific edges. It does not prove semantic authority or validate all optional feature combinations.
Every `Cargo.toml` under `crates/`, `tools/` or `examples/` must join the workspace and architecture registry. Direct
third-party dependencies require an exact `=version` registry requirement and a policy entry with declared
license, purpose and target/build cost; the guard cannot verify the upstream license or transitive closure.
Record dependency purpose, exact version/license and target costs before adding external libraries. Do
not vendor third-party engine code without source revision, applicable license/notices and a maintenance rationale.

`./dev check` must pass before a merge and runs pinned rustfmt, workspace Clippy, Rust tests and repository checks.
Clippy runs `all` and `pedantic` plus selected restriction lints (`Cargo.toml`, thresholds in `clippy.toml`)
with every warning denied. Suppress only with `#[expect(lint, reason = "...")]` on the narrowest item;
`#[allow]` is rejected and an unfulfilled expectation fails. Production code does not `unwrap` or print,
keeps `pub` for exported items only, and documents every unsafe block of an approved exception.

The repository check also keeps `#[cfg(test)]` to test modules in production files and requires a reason
on every `#[ignore]`. The size limits are generous backstops: a function fails above 200 lines, a
production file notes above 1,000 code lines and fails above 2,500, and a test file notes above 2,000 and
fails above 4,000 (blank, comment and inline-test lines excluded). A note asks whether the file still holds
one responsibility; split by responsibility, never to satisfy a count. Folder file counts and argument
counts have no limit.

For capability design and performance-sensitive implementation, use the owning design's
[cost account](docs/design/AI-PERFORMANCE.md#the-cost-account-in-each-owning-design). Consider repeated cheap work,
scaling, data movement and waiting before choosing representations. AI-manageable specialization, generated code
and target-specific paths can justify substantial internal complexity; document invariants, dispatch/validity and
regeneration, with relevant correctness and integration checks. Preserve the small public model. No new unsafe-code
allowance follows from this policy. Build verification alongside the selected implementation and report the scope of
any performance claim. Routine fixes need only their affected cost/behavior checks.

Test behavior and plausible failure, not implementation shape. For authority work cover rejected/stale
input and complete committed outcomes; for lifecycle work cover interruption and recovery. Keep small
fixtures deterministic within a stated envelope. No timing/performance claim follows from unit tests.
Run the affected target/feature combinations when added; whole-engine optimization needs the named
integrated workload and unchanged quality/correctness constraints in [AI performance](docs/design/AI-PERFORMANCE.md).

## Tests and the check

[The test tiers](ORCHESTRATE.md#test-tiers) say which tests run when, and the risk each tier accepts. This section
is how to work with them.

| Command | When it runs | What it runs | How long it takes |
| --- | --- | --- | --- |
| `./dev check` | Before every merge via `THRUM_MERGE_VERIFY=1 ./dev check`, and by hand | The whole workspace's guards, script suites, build and lints, and the fast tests of the packages the change can reach | Planned at about five minutes for a typical landing. The 30 merges before `14a4a7c83` (22 November 2026) had a median of 6.4 minutes; the four that ran tests from `b69c9812f` took 3.3 to 4.4 |
| `./dev check-all` | On main after merges, at most once an hour (`workspace-tests`) | The same, with every fast test of the workspace | Six to eight minutes |
| `./dev long-tests` | On main after merges, at most every four hours (`long-tests`) | The long tests in release with overflow checks on, and the tests behind a Cargo feature with their features on, a GPU required | About 17 to 20 minutes, one binary at a time |
| `bash scripts/check_cross_target.sh` | On main after merges, at most once an hour (`cross-target`) | A type-check, with nothing linked, of every target of every workspace package for Linux and Windows | Under a minute when the build is warm |

**What `./dev check` runs.** In order: the selection, the 2D profile guard, the exact-baseline reference guard,
the repository guards, the script suites, rustfmt, Clippy for the workspace, Clippy for the V1 game's Wasm
build, and Clippy for the tests behind a Cargo feature, which are built and linted at every merge but run only
on main. Then the selected Rust tests side by side through cargo-nextest, the doc tests, and the game's four
draw-probe tests. Everything but the tests covers the whole workspace whatever changed, and an unchanged
crate's build is reused.

**Merge Cargo cache on macOS.** Orchestrate's configured command can check an equivalent committed candidate in
one detached worktree under `$(git rev-parse --git-common-dir)/orchestrate/merge-cargo/checkout`. Cargo keeps its
normal `target/` there. The cache holds an exclusive lease through the check and cleanup, with one candidate
at a time. Every check reruns its guards and selected tests. Cargo decides which builds are fresh.

Admission reserves space for the build above a 30 GB free-space floor. Maintenance discards the cache's own
Cargo artifacts after an interrupted run, when retention exceeds 8 GiB, or when free space is under pressure.
The command logs the candidate, merge base, disk accounting and the reason for a reset or fallback. A busy
cache, insufficient space, local changes or different checkout inputs send verification to the caller's
worktree. Set `THRUM_MERGE_CACHE=0` in the orchestrate runner's environment to bypass reuse. Plain
`./dev check` and scheduled main-branch checks use their existing build directories.

**How it selects.** The change is every path that differs from the merge base with `origin/main`, uncommitted
and untracked paths included. The check runs the tests of each workspace package that holds a changed path and
of every package that depends on one, through Cargo's graph. A test that reads another package's files when it
runs is a dependency the graph can't show, so it needs a declared edge in `tools/xtask/src/selection.rs`, added
in the landing that adds the reader. A shared input selects the whole workspace: the root `Cargo.toml`,
`Cargo.lock`, `rust-toolchain.toml`, `clippy.toml`, `.cargo/`, `dev`, `dev.ps1`, `tools/xtask`,
`.config/long-tests.txt` and `.config/nextest.toml`. Markdown, `docs/` outside `docs/evidence`, `.agents/`,
`.github/`, `scripts/` and the rest of `.config/` select no tests. Any other path, a checkout with no merge base
and a checkout that doesn't differ from main also select the whole workspace. The check prints what it selected
and why, and `./dev selection` prints that without running anything.

**Tests run side by side.** Each test runs in a process of its own, as many at once as the machine has cores,
across every test binary. So a test can't rely on another test having run first, or on a static it shares with
another test, and a mutex in a test file no longer makes tests take turns. A test that needs a device to itself
joins a test group in `.config/nextest.toml`, and one that starts a thread pool is given `threads-required`
there, in the landing that adds it. A failed test is never run again, so a test that fails one time in ten
fails the check.

**A test takes about ten seconds or less, or is a long test.** Ten seconds is measured with the test run alone,
in the build the check uses. The check stops a test still running at ten seconds, runs each stopped test once
more alone after the others, and fails unless it passes within ten seconds then. In `./dev check-all`, the
initial limit is also ten seconds and the serial rerun has thirty seconds. The report calls out serial
passes above the ten-second fast-test target. A test that can't be made fast
is a long test: it is marked `#[ignore = "long: …"]`, its binary is listed in `.config/long-tests.txt` with the
fault it leaves unguarded before a merge written beside it, and only `./dev long-tests` runs it. A new long test
needs a plan's done-when item that asks for it, and no shortened copy of it stays in the merge tier.

**A test proves its behaviour on the smallest world that shows it, and tests logic without a device.** The world
is the grid, valley, cooked store, count of entities, ticks or frames the test builds to run on. Use the
smallest one on which each assertion could still fail. What a CPU can decide, such as a selection, a layout, a
state machine, a record's bytes or a refusal, is tested without a GPU, and a GPU test covers only what a device
alone can show, at the smallest target and for the fewest frames that show it. [The test
tiers](ORCHESTRATE.md#test-tiers) give the full rules, and [the engineering
patterns](.agents/skills/thrum-engineering/references/patterns.md#tests) show how.

**A package's test files are modules of one test binary.** A package's integration tests build into one test
target, `it`, whose root is `tests/it.rs`, so the check links and starts one binary for the package rather than one
for each file. To add a test file, write `tests/<file>.rs` and add `mod <file>;` to `tests/it.rs`; its tests are
then named `<file>::<test>`. A file under `tests/` that no target compiles fails the repository guards, and a new
test target that no plan names fails the check, so in a package with no `it` yet, such as `thrum-render`, add the
test to a file that exists. The `[[test]]` entries left in a package's `Cargo.toml` are files that something names
by binary or that have to run apart, each with its reason beside it.

**After the merge, on main.** The script runs three main-branch checks on main's tip, as the `Main-branch check`
lines of [ORCHESTRATE.md](ORCHESTRATE.md#rules) say: `workspace-tests` runs `./dev check-all`, `long-tests` runs
`./dev long-tests`, and `cross-target` runs `bash scripts/check_cross_target.sh`, which type-checks the workspace for
Linux and Windows from the Mac (it needs `rustup target add x86_64-unknown-linux-gnu x86_64-pc-windows-msvc`). A
failure there is run again once, then traced to the first commit that fails, and that commit's module fixes it. A
merge doesn't wait for any of them.

**Running tests by hand.** From the repository root, with the toolchain `./dev` uses:

- one file's tests: `cargo nextest run --locked --offline -p thrum-core --test it -E 'test(/^clock::/)'`;
- one test: `cargo nextest run --locked --offline -p thrum-core --test it -E 'test(=clock::display_callbacks_do_not_set_simulation_rate)'`;
- one long test, the way `./dev long-tests` runs it:
  `CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS=true THRUM_REQUIRE_GPU=1 cargo test --locked --offline --release -p v1-game --test exit_scene -- --ignored --exact movers_keep_their_authored_loops_after_a_stepped_hour`.
  Leave out `--release` for a binary the list builds `debug`;
- the whole main tier: `./dev long-tests`.

`cargo test -p <package> --test it <file>::` also runs one file's tests, but in one process on several threads,
not side by side as the check runs them.

**Reading the times the check prints.** It ends, pass or fail, with:

- `./dev check step times (wall)`: each step's wall time and the total. The tests step includes building and
  listing the test binaries as well as running them, so it is the longest, and it is the step to read first;
- the ten slowest tests, each timed side by side, where a test can take up to about twice its time alone;
- how many tests ran over ten seconds, and each test stopped at ten seconds with its time side by side and alone;
- the tests' summed time, and the share of it in tests whose binary or name says `gpu` or `metal`.

The per-attempt limits are the only times that can fail a check. A check's length follows the load other work puts
on the machine, so compare a step with the same step in other checks rather than reading one total on its own.

## Review and evidence

Evidence is never committed. A run record, a review capture or a log goes in the working folder,
`.orchestrate/evidence/<MODULE>/`, which every worktree on this Mac shares and Git does not hold; keep its
reproduction record (command, revision, tool versions, SHA-256) beside it. Data a check reads is test data,
tracked in `tests/fixtures/<module>/` or beside the crate's tests. A module's record of what it copied, from
where and under what licence is tracked at `docs/provenance/<MODULE>.md`. The merge refuses a new or changed
file under `docs/evidence/`. Large captures stay in `tmp/<task>/`. If a reviewer on another worktree must watch
a 60 MB capture that cannot sit in the working folder, run
`python3 scripts/evidence_store.py put tmp/<task>/capture.mp4 --path docs/evidence/MODULE/capture.mp4`: the
logical path is a name, never a tracked file, and the bytes go to the local store. Commit only the manifest it
updates, `.config/evidence-manifest.json`; the reviewer runs
`python3 scripts/evidence_store.py restore docs/evidence/MODULE/capture.mp4` for an ignored copy. Verify
recovery before cleaning your scratch source. The shared local store beside the main checkout has no remote
backup; a lost disk loses new local captures. Historical entries restore from Git in any full clone.
[Bulk review evidence](docs/ASSET-LIFECYCLE.md#bulk-review-evidence) explains missing local files, overrides
and hash verification.

For unattended submitted-output images and elapsed-time motion on Mac, use the [capture runbook](docs/CAPTURE.md).

Explain the result, rationale, tests actually run and remaining limits. Update the owning specification
and maturity entry when behavior or evidence changes. Visible output is captured and checked as in the
[Thrum engineering rules](.agents/skills/thrum-engineering/references/building.md#verify). The lead runs
orchestrate's `review` command for work packages marked Ready for Review ([project settings](ORCHESTRATE.md)).
Owner feedback is asynchronous and never blocks work. Current document hashes can be refreshed after reviewing
the diff; this does not update historical review hashes or promote any maturity status.
Documentation-only changes need `./dev check-docs`, not an unrelated engine test campaign.

Keep compact reproducible evidence, in the working folder, with exact source/build/fixture identity. Do not commit generated
build trees, transcripts, every rejected asset or bulk captures. [Storage rules](docs/ASSET-LIFECYCLE.md)
cover promotion and closeout. There is no remote or release configuration implied by this scaffold;
package publication is disabled pending a deliberate distribution and licensing decision.

The macOS V1 game accepts the comma-separated `THRUM_HARDWARE` launch list. A bare name
such as `ray-query` is required. A trailing `?` makes it preferred, as in
`THRUM_HARDWARE='ray-query?'`. Captures require all listed hardware. For a window launch,
the registry can decline an unavailable preference or retry a failed request once without
preferences. A host can supply typed requests through `Game::hardware_requests`.
Enabling ray queries admits the device feature. Scene preparation separately decides whether
to allocate and rebuild a ray structure.

## Exact image baselines and the reference guard

No exact-image proof is needed before a merge. After merges, the main-branch check `exact-images` runs
`python3 scripts/exact_baselines.py verify --full` on main's tip, in a clean checkout of its own. It needs
this Mac's Metal device, Chrome and at least 16 GiB free. It builds release native and Wasm outputs, cooks
fresh content, and compares both native matrices and all four Chrome sets exactly. One run covers every
commit merged since the last. A failure that holds on a second attempt is traced to the first failing
commit, and that commit's module fixes it or the commit is reverted. A mismatch names the baseline and its
owner; only that owner can refresh retained references under
[the baseline rules](docs/plans/ARCHITECTURE.md#exact-baselines).

Orchestrate's `main-check exact-images` command reports the run status.
The scheduled check runs independently of work-package completion.

The [registry](.config/exact-baselines.json) lists references, comparison files and covered paths.
`./dev check` runs the CPU-only `python3 scripts/exact_baselines.py guard`. It refuses changed references,
comparison files or baseline definitions unless their pins and refreshes are updated. Each pin hashes the
definition and the paths and Git object IDs of all protected files in HEAD. The guard prints the computed
pins; copy them into the registry after committing the protected changes. A refresh records `anchor` (an
existing anchor in `docs/plans/ARCHITECTURE.md`), `module` and the work package ID in the registry's `milestone` field.
A changed pin needs a changed refresh. Commit the registry update before checking; uncommitted protected changes fail.
A full run keeps compact records
with revision, binary/Wasm hashes, cooked root, comparisons and elapsed time; successful bulk frames are
removed after the owner publishes their comparisons or complete motion verification; source manifests and
sample identities are archived first. Failed captures remain for diagnosis.

A planned refresh needs a ruling in the baseline rules: regenerate and commit the references on the candidate,
hold the `gpu` lock for `verify --full`, update the pin and refresh, commit, and merge everything in one
landing with `agent_loop.py merge`. An owner editing a comparison file also updates each affected pin and
refresh, naming its ruling or the rule assigning ownership. Adding or redefining a baseline needs a correct
pin and refresh. To retire one, a planner moves its identity and owner to the registry's `retired` list with
a refresh naming the ruling that ends it. Removing an entry without retirement fails against the registry
at the merge base with main. Covered paths key the run's record; they impose no pre-merge image run.
The verifier keeps 48 completed runs' compact records and the latest failed run's captures in its own
`tmp/baseline-guard/`, pruning older completed runs at startup; incomplete runs are preserved for diagnosis.
