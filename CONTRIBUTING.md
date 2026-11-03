# Contributing to Thrum

Use Rust 1.99.1 and Python 3.11+. `./dev` selects the pinned Rust toolchain even if a standalone
Cargo comes first on PATH. PowerShell users can use `./dev.ps1` with the same commands. The workspace lockfile is committed. Run `rustup run 1.99.1 cargo fetch --locked` once on a clean copy; checks then use locked offline Cargo commands.

| Command | What it does |
| --- | --- |
| `./dev help` | List implemented contributor commands |
| `./dev check` | Repository checks and their tests, rustfmt, Clippy and workspace tests |
| `./dev check-docs` | Local Markdown links/anchors, skill metadata, maturity hashes and dependency policy |
| `./dev check-web` | Compile the portable core for `wasm32-unknown-unknown` |
| `./dev check-mobile-core` | Compile the portable core for ARM64 iOS device, iOS simulator and Android |
| `./dev fmt` | Apply rustfmt |
| `./dev refresh-design-hashes` | Explicitly refresh current design digests after reviewing a design edit |

Before the web compile check, install `rustup target add --toolchain 1.99.1 wasm32-unknown-unknown`.
This is a portable library compile check, not a browser runtime/rendering test. The GitHub Actions
workflow describes Linux, macOS and Windows checks, plus Wasm compilation; a configured job is not
evidence it has run.
On macOS with Xcode, `./dev check-mobile-core` needs the pinned Rust targets
`aarch64-apple-ios`, `aarch64-apple-ios-sim` and `aarch64-linux-android` installed with `rustup target add --toolchain 1.99.1`.
It compiles only `thrum-core`; app packages, native activity/UI, devices and lifecycle remain separate checks.
`THRUM_PYTHON` can select the Python executable. Build products use `target/`, or an explicitly
owned `CARGO_TARGET_DIR`. `cargo clean` affects that build directory; inspect ownership before cleanup.

## Change the smallest complete interaction

Start from the active [build plan](docs/BUILD-PLAN.md), the owning specification and affected contracts.
Describe the behavior, its state owner and observable result before adding APIs. Keep simulation rules
independent of rendering, transport and platform I/O. Local and remote inputs use the same validation
and authority; transport does not become a second gameplay implementation.

Use idiomatic Rust: modules express ownership, types distinguish identities/units/lifetimes where useful,
`Result` carries expected failure, and public APIs document invariants and error behavior. Avoid hidden
global state, unchecked assumptions at external boundaries and blanket lint suppression. Workspace lints
forbid unsafe code by default; a future necessary adapter must justify a narrow exception and its safety
invariants in review. The [engine contributor skill](.agents/skills/engine-contributor/SKILL.md) records how
to choose work, match rigour to state and shape code.

Add a crate only for a demonstrated boundary, reuse or platform/build need. Register it in
[architecture policy](.config/architecture.toml); all workspace crates inherit common package/lint settings.
The dependency guard checks direct workspace edges of every dependency kind, including renamed and
target-specific edges. It does not prove semantic authority or validate all optional feature combinations.
Every `Cargo.toml` under `crates/`, `tools/` or `examples/` must join the workspace and architecture registry. Direct
third-party dependencies require an exact `=version` registry requirement and a policy entry with declared
license, purpose and target/build cost; the guard cannot verify the upstream license or transitive closure.
Record dependency purpose, exact version/license and target costs before adding external libraries. Do
not vendor Bevy code without source revision, applicable license/notices and a maintenance rationale.

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

Explain the result, rationale, tests actually run and remaining limits. Update the owning specification
and maturity entry when behavior or evidence changes. Current document hashes can be refreshed after
reviewing the diff; this does not update historical review hashes or promote any maturity status.
Documentation-only changes need `./dev check-docs`, not an unrelated engine test campaign.

Keep compact reproducible evidence with exact source/build/fixture identity. Do not commit generated
build trees, transcripts, every rejected asset or bulk captures. [Storage rules](docs/ASSET-LIFECYCLE.md)
cover promotion and closeout. There is no remote or release configuration implied by this scaffold;
package publication is disabled pending a deliberate distribution and licensing decision.
