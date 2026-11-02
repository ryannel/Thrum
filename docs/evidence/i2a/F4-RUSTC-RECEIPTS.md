# Selected rustc invocation receipts for the first headless build

3 November 2026. **Local macOS ARM64 candidate, `buildReady:false`.** This
extends the [admitted A/B compile](F4-ADMITTED-HEADLESS-COMPILE.md),
[first-profile build design](../../design/F4-FIRST-HEADLESS-BUILD.md) and
[native release trust policy](../../design/T2-NATIVE-RELEASE-TRUST.md). It does
not qualify a general native compiler adapter or a target-ready `BuildRevision`.

## Mechanism and cost placement

Cargo's [stable `RUSTC_WRAPPER` hook](https://doc.rust-lang.org/cargo/reference/config.html#rustc_wrapper)
passes the selected rustc path and arguments to an executable. The local Rust
wrapper writes one exclusive, bounded binary receipt per invocation before
launching rustc, then appends its completion status. The receipt retains raw
ordered Unix argument bytes, working-directory bytes, the sorted child
environment after the two wrapper control variables are removed, and the
selected rustc path. It limits each field to 128 KiB, the receipt to 1 MiB,
arguments to 1,024, environment pairs to 512 and the set to 64. An incomplete,
failed, malformed or aliased receipt refuses verification. Rustc response-file
arguments refuse because their file bytes are not captured. The wrapper gives
the child only its recorded environment and inherits stdio; it uses no unsafe
code or additional dependency.

The Python verifier runs after Cargo exits. It requires the selected compiler,
source/cache/output roots, known fixed environment values, successful probes
and compiles, dep-info emission, one source file under a selected root per
compile, output and `--extern` paths under the fresh output root, dependency
library search under its `debug/deps`, and no executable Cargo hooks. It
compares each receipt's crate and source to Cargo's structured target and
requires each `--extern` path to be an artifact Cargo reported from the named
dependency unit. Cargo's macOS `DYLD_FALLBACK_LIBRARY_PATH` is retained exactly
and checked against the selected output, Rust toolchain and fixed fallback
locations; empty/duplicate/outside entries and other `DYLD_` overrides refuse.
An information-only probe may omit the variable. The first profile also
requires its three Cargo probes to use the admitted source root as working
directory and one of the two exact argument forms observed with Cargo 1.99.1;
an unknown query refuses. Library resolution remains an open read boundary.
The report includes per-unit receipt hashes, a receipt-set digest,
artifact paths and `buildReady:false`.

The wrapper allocates and writes once per rustc invocation, including probes;
it does no source-tree walk or per-frame work. The existing private-cache
integrity check remains the major warm per-edit scan (67.8 MB in the measured
fixture). In a separate warm, unsandboxed direct-checkout cost probe, three
alternating fresh-output pairs took 1,162/1,231/1,110 ms without the wrapper
and 1,196/1,154/1,151 ms with it, with eight receipts per wrapped build.
Medians were 1,162 and 1,154 ms, respectively. The variation exceeds the
observed median difference, so this probe detects no wrapper penalty at its
resolution; it is not a cross-host or full-adapter latency guarantee.

## Qualification

Build the wrapper first with the repository's selected Rust toolchain and run
the ignored opt-in fixture. On macOS, the fixture requires permission to apply
its native sandbox and a populated local Cargo registry for private-cache
preparation:

```sh
RUSTC_WRAPPER= CARGO_TARGET_DIR=/Users/ryannel/Workspace/thrum/target \
  rustup run 1.99.1 cargo build --locked --offline -p xtask --bin rustc_receipt
THRUM_PYTHON=python3.11 THRUM_TEST_NATIVE_SANDBOX=1 \
  THRUM_TEST_PREPARE_CARGO_HOME=1 \
  THRUM_TEST_RUSTC_RECEIPT_WRAPPER=/Users/ryannel/Workspace/thrum/target/debug/rustc_receipt \
  RUSTC_WRAPPER= CARGO_TARGET_DIR=/Users/ryannel/Workspace/thrum/target \
  rustup run 1.99.1 cargo test --locked --offline -p thrum-cook \
  admitted_workspace_builds_headless_game_and_failed_edit_keeps_prior_binary \
  -- --ignored --nocapture
```

The final local fresh-output A build admitted 54 files (1,257,073 bytes), took 1,297
ms for Cargo build, and yielded five successful compile receipts plus three
probes. The verifier matched all five compiles to distinct Cargo targets and
their selected source paths. It reported receipt-set digest
`508c4f6f0ec5d849ba24740a37e3e67efc5e1f26e56deaf08ffe463fd13191d0`.
The A binary was 2,345,736 bytes and ran successfully. The separately
admitted invalid-code B build failed; A's hash and execution still matched.
The source and private cache inventories were unchanged after both builds.
The receipt digest contains temporary absolute paths and is an observation,
not a portable BuildRevision input.

Python refusal fixtures cover incomplete or malformed receipts, wrong
compiler/source, duplicate unit, altered fixed environment, escaped artifact
or library path, proc-macro input, and an `--extern` path not reported by Cargo.
Another fixture refuses a source file inside the allowed root when Cargo's
target names a different source; a two-unit fixture accepts a valid dependency
and refuses the wrong extern name. The macOS dynamic-library search fixture
refuses an outside or empty entry and wrong private home. Another fixture
refuses a probe from the cache root or with an unrecognized query. Six Rust
wrapper tests cover byte preservation, limits, probe classification,
exclusive receipt creation, process outcomes and exact child environment.
`./dev check` passes; its normal test suite omits the expensive A/B fixture.

## Remaining release boundary

The wrapper and Cargo transcript describe invocations and reported outputs,
not every file rustc or the linker opened. The sandbox still allows named
system roots and broader metadata reads; loaded libraries, Xcode support
bytes, path ancestors and Cargo's `DYLD_FALLBACK_LIBRARY_PATH` resolution need
closure. The B build uses the wrapper, but its failed receipts have not been
audited. The wrapper executable itself is not part of a reviewed release
selection. The candidate has no reviewed release manifest, retained output
manifest, durable ready publication or game-run handoff. These gates remain in
the unchecked native compiler-adapter item of the [build plan](../../BUILD-PLAN.md).
