# R0 queue reuse: narrow public-wgpu Metal diagnostic, 3 November 2026

**Scope:** one tracked `RGBA8Unorm` texture, two same-queue render-clear/copy
submissions at 1280×720, then a separately allocated 1920×1080 texture.
This supports the hazard-ordered same-allocation premise of the
[R0 schedule candidate](../../design/research/R0-RESOURCE-LIFETIME-SCHEDULE-2026-11-03.md).
It does not execute the four-role R0 graph or hold genuinely incomplete GPU
work during the resize. It is not a presentation, residency or performance
qualification.

The [source/lock archive](R0-GPU-LIFETIME-PUBLIC-DIAGNOSTIC-2026-11-03.tar.gz)
has SHA-256 `481674e2e213b335acdcf0298b94c8810ac2d1304548d7ae91857fd6ed267700`.
Inside it, `src/main.rs` has SHA-256
`bb496f4a3367c1d37b73b4b894c50f96fad7043ae856fab3ef18cc4e59fffa22`;
`Cargo.lock` has SHA-256
`123037c1c2c8cb7693930c77ac066cccb2712a3f56c56050e2c8e9600126bdae`.
The source was developed from repository revision `477d3a9` with Rust
1.99.1, wgpu 30.0.1, pollster 1.0.1, macOS 26.6.2 arm64. The run selected
Apple M3 Max through Metal and reported device maximum 2D dimension 8192.

Reproduce from repository root on a Metal host:

```sh
mkdir -p tmp/r0-gpu-lifetime-repro
tar -xzf docs/evidence/render/R0-GPU-LIFETIME-PUBLIC-DIAGNOSTIC-2026-11-03.tar.gz -C tmp/r0-gpu-lifetime-repro
RUSTC_WRAPPER= rustup run 1.99.1 cargo run --offline --locked --manifest-path tmp/r0-gpu-lifetime-repro/Cargo.toml
```

The final run printed:

```text
adapter=Apple M3 Max backend=Metal driver= device_max_2d=8192
submissions A=SubmissionIndex { index: 1 } B=SubmissionIndex { index: 3 }; A/B same texture, C new texture; every RGBA pixel passed
logical scratch bytes: old=3686400 new=8294400 overlap=11980800; readbacks separately=15667200
```

Submission A cleared the same 720p texture red and copied it into A's own
buffer *before* B cleared it blue and copied into B's buffer. C cleared a
new 1080p texture green. After queue completion, the harness mapped all
three buffers and checked **every RGBA pixel** against its exact expected
tag. It kept the old Rust texture handle until completion callbacks and
readback checks finished. A 720p texture's logical texel payload was
3,686,400 bytes; the new 1080p texture added 8,294,400, for 11,980,800
bytes of explicit scratch overlap. The three readback buffers added
15,667,200 bytes; total explicit payload was 27,648,000 bytes. These are
descriptor-size calculations, not observed driver allocations or physical
memory high-water.

A Sol High review found that the initial three-pixel check could miss mixed
pixels; the final source scans all pixels. It also removed an unnecessary
assumption that completion callbacks arrive in A-then-B order. The run proves
selected output order **after completion**, not that A or B were still
executing when C was allocated/submitted. Withholding application polling
cannot establish GPU-incomplete work. The pinned Metal implementation may
overlap *independent* command buffers; this test reuses the same tracked
texture, so it tests conflicting accesses, not global serial GPU execution.
[Apple hazard tracking](https://developer.apple.com/documentation/metal/mtlhazardtrackingmode),
[pinned wgpu-hal Metal source](https://github.com/gfx-rs/wgpu/blob/v30.0.1/wgpu-hal/src/metal/mod.rs).

**Still open:** a deterministic native gate proving old submission
noncompletion while new work is prepared; the four R0 scratch roles and
pass dependencies; F4 product/upload leases and two-credit admission;
old/new retirement, 60 MiB refusal/loading, device loss, real surface/output,
other backends and named device targets. Actual GPU memory, frame time and
visual quality were not measured.
