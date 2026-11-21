# Contributing to Thrum

Use stable Rust and Python 3.11+. `./dev` selects the stable rustup toolchain even if a standalone
Cargo comes first on PATH. PowerShell users can use `./dev.ps1` with the same commands. The workspace lockfile is committed. Run `rustup run stable cargo fetch --locked` once on a clean copy; checks then use locked offline Cargo commands.

The Bash `./dev` launcher defaults to an empty `RUSTC_WRAPPER`, overriding machine-wide Cargo wrapper configuration.
Set `RUSTC_WRAPPER` explicitly when opting into a compiler wrapper for a particular invocation.
Both launchers set `CARGO_INCREMENTAL=0` for `check`, including the xtask build and its child Cargo
commands. The pinned compiler has failed verification with unstable incremental trait-obligation
fingerprints after revision changes; the gate therefore does not reuse that query cache.

| Command | What it does |
| --- | --- |
| `./dev help` | List implemented contributor commands |
| `./dev check` | Repository checks and their tests, rustfmt, Clippy, workspace tests (including Mac Metal) and V1 game Wasm Clippy |
| `./dev check-docs` | Local Markdown links/anchors, skill metadata, maturity hashes and dependency policy |
| `./dev check-web` | Lint/compile the V1 game, starter and kit test targets for `wasm32-unknown-unknown` |
| `./dev check-mobile-core` | Compile the portable core for ARM64 iOS device, iOS simulator and Android, plus the starter facade for iOS simulator |
| `./dev fmt` | Apply rustfmt |
| `./dev starter` | Cook and watch the [starter](examples/starter/README.md)'s assets and open its window (macOS) |
| `./dev scene <folder>` | The same for any scene package, such as your copy of the starter (macOS) |
| `./dev refresh-design-hashes` | Explicitly refresh current design digests after reviewing a design edit |

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

Start from [the brief](docs/plans/BRIEF.md), your module in [the module list](docs/plans/ARCHITECTURE.md), the owning specification and affected contracts.
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

`./dev check` is the hard gate: pinned rustfmt, workspace Clippy, Rust tests and repository checks.
Clippy runs `all` and `pedantic` plus selected restriction lints (`Cargo.toml`, thresholds in `clippy.toml`)
with every warning denied. Suppress only with `#[expect(lint, reason = "...")]` on the narrowest item;
`#[allow]` is rejected and an unfulfilled expectation fails. Production code does not `unwrap` or print,
keeps `pub` for exported items only, and documents every unsafe block of an approved exception.

The repository check also keeps `#[cfg(test)]` to test modules in production files and requires a reason
on every `#[ignore]`. Size gates are generous backstops, not targets: a function fails above 200 lines, a
production file notes above 1,000 code lines and fails above 2,500, and a test file notes above 2,000 and
fails above 4,000 (blank, comment and inline-test lines excluded). A note asks whether the file still holds
one responsibility; split by responsibility, never to satisfy a count. Folder file counts and argument
counts are not gated.

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
[Thrum engineering rules](.agents/skills/thrum-engineering/references/building.md#verify), and milestones marked for review close through
`/review-milestone` ([ORCHESTRATE.md](ORCHESTRATE.md)); owner feedback is asynchronous and never blocks work. Current document hashes can be refreshed after
reviewing the diff; this does not update historical review hashes or promote any maturity status.
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

`python3 scripts/exact_baselines.py verify`, without `--full`, draws nothing. It asks whether a passing run
on main contains the current commit, and is the check of a done-when item in most modules. Don't run it
before merging: on a branch it answers that the commit isn't on main yet.

The [registry](.config/exact-baselines.json) lists references, comparison files and covered paths.
`./dev check` runs the CPU-only `python3 scripts/exact_baselines.py guard`. It refuses changed references,
comparison files or baseline definitions unless their pins and refreshes are updated. Each pin hashes the
definition and the paths and Git object IDs of all protected files in HEAD. The guard prints the computed
pins; copy them into the registry after committing the protected changes. A refresh records `anchor` (an
existing anchor in `docs/plans/ARCHITECTURE.md`), `module` and `milestone`. A changed pin needs a changed
refresh. Commit the registry update before checking; uncommitted protected changes fail.
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
