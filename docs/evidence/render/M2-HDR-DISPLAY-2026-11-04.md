# M2 pre-exposed scene and SDR display diagnostic

4 November 2026. Bounded proof of the [M2 scene-colour decision](../../design/decisions/M2-HDR-DISPLAY.md) on Apple M3 Max/Metal, macOS 26.6.2, Rust 1.99.1 and wgpu 30.0.1. The existing 10,000 static cubes at 1920×1080 now use a pre-exposed `Rgba16Float` scene attachment and a separate fixed-exposure tone/sRGB pass. This does not establish a full renderer, frame pacing or supported-device budget.

## Behaviour

The explicit real-adapter test passed with an independent f64 direct-light and display oracle. It checks 65×33 padded SDR readback; direct/material response; emitted radiance 2 and 8 at exposures 0.25 and 1; chromatic HDR gamut compression; maximum finite emission; background, emission and depth order; stale generation/exposure; an undersized two-pass timestamp batch; and repeated enabled/disabled timestamp use. The same prepared pipelines also render a 97×49 resized target with checked pixels and readback length. The typed graph reports forward → display → optional readback. An sRGB PPM capture and a 480×360 native-pixel crop were inspected: the red/blue cube grid and distinct lit faces are visible with no obvious invalid pixels. The image remains a static procedural diagnostic without AA, shadow or artistic content.

The final capture command was:

```sh
RUSTC_WRAPPER= THRUM_RENDER_CAPTURE=/Users/ryannel/Workspace/thrum/tmp/m2-hdr-display/diagnostic.ppm \
  rustup run 1.99.1 cargo test --locked --offline -p thrum-render --test gpu -- --ignored --nocapture
```

Metal adapter discovery required an approved unsandboxed test run; the sandboxed attempt could not discover an adapter. The 6,220,879-byte PPM and converted QA crop are scratch outputs under `tmp/m2-hdr-display/`, not retained Git evidence.

Fixture identity is synthetic revision 3, 10,000 static instances, two materials, fixed orthographic camera, default directional source and exposure 1 for the timing/capture run. SHA-256 of the checked inputs: `synthetic.rs` `2c752cf28846018ac2480fd5bf4ce5b193c64c8a26ec5baebe1f6b98fc72df78`; `synthetic.wgsl` `5a2c9a2ee0051f616544e413cb784ee4202645820ad6f3c0d290e4222773b6b0`; `synthetic_display.wgsl` `f74626bca1b4fac8e2f878e6a2fad673a8517835f7b5da78103a258eca3c468a`; `tests/gpu.rs` `b3aa00479510c33a7cad3ae0a9914cac4413bc1f459ff4d39eb3052c45a35118`.

## Isolated cost

One capture run used five warmup pairs and 32 sequential measured pairs, with host completion and a separate timestamp resolve between timed and untimed renders. Values exclude submission/poll, readback, game simulation, cooking and sustained presentation. Cold forward+display preparation in this run was 7.452 ms.

| Interval | Median | p95 |
| --- | ---: | ---: |
| GPU forward opaque/depth | 0.121625 ms | 0.123500 ms |
| GPU fullscreen tone/sRGB | 0.082334 ms | 0.083583 ms |
| CPU encode plus timestamp finish | 11.958 µs | 29.958 µs |
| CPU encode without timestamps | 8.583 µs | 14.500 µs |

The two GPU intervals are separately measured and should not be summed into a frame-time claim. At 1080p, the logical HDR scene payload is 16,588,800 B and the final SDR payload 8,294,400 B, totalling 24,883,200 B; depth has 2,073,600 samples with backend-selected physical storage. Peak residency, physical bandwidth, steady allocation, browser/iPhone execution and sustained frame pacing were not measured. A read-only Sol High defect review found no actionable regression; its requested chromatic and resize cases passed in a later focused Metal run. That run's timing was noisy and is not used as a performance comparison. Per-pass nonfinite/half-float clamp counters and a visible repair diagnostic remain unimplemented.
