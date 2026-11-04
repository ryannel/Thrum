# First constant-light F4 codec: isolated proof — 2 November 2026

The selected [E1/F4 product contract](../../design/modules/ENVIRONMENT.md#selected-first-f4-constant-light-product-and-binding)
has a standalone typed-input Rust codec proof. It does **not** consume an
admitted-source lease, publish a production F4 product, start an engine run or
qualify a renderer. The three small source trees are exact-source identity
fixtures, not independently admitted roots. The selected proof package ID is
`thrum.e1.constant-light.f4-codec-proof.v1`.

The [retained source, fixtures, goldens and verification record](../ARCHIVE-ACCESS.md#archive-inventory)
are a 23,876-byte archive, SHA-256
`376a60110afebd55aa91a251707a180cef3ea5832e6ed136c57215b8ffc820b6`.
All 32 archive files were compared byte-for-byte with the selected scratch
files after cleanup; the package includes its own `inventory.txt` and
`verification.txt`. Rust source SHA-256 is
`315f30adb7e7d98e006ef798404ea3772ed26144cb842cdfe004154738a5fbd5`;
Python oracle SHA-256 is
`6c543bc643e3334dd575fbf6e8b22fa45142c53d2592efc1855fe2a6144b7aa5`.
The code is original proof code using the workspace's locked `sha2 0.11.1`;
there is no borrowed engine implementation or asset.

## What was proved

Rust cooks already-typed schema-1 values, normalizes f64 direction and linear
Rec.709 colour in the specified order, writes exactly 160 little-endian bytes,
and refuses invalid decode inputs. A separately written Python standard-library
oracle compares the **complete** bytes, BlobHash, semantic CookKey, product
revision and exact-source build-binding digest for cardinal, negative/noncardinal
and all-nonzero negative-Z directions. The latter closes an adversarially found
coverage gap for the Z component. A second audit finding was corrected by
comparing every cosmetic edit's binding digest with its exact Python golden,
not merely checking that it changed. The original two goldens stayed unchanged.

Nine tests passed in both debug and release. They cover signed-zero
canonicalization, strict length/magic/version/flags/mask/UUID/finite/quantity
and residual refusal, equal-ID refusal, cache key separation and reuse,
corrupted cache bytes/manifest refusal and stale binding refusal. The tests
include source values that cook to identical payload bytes but keep distinct
semantic keys, and cosmetic exact-source changes that keep the product identity
while changing the binding. Rustfmt and Clippy with warnings denied passed.
The archive's `README.md`, `oracle.golden` and `verification.txt` give the
exact grammar, commands, tool versions and all three result hashes. Reproduce
after extraction into an owned `tmp/<task>/` with `python3 run_proof.py`.

The proof-local CookKey/manifest/binding domain strings and byte grammar are
**not persistent F4 identity choices**. The governing design selects their
contents, but production must choose and pin canonical encodings before cache
or build publication. The proof is independent at the codec/golden level; it
does not prove the F3 typed decoder and cooker are connected through a verified
source lease.

## Cost scope and next gate

One warm release microprobe on Apple M3 Max / Rust 1.99.1 sampled 500,000
iterations per function: cook 22.216, encode 11.460, decode 13.683 and
SHA-256 of 160 bytes 66.195 ns/op. These are isolated function measurements;
the earlier audit replay differed materially, so they are **not** an engine
latency or stable performance estimate. The serialized payload is 160 bytes;
on this target the typed Product is 144 bytes, product manifest 64 bytes and
build binding 192 bytes. Two payload/manifest/binding sets account for 832
layout bytes excluding allocation, source, cache and runtime ownership.

Next: an authenticated pinned admitted-source lease must feed the F3 typed
decoder, the cooker must select the exact active scene→environment→light
closure, and F4 must verify cache/product bytes and bind the exact
SourceRevision before publication. The joined fixture must measure source
decode/index work, allocations and old/new overlap. GPU f64→f32 envelope,
visual quality, frame cost and cross-device behavior remain separate gates.
