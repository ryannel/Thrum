# Held private Cargo cache candidate

3 November 2026. Source commit
`0aad9a02c525df23463aa4b5aaca24d8475e2783` adds the unready
[`hold_macos_private_cargo_cache.py`](../../../scripts/hold_macos_private_cargo_cache.py)
candidate. On macOS 26.6.2 ARM64, five focused fixtures passed with
`python3.11 -m unittest discover -s scripts -p test_hold_macos_private_cargo_cache.py -v`.
`RUSTC_WRAPPER= ./dev check` also passed, including all 112 Python tests
(three skipped) and the Rust workspace checks.

The cold preparer compares the pruned source cache to its existing
`thrum.cargo-home.v1` manifest identity, inventories a separate whole-tree
digest including directory membership, clones and seals the tree through the
existing append-only APFS root mechanism, then rechecks the legacy digest.
It retains an exact sealed manifest copy and publishes a canonical unready
cache-selection record last. Cold open verifies the generic sealed-root lease,
the cache record, the exact manifest hash and legacy cache digest. The returned
handle holds the root lock and file descriptors; its warm check inspects only
their identities and immutable flags.

The fixtures cover a sealed cache with two simultaneous readers excluding an
exclusive owner, write refusal, an extra empty directory that changes the
whole-tree digest while leaving the legacy digest unchanged, a modified
manifest copy, interrupted cache-record publication, and an injected sidecar
close error that still releases the root lease. A partial digest name is not
repaired or selected. Test-only teardown clears flags on its owned temporary
tree; the production candidate has no unseal or deletion path.

These fixtures use a tiny synthetic cache. They do not exercise Cargo against
the sealed tree, bind a dependency-selection key to a real A/B job, authenticate
registry origin, measure a full cache or the complete warm edit path, or
provide an OS-enforced same-user mutation boundary. No release selection,
ready build or `BuildRevision` is claimed; `buildReady:false` remains required.
