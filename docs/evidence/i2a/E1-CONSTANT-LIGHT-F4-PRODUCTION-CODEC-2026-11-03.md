# First production constant-light F4 codec slice

> Status after the code-quality cleanup: this inactive module was removed from the active tree. The historical implementation remains at `git show e331e78:crates/thrum-cook/src/constant_light.rs`; the measurements below describe that historical slice.

3 November 2026. The [selected 160-byte constant-light contract](../../design/modules/ENVIRONMENT.md#selected-first-f4-constant-light-product-and-binding)
now has a pure Rust codec in
`thrum-cook/src/constant_light.rs`.
It is a production module and public typed API, but it does **not** yet read
the authenticated F3 source pin, establish selected scene/environment/light
closure, create persistent cook identities or publish a ready build.

`ConstantLightProductV1::cook` accepts already F3-validated typed numeric
values. It checks UUID octet version/variant and distinct environment/light
IDs, finite values, and nonnegative photometric quantities. It applies the
specified ordered binary64 direction-length and colour-luminance expressions,
normalizes once, canonicalizes every zero to positive zero and checks the
cooked `1e-12` residuals. It writes the fixed 160 little-endian bytes at
explicit offsets. `from_bytes` checks exact length, magic, version, flags,
seven-bit unsupported mask, UUIDs, finite/signed numeric domains, positive
zero and cooked residuals without repairing payload bytes. The API does not
perform F3's authored `1e-9` direction/colour source checks; the admitted
source adapter must supply that proof before calling it.

Eight focused unit tests passed. Three complete payloads match the
[independent Python goldens](E1-CONSTANT-LIGHT-F4-CODEC-2026-11-02.md) from
the retained proof; a fourth independent `struct.pack` payload fixes distinct
ambient/background channel order. Tests cover all 4,096 choices of signs in
12 zero positions, every numeric slot's malformed finite/negative-zero
encoding, invalid headers/UUIDs, residual acceptance/refusal, nonnegative
finite extremes and zero/overflow normalization refusal. A read-only
review-agent pass found no actionable code defect.

Verification on this checkout:

- `RUSTC_WRAPPER= rustup run 1.99.1 cargo test --locked --offline -p thrum-cook constant_light`: eight passed.
- `RUSTC_WRAPPER= rustup run 1.99.1 cargo clippy --locked --offline -p thrum-cook --all-targets -- -D warnings`: passed.
- `RUSTC_WRAPPER= ./dev check`: repository documentation/policy check,
  118 Python tests (five skipped), workspace rustfmt/Clippy and Rust/doc tests
  passed after the eighth test was included.

The codec runs once per changed selected source, not per tick, view or
fragment. The 160-byte payload is a fixed logical size; this checkpoint does
not measure production allocation, old/new overlap, cook latency or
cross-target byte identity. Existing isolated nanosecond timings from the
proof package are not promoted to engine performance. Product hashes,
semantic `CookKey`, exact-source build binding and authenticated cache/load
checks remain separate next work. A valid payload alone is not a source or
build authority.
