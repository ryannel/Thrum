# R0 Metal GPU-incomplete lifetime diagnostic, 3 November 2026

**Result:** a native event held three nonempty ordinary-wgpu submissions
GPU-incomplete while the harness allocated a different-sized texture. After
CPU release, native GPU tail markers and every copied RGBA pixel passed in
three fresh processes on Apple M3 Max. This is a narrow lifetime observation,
not the four-role R0 graph or a device-memory/performance qualification.

The [source and lockfile archive](../ARCHIVE-ACCESS.md#archive-inventory)
has SHA-256 `5eb011fca0d237250c4d8a8ab7209b0e9a2941acc67c11aa6938234a737a7744`.
Inside it, `src/main.rs` has SHA-256
`baa851759a43f6ef19fb3bd6c0a6f89db950a9b3e1f1963515763a6d63ffbfd7`
and `Cargo.lock` has SHA-256
`db215e9e4572f48b9169efc087111c35f34fdce25636803f893732dc16015906`.
The harness was developed from repository revision `f0d3870` with Rust 1.99.1,
wgpu 30.0.1, objc2-metal 0.3.2 and pollster 1.0.1 on macOS 26.6.2 arm64.

The fixture creates one 1280×720 tracked `RGBA8Unorm` texture. Submission A
clears it red and copies it to A's readback buffer; submission B clears the
*same allocation* blue and copies it to B's distinct readback buffer. Before
releasing either submission, the CPU allocates a separate 1920×1080 texture
and submission C clears it green and copies it to C's buffer. The same CPU
thread finishes and submits A, then B, then C. A strict Metal queue relay
adds a native wait to every user command buffer, and each of these three
submits stages the same initially unsignaled CPU event. Distinct native tail
events mark the GPU end of each submission. This is proof instrumentation;
strict mode has extra per-command-buffer and per-submit work and is not an
ordinary-path timing setup. [Pinned Metal queue implementation](https://github.com/gfx-rs/wgpu/blob/v30.0.1/wgpu-hal/src/metal/mod.rs),
[Apple shared-event wait](https://developer.apple.com/documentation/metal/mtlcommandbuffer/encodewaitforevent(_:value:)).

Before CPU release, `Device::poll(Wait)` for **A, B and C** each timed out
at 10 ms, the gate value was zero and all three native tail values were zero.
The distinct 1080p allocation already existed while old A and B were still
submitted but GPU-incomplete. After setting the gate to one, all three tail
values became one and every pixel of A, B and C matched exact red, blue and
green respectively. The old texture, the new texture, all readback buffers
and the native events remained owned through verification. The selected
logical texel payloads were 3,686,400 and 8,294,400 bytes, with 11,980,800
bytes of old/new scratch overlap; separately charged readback payloads were
15,667,200 bytes. These are descriptor calculations, not driver allocation
or physical residency measurements.

The first attempt released the last application-owned gate reference at CPU
signal time. In that failing run, wgpu reported `QueueEmpty` while native
tail markers did not advance. The corrected final source retains the event
through native completion and all-pixel verification. Its guard also signals
the gate on an error, waits for submitted tail events, and retains native
event references until process exit if draining times out. That is fixture
lifetime protection, not general device-loss recovery. The pinned Metal
fence counts a native command-buffer `Error` as finished, so `QueueEmpty`
alone is not an execution-success oracle; the independent tail markers and
pixel checks are required. [Pinned fence status handling](https://github.com/gfx-rs/wgpu/blob/v30.0.1/wgpu-hal/src/metal/mod.rs).

The final archived source was rebuilt in release mode and executed in three
fresh processes. Each exited zero and printed:

```text
adapter=Apple M3 Max backend=Metal driver= device_max_2d=8192
wgpu completion=QueueEmpty; native tail markers immediately after poll=(1, 1, 1)
submissions A=SubmissionIndex { index: 1 } B=SubmissionIndex { index: 2 }; A/B same texture, C new texture; native gate held all three incomplete, every RGBA pixel passed
logical scratch bytes: old=3686400 new=8294400 overlap=11980800; readbacks separately=15667200
```

Reproduce from the repository root on a Metal host:

```sh
mkdir -p tmp/r0-gpu-incomplete-repro
tar -xzf docs/evidence/render/R0-GPU-INCOMPLETE-LIFETIME-2026-11-03.tar.gz -C tmp/r0-gpu-incomplete-repro
RUSTC_WRAPPER= rustup run 1.99.1 cargo run --offline --locked --release --manifest-path tmp/r0-gpu-incomplete-repro/Cargo.toml
```

**Still open:** the exact four R0 scratch roles and pass dependencies, F4
product/upload leases, two-credit admission/refusal, old/new retirement and
loading transaction, real output/surface ownership, other backends, physical
GPU high-water, frame cost and ordinary moving visual quality. This probe
uses three submissions, so it does not qualify the proposed two-credit
policy. Its gate blocks work before execution; it does not prove the state
of a partially executed frame or global serial execution of independent
command buffers.
