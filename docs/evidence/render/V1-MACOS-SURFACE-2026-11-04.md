# V1 macOS surface qualification

4 November 2026. The [surface handoff decision](../../design/decisions/V1-MACOS-SURFACE.md) extends the bounded 10k-entity/1.5k-moving host with one prepared fifth pass. This is a partial result, not a visible-window or frame-budget qualification.

## Passed

- `./dev check`: repository guards, strict workspace Clippy, unit/integration tests and docs passed in the surface worktree.
- `cargo test -p v1-scene --test surface_gpu -- --ignored --nocapture` on Apple M3 Max/Metal: both tests passed. The 640×640 Bgra8Unorm surface-format target has black bars outside the 640×360 picture; every picture pixel matches the source capture in orientation and encoded RGB bytes. Bgra8UnormSrgb differs by at most two 8-bit steps per channel after decode/hardware re-encode. Both variants completed the four scene plus one surface timestamp passes without a timing fault.
- A missed surface view committed tick 1 without submission or extraction, incremented the surface-drop counter once, then a 1000×1000 graph resize reused the prepared pipeline and presented tick 2. The initial scene is 10k entities with 1.5k moving.
- `cargo check -p thrum-render -p v1-scene --target wasm32-unknown-unknown` and `--target aarch64-apple-ios-sim` passed with the pinned Rust toolchain. These are compile checks, not browser/phone runs.

The integrated Divan benchmark on source base `93c3b14` plus this benchmark-only change (`cargo bench -p v1-scene --bench frame -- --sample-count 100`) compared the same 10k/1.5k-mover Metal frame with and without the prepared surface pass, each with 100 samples and GPU completion included. The target was a synthetic Bgra8Unorm texture, not a macOS drawable; setup/pipeline creation was outside the timed sample. Divan's allocator profiler counts Rust allocations on its benchmark thread, not GPU driver or other-thread allocations.

| Frame profile | Median wall | Median benchmark-thread allocator calls | Median benchmark-thread allocated payload |
| --- | ---: | ---: | ---: |
| Four scene passes, timestamps | 1.284 ms | 141 | 26.37 KB |
| Four scene plus 1280×720 surface pass, timestamps | 1.272 ms | 155 | 27.47 KB |
| Four scene passes, no timestamps | 1.023 ms | 80 | 14.39 KB |

The additional pass adds 14 benchmark-thread allocator calls and about 1.10 KB of benchmark-thread allocated payload per frame in this setup. Wall-time ranges overlap (1.168–14.93 ms and 1.186–13.02 ms), so these medians do not establish a speedup or regression. The benchmark includes GPU waits and excludes drawable acquisition, compositor pacing, driver allocations and a visible window; it cannot qualify a device frame budget.

## Pending real window result

The first optimized `v1-scene --bin window` created a winit window and advanced 2,400 world ticks, but acquired zero surface textures. Review found that Metal's `nextDrawable` can block with its timeout disabled, so acquisition moved to a zero-capacity request/result worker before repeating the run. A second review caught surface reconfiguration races; the final handshake waits for old textures and GPU packets, and tracks worker configuration separately from host presentation. The final process committed 122 ticks while receiving 121 explicit `Occluded` results, with two callbacks waiting for acquisition and 122 optional views dropped; no drawable acquisition ran in the tick callback. The computer UI reports **the Mac is locked**, which prevents the native drawable run. The app stops after 120 occluded callbacks and reports occlusion, waiting and stale acquisitions. A 600-presented-frame run on an unlocked display, visual inspection, resize trial, complete callback CPU and fifth-pass GPU measurements remain required. No surface frame-time or visual acceptance claim is made from the texture-format readback.
