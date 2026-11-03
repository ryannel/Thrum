# R0 masked shadow and FXAA GPU diagnostic — 2 November 2026

**Evidence level:** isolated synthetic offscreen GPU diagnostic. It is not the
I2a admitted source/build path, the R0 PBR material closure, a production
renderer, visual acceptance or a sustained device-performance result.

The task experiment in `tmp/r0-gpu-proof/` used pinned Rust 1.99.1,
`wgpu 30.0.1` with Metal/WGSL and `pollster 1.0.1`. It ran on the reported
Apple M3 Max Metal adapter. One 256×256 synthetic scene contains a ground
quad and a masked checker fence. The same mask function runs in a shadow map,
reverse-Z camera depth and forward colour pass. A shifted light projection
puts a fence shadow on visible ground beside the fence. Scene colour is
toned/encoded into an RGBA16Float RGB+luma intermediate, then a first WGSL
port of pinned FXAA 3.11 Quality preset 12 writes the offscreen output.
A second deterministic encoded-image fixture exercises diagonal boundaries,
thin crossing strokes, low-contrast waves and intersecting edges independently
of the masked-scene pass.
The [source selection](FXAA-SOURCE-SELECTION-2026-11-02.md) records the exact
reference file and notices.

| Check | Observed result |
| --- | --- |
| Offline Rust build | Succeeded with `RUSTC_WRAPPER= rustup run 1.99.1 cargo build --offline` from task scratch |
| GPU readiness | Metal adapter/device, WGSL modules, four geometry/display passes plus FXAA pipeline created and submitted; readback completed |
| Five pixel classes | Lit ground `(64,128)=[150,172,185]`; fence hole `(128,128)=[150,172,185]`; opaque fence `(128,64)=[228,190,91]`; asymmetric projected hole `(190,64)=[150,172,185]`; adjacent projected blocker `(190,95)=[72,83,90]`. Recorded channel tolerances/assertions passed |
| Representative output | [256×256 diagnostic frame](R0-MASKED-FXAA-DIAGNOSTIC-2026-11-02.png), SHA-256 `eca04b9731a75398bd973455f143185c0868212f094f58942373ca753d7f58e8` |
| Independent encoded fixture | [256×256 FXAA fixture](R0-FXAA-FIXTURE-2026-11-02.png), SHA-256 `c8c92e7b0d0edeee69b452fe8b59c5afaca321807ac1c82fbbffa0aca6a6dfa8` |
| Scalar RGB comparison | A separate Python implementation reads each *actual GPU-produced* RGBA16Float input and compares the WGSL output with bilinear clamp sampling. Masked scene: 60,025 pixels outside the recorded threshold-ambiguity set, maximum difference 1/255; 164 pixels above 2/255, all at search-stop ties, maximum 16/255. Fixed fixture: 59,205 outside that set, maximum difference 1/255; 381 above 2/255, all at search-stop ties, maximum 69/255. The comparison assertions passed with that explicit exclusion. |

The exact Metal readbacks used by the comparison are retained below. Each
`.rgba16f` is 256×256 row-major RGBA half floats (8 bytes per pixel); each
`.ppm` is the raw 8-bit RGB output written from the mapped GPU buffer. The
gzip files use `gzip -n`, so archive metadata does not carry a local timestamp.
Decompress each pair under the short names in the table before running the
Python commands below. The raw SHA-256 checks identify the bytes actually
compared; the gzip SHA-256 checks identify the retained files.

| Pair | Retained gzip (SHA-256) | Decompressed name (raw SHA-256) |
| --- | --- | --- |
| Masked input | [R0-MASKED-ENCODED-RGBA16F-2026-11-02.gz](R0-MASKED-ENCODED-RGBA16F-2026-11-02.gz) `ef08c8ba7e8d3544f8ec593812230d4b062fbe6e742bea448fa638d0bfdabd0c` | `encoded.rgba16f` `83d2e3c00263e4824171daddb244df8a9ea5934777aa3ef51abb92da8704d7a7` |
| Masked output | [R0-MASKED-FXAA-RGB-PPM-2026-11-02.gz](R0-MASKED-FXAA-RGB-PPM-2026-11-02.gz) `a887c6f56f4a719fd1a2f605a3030e4e03008598432687b0d0365350fa463391` | `frame.ppm` `27adf95c491c2774897bc413bbb563135edb0d4a381239421967bcd7236c2198` |
| Fixture input | [R0-FIXTURE-ENCODED-RGBA16F-2026-11-02.gz](R0-FIXTURE-ENCODED-RGBA16F-2026-11-02.gz) `f971e50d0038c6fd84d6249f879d3002bb12308b124eb1eef64b517d14846ece` | `fixture.rgba16f` `e6aeb91cae563bfbaae9167d1d185f0d9fc51dab02594f2c2f8170f05c001dbd` |
| Fixture output | [R0-FIXTURE-FXAA-RGB-PPM-2026-11-02.gz](R0-FIXTURE-FXAA-RGB-PPM-2026-11-02.gz) `2fb6ea14fa64ebb6724486a8db16aee12b1933f814449580404a0026293fe475` | `fixture.ppm` `387ca7cf8dc03933a600b990e17712faffe90c31d43606ad565b02cc619da8c2` |

The small [source archive](R0-GPU-DIAGNOSTIC-SOURCE-2026-11-02.tar.gz),
SHA-256 `824120d7abd7e413cbd7e96402b26d4b12a5a16f5f8fd59e3760673702649909`,
retains the exact diagnostic Rust/WGSL files, scalar comparison, Cargo lock and reference notices
without build products. Extract it into `tmp/r0-gpu-proof/` to reproduce the
synthetic run. Build there with `RUSTC_WRAPPER= rustup run 1.99.1 cargo build --offline`,
run `target/debug/r0-gpu-proof` on Metal, then compare the two readbacks with
`python3 reference_fxaa.py encoded.rgba16f frame.ppm` and
`python3 reference_fxaa.py fixture.rgba16f fixture.ppm`.
An extraction into a fresh task scratch directory, decompression of the four
retained readbacks, raw-hash recheck and both scalar comparison commands
reproduced the counts above without rerunning the GPU.
The source identities at this run were `main.rs`
`7819a425c1b5cf64fe6ffe600778150b04d9be4078ea75b60c29cf503730c4e1`,
`proof.wgsl` `8911fdc464b50f6caf8ca842a529c8aab2553ed02c7e00c822b07e3adcecb468`
`fxaa.wgsl` `5f26af44c01e3e605c7822b1881b4200ec8bebb6dfcdbabd41fe850cacb64384`,
`fixture.wgsl` `e1ec7f5997b1b04cc9e317d2359576e4eb197dcbab6027555c5920dda2d1e215`
and `reference_fxaa.py` `ca471f34407c75fb628840b9d84eac6ce7bedb4e3639df0409fce162649dc648`.
Scratch is retained while this proof is developed; the archive is evidence,
not an engine crate or SDK example. The 256² texture/readback allocation is
about 4 MiB of logical texel and readback payload before driver overhead and
command storage; it is not the R0 720p/1080p residency result.

The diagnostic's pixels and image show that these selected synthetic cases
survive the GPU path. The scalar checker implements the pinned branch's selected
quality-12 path separately and conservatively marks comparisons within `1e-6`
of its early-exit, axis, pair, span-sign or search-stop decisions as ambiguous.
All differences above 2/255 have a scalar search-stop tie; many are on repeated
thin strokes. This is evidence of sensitivity at branch thresholds, **not** a
pass of whole-image numerical equivalence to an independently compiled HLSL
reference. GPU float arithmetic and filtering can choose different sides of an
exact scalar tie. The 69/255 maximum on a thin-stroke fixture is material and
needs perceptual inspection and a deliberate portability decision before
promoting the WGSL port. Text-like edges and moving clips still need comparison
at native size. The proof also omits F3/F4 identity,
real mesh inputs, the material/light law, UI, source edits, device loss,
multi-view extraction and measured cold/sustained work. The next experiment
should compare an independently compiled reference on more fixed and moving
images, inspect threshold-sensitive edges, then build one complete R0 authored
action-to-image path from the admitted I2a source.

An adversarial read found that the first shadow lookup used unflipped texture
Y, mirroring the projected mask. That earlier frame is rejected. The selected
run maps clip-space +Y to texture row 0 and checks two asymmetric projected
points: `(190,64)` must be a light-source mask hole, while `(190,95)` must be
blocked. The earlier four-class test would not have detected this error.
