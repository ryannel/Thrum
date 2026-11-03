# Retained unready native output candidate

3 November 2026. One opt-in macOS ARM64 A/B qualification from the admitted
55-file first-profile workspace. The owning design is
[F4 first headless build](../../design/F4-FIRST-HEADLESS-BUILD.md), with T2's
[prepared native environment](../../design/T2-PREPARED-NATIVE-ENVIRONMENT.md).
This is a fixture proof, not a trusted builder or a target-ready `BuildRevision`.

## Behavior and boundary

After A's sandboxed Cargo build, the fixture checks selected Cargo units and
dep-info, reconciles the rustc wrapper receipts with the fresh Cargo transcript,
and requires successful Cargo completion. It then streams the 2,345,784-byte
executable into a separate owned candidate directory. The same directory
retains cooked scene bytes, all raw `.receipt` files, receipt-verifier output,
Cargo transcript, supervisor request/report, admitted build declaration,
private-cache candidate manifest, generated sandbox policy and a local
environment observation. That observation names the selected source/product,
target, cache-manifest hash, selected Cargo/Rustc executable hashes and observed
tool-file hashes and paths. It explicitly says installed roots and host runtime
are still mutable and unclosed.

`candidate.json` is compact canonical JSON with exact names, byte lengths and
SHA-256 values. Its separately held SHA-256 anchors reload; it is neither a
`BuildRevision` nor a ready manifest. Reload checks the accepted source root,
distinct `SceneProductRevision`, scene codec and content hash, complete file
membership, all retained bytes, raw receipt-set digest, and the relationship
between the cache manifest, environment, supervisor request and scene
declaration. It refuses nonordinary files and bounded-file violations.

The fixture removes A's materialized source and original Cargo output before
reloading and executing the candidate. Flipping a byte in the retained binary
refuses until restored; withholding one raw receipt refuses; a rehashed
environment record carrying B's `SourceRevision` still refuses. Replacing the
candidate directory with a symlink to the same bytes also refuses. Invalid B
fails to compile and makes no B candidate, while A reloads and runs with its
original binary hash. A report or file substitution requires changing the
external report digest as well, so the check depends on the caller retaining
that digest independently.

## Local cost and limits

The final local run used Rust 1.99.1, Python 3.11, the opt-in macOS sandbox,
prepared private Cargo home and rustc receipt wrapper. A's fresh-output build
took 3,342 ms; cache preparation took 1,228 ms. Candidate copy plus its first
internal verification took 161 ms, and the first reload after source/output
removal took 19 ms. The successful build had five compile receipts and three
probes; the private cache remained at 2,923 files and 67,792,387 logical bytes.
These are single local elapsed times, not tail or device guarantees. The copy
streams large files through a 64 KiB buffer; the scene/JSON records are bounded
small inputs. Peak RSS was not measured.

The helper exists only inside the opt-in Rust test. It does not atomically
publish across a crash or make the candidate a durable engine artifact. It
does not prove that raw receipts describe every compiler/linker/system read,
make installed tool/SDK roots immutable, review the release manifest, or
contain escaped child sessions. `buildReady` remains false. The next T2 gate
is complete selected runtime/input closure plus reviewed release selection;
only then can F4 publish and reload a ready target manifest.

Reproduce from the repository root with the pinned toolchain:

```sh
RUSTC_WRAPPER= rustup run 1.99.1 cargo build --locked --offline -p xtask --bin rustc_receipt
THRUM_PYTHON=python3.11 THRUM_TEST_NATIVE_SANDBOX=1 THRUM_TEST_PREPARE_CARGO_HOME=1 \
  THRUM_TEST_RUSTC_RECEIPT_WRAPPER="$PWD/target/debug/rustc_receipt" RUSTC_WRAPPER= \
  rustup run 1.99.1 cargo test --locked --offline -p thrum-cook \
  admitted_workspace_builds_headless_game_and_failed_edit_keeps_prior_binary -- --ignored --nocapture
```

The test requires the selected macOS sandbox and installed Xcode toolchain;
ordinary repository checks do not invoke it.
