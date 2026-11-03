# Held release-selection bridge candidate — 2026-11-03

## Decision and scope

The [prepared-environment design](../../design/T2-PREPARED-NATIVE-ENVIRONMENT.md#proposed-held-release-selection-bridge)
selects one canonical release-manifest SHA-256 as the wire identity. Its
origin-record and six proof digests form a transitive chain. The exact
private-cache manifest SHA-256 and four root inventories are a separate local
owner binding. This is an unready candidate; no reviewed real release manifest
or Apple-origin selection was supplied.

The macOS helper copies the manifest and seven evidence files into one private
append-only APFS selection, verifies canonical bytes and cross-references on
those copies, seals the clone, and keeps the record descriptors plus a shared
root lease open. It opens the selected Rust root with all Rust component file
witnesses collected during that root's existing complete cold inventory. A
four-root composition checks the release root inventories, exact cache-manifest
SHA-256, selected Cargo/Rustc identities and dependency key. Warm code-only
reuse checks the held leases and source key without a root scan or repeat tool
hash. The unheld matcher and existing unreviewed A/B guard path retain their
original full checks.

## Verification

- `PYTHONPATH=scripts python3.11 -m unittest scripts/test_hold_macos_release_records.py scripts/test_hold_macos_tool_root_clone.py scripts/test_verify_headless_release_inputs.py scripts/test_hold_macos_private_cargo_cache.py` passed on macOS ARM64. The release-record fixture freezes valid synthetic reports, changes the original origin file afterward, and still opens the selected copy; wrong reviewed manifest digest and crosswired cache proof refuse.
- A second fixture seals a tiny private cache and Rust/linker/SDK roots, opens all five leases, binds the canonical record selection, accepts a code-only edit and refuses a Cargo manifest edit. It injects a second-tool-hash failure to prove the Rust witness path supplies selected Cargo/Rustc bytes. A `format:2` versus canonical `format:2.0` cache-manifest substitution compares equal as Python objects but refuses by exact SHA-256. A selected Rust witness with a missing path or wrong digest refuses. An injected first-record close error still closes the other record descriptors and root lease.
- The same fixture starts the persistent Python guard in optional release mode against those sealed records and four roots. Its bounded startup line carries the canonical manifest SHA-256, a warm success reply carries the same digest and selected reuse key, a wrong-target reply carries that digest with an explicit refusal, and a wrong manifest SHA-256 fails before startup readiness. The existing unreviewed `READY` protocol remains unchanged when release mode is absent. This is a subprocess guard proof, not the Rust consumer or a real A/B build.
- `RUSTC_WRAPPER= ./dev check` passed after this candidate code was added. Independent review found and prompted fixes for record no-writer ownership, one wire identity, descriptor cleanup and exact cache-manifest binding.

One later full-gate run encountered `OwnerBusy` in the unrelated
`thrum-source` uncertain-COMMIT live-lease test. That named test passed on a
focused rerun, and a subsequent full `./dev check` passed. The one failure
remains a test-stability observation rather than proof of a bridge regression.

These are small synthetic correctness fixtures. They do not measure the full
4.86 GB cold selection, a complete warm edit, CPU time, memory allocation or
end-to-end speedup. The already measured three-root cold scan remains a
one-time selection cost; this bridge avoids a **second** such scan in its
proposed combined path, but the real admitted A/B build has not yet selected
it. The record bundle adds up to six bounded 512 KiB proofs and two bounded
64 KiB records to cold preparation, with no network fetch in warm requests.

## Remaining gates

Connect this exact record owner to the persistent Rust guard and retained
candidate evidence, then qualify a real admitted A/B build against a reviewed
manifest. Resolve the pending Apple official-origin route and independently
close support/runtime bytes, compiler reads, helper lifetime, ready
publication and the unexplained prior retained-A rerun failure. Nothing here
publishes a `BuildRevision` or sets `buildReady:true`.
