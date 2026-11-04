# A2 simulation-to-render diagnostic

4 November 2026. Apple M3 Max, Metal, macOS, Rust 1.99.1 release, wgpu 30.0.1. `RUSTC_WRAPPER= cargo run -p v1-scene --release` runs the actual example against M2's two-pass HDR/display renderer. It seeds 10,000 A1 hecs entities, 1,500 moving, advances 60 committed ticks, extracts adjacent tick views into reused SoA columns, maps them to the synthetic renderer's 10k instances, then renders 640×360 offscreen. The second extraction reused valid history for all 10k and observed 1,500 changed positions. GPU readback contained 108,608 foreground pixels. [The selected HDR/display capture](A2-SIM-RENDER-2026-11-04.png) was inspected; it is a grid diagnostic, not accepted V1 art.

| Observation | Local result |
| --- | ---: |
| One serial simulation tick | 46.0–54.3 µs across two HDR process runs |
| Simulation visit plus render extraction | 203.8–312.5 µs; single tick per run |
| CPU encode, 32 warmed samples per run | median 2.08–3.83 µs; p95 6.08–11.13 µs |
| Forward GPU pass, 32 warmed samples per run | median 0.230–0.244 ms; p95 3.74–3.82 ms |
| Tone/display GPU pass, 32 warmed samples per run | median 0.112–0.135 ms; p95 2.32–2.39 ms |
| Divan isolated 10k extraction, 100 samples | median 99.70 µs, fastest 96.91 µs |

Three focused extraction tests pass: adjacent/stale/visual-revision history, explicit presentation degradation counts, and zero measured heap allocations over 60 steady 10k extractions after two-frame warmup with ID churn. The first frame allocates during setup; the assertion covers reused steady frames. Strict crate Clippy, `wasm32-unknown-unknown` compile for both renderer and example, and the full `./dev check` gate pass. Wasm compilation is not a Chrome runtime test.

This proves a simulation-driven **offscreen** Mac render path and CPU publication shape. It does not update GPU instance buffers every frame, overlap extraction with simulation, supply C02 async snapshot/resource leases, cook or hot-reload an asset, render clustered lights/CSM, qualify sustained frame pacing, or run in Chrome/iPhone. The large GPU p95 tails remain unexplained; repeated offscreen submissions with explicit waits are not paced presentation and do not establish a frame budget.
