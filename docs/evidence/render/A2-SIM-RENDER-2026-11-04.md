# A2 simulation-to-render diagnostic

4 November 2026. Apple M3 Max, Metal, macOS, Rust 1.99.1 release, wgpu 30.0.1. `RUSTC_WRAPPER= cargo run -p v1-scene --release` runs the actual example. It seeds 10,000 A1 hecs entities, 1,500 moving, advances 60 committed ticks, extracts adjacent tick views into reused SoA columns, maps them to the synthetic renderer's 10k instances, then renders 640×360 offscreen. The second extraction reused valid history for all 10k and observed 1,500 changed positions. GPU readback contained 108,608 foreground pixels. [The selected capture](A2-SIM-RENDER-2026-11-04.png) was inspected; it is a grid diagnostic, not accepted V1 art.

| Observation | Local result |
| --- | ---: |
| One serial simulation tick | 46.8–47.2 µs across two process runs |
| Simulation visit plus render extraction | 193.7–203.9 µs |
| CPU encode plus readback/timestamp commands | 65.0–66.2 µs |
| GPU pass timestamp, one frame per run | 0.541–1.959 ms; variable, no budget claim |
| Divan isolated 10k extraction, 100 samples | median 99.52 µs, fastest 97.79 µs |

Three focused extraction tests pass: adjacent/stale/visual-revision history, explicit presentation degradation counts, and zero measured heap allocations over 60 steady 10k extractions after two-frame warmup with ID churn. The first frame allocates during setup; the assertion covers reused steady frames. Strict crate Clippy, `wasm32-unknown-unknown` compile for both renderer and example, and the full `./dev check` gate pass. Wasm compilation is not a Chrome runtime test.

This proves a simulation-driven **offscreen** Mac render path and CPU publication shape. It does not update GPU instance buffers every frame, overlap extraction with simulation, supply C02 async snapshot/resource leases, cook or hot-reload an asset, render clustered lights/CSM, qualify sustained frame pacing, or run in Chrome/iPhone. The two GPU samples disagree enough to require sustained isolated measurement before any frame-budget claim.
