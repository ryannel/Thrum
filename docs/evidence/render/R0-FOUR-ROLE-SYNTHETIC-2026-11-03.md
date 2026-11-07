# R0 four-role synthetic GPU dependency chain, 3 November 2026

**Result:** one 1280×720 offscreen frame ran the selected four V1 scratch
formats through shadow, depth, forward, tone and pinned FXAA/output passes on
Apple M3 Max Metal. An independent CPU calculation matched every one of
921,600 output pixels with maximum error **zero** code values. This is a
synthetic pass/format dependency proof, not an authored R0 game or a lifetime,
physical-memory or performance qualification.

The [source, lockfile, WGSL, notices and run archive](../ARCHIVE-ACCESS.md#archive-inventory)
has SHA-256 `dc542f56dbdb261f4f5ff14b766d4e1c6d58ae742b59ce9becb2e7c4fe0ede38`.
Inside it, `src/main.rs` has SHA-256
`751cba6cc39b718e03dc03a0f91d3c3113a44ded5a48629feaf3a5b37436575e`,
`Cargo.lock` has SHA-256
`aa75c78144136cccf78223f6905381caa08f83b2c9e49d7f395122710054a91d`,
and the `src/fxaa.wgsl` copy matches the unchanged
[FXAA package](fxaa-quality12-v1/README.md) at SHA-256
`5f26af44c01e3e605c7822b1881b4200ec8bebb6dfcdbabd41fe850cacb64384`.
The assembled shader changes that file's fragment entry signature into an
ordinary function and appends a small disclosed output wrapper; it is **not**
a byte-identical assembled shader. The archive carries the upstream notices
and licence. The standalone source was developed from local revision
`0ba7cd9` and finally run with local main at `e507d93`, Rust 1.99.1, pinned
wgpu 30.0.1 and pollster 1.0.1 on macOS 26.6.2 arm64.

| Texture role | Format and extent | Required usage | Actual dependency |
| --- | --- | --- | --- |
| Sun shadow | D32, 2048² | Render attachment, texture binding | Depth draw → four centred comparison taps in forward. |
| View depth | D32, 1280×720 | Render attachment | Depth draw → read-only depth test in forward. |
| Scene linear | RGBA16Float, 1280×720 | Render attachment, texture binding | Forward write → tone read. |
| Encoded scene | RGBA16Float, 1280×720 | Render attachment, texture binding | Tone write → FXAA read. |
| Independent output | RGBA8Unorm, 1280×720 | Render attachment, copy source | FXAA/output write → separate buffer readback. |

Every texture is 2D with one layer, mip and sample and no alternate view
formats. The fixture checks adapter format usage/filterability and adapter
and requested-device maximum 2D dimension before allocation. wgpu validation
reported no error. The scene clear is magenta, so missing forward coverage
would be visible. Sun and view depth are cleared to reverse-Z zero and each
receives a full-screen `.75` draw. Forward uses `Greater` with depth writes
off and a read-only loaded view-depth attachment. A first `.875` draw passes;
a later `.5` draw fails. Four Sun comparison taps at reference `.5` read the
`.75` shadow, returning zero. Sun result, surviving draw and immutable tag
occupy separate scene-linear RGB channels. Thus the selected scene storage is
`[.125,.125,.5]/1024` before tone. The scene target is sampled into the
selected exposure/luminance/Reinhard/gamut and sRGB encode formula, including
encoded-luma alpha; the encoded target feeds the pinned FXAA operator's
uniform-input early exit and an explicit no-UI output wrapper.

The CPU oracle uses f64 and separately rounds the two half-float target
values before final UNORM conversion. It allows at most one RGB code value
of numeric difference and requires exact alpha. The final release run
reported expected and first output `[93,93,176,255]`, **zero observed error**
for all 921,600 pixels, and no wgpu validation error. Counterfactual
arithmetic gives different visible red and green channels if Sun comparison
or the surviving depth-tested draw changes; this is analytical sensitivity,
not a separately executed mutant-GPU test.

Checked arithmetic before texture creation derived **35,209,216 bytes** of
four-role scratch, plus **3,686,400 bytes** of independent output and
**3,686,400 bytes** of row-aligned readback: **42,582,016 bytes** of explicit
logical payload. This excludes driver alignment, pipelines and other
allocations; no actual GPU residency or high-water was measured.

Reproduce from the repository root on a Metal host:

```sh
mkdir -p tmp/r0-four-role-repro
tar -xzf docs/evidence/render/R0-FOUR-ROLE-SYNTHETIC-2026-11-03.tar.gz -C tmp/r0-four-role-repro
RUSTC_WRAPPER= rustup run 1.99.1 cargo run --offline --locked --release --manifest-path tmp/r0-four-role-repro/Cargo.toml
```

The final source passed pinned rustfmt, release Clippy with `-D warnings`
and a rebuilt native Metal run. The separate [lifetime diagnostic](R0-GPU-INCOMPLETE-LIFETIME-2026-11-03.md)
holds actual GPU-incomplete work, but uses one RGBA8 scratch texture; the
two proofs have **not** yet been combined. Uniform output exercises only
FXAA's early-exit branch and cannot verify encoded-luma use, spatial
coordinates, edge filtering or PCF edge/border behavior. The output
decode/encode pair cancels for this fixture, so its separate transfer
functions are not independently proved. Stored D32 values are not measured
numerically. A typed graph validator, two-credit admission, resize overlap,
retirement, F4 leases, a real surface, device loss, named-target frame cost
and ordinary-game visual quality remain unrun.
