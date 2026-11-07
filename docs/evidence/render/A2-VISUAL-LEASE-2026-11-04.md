# A2 committed visual-page lease

4 November 2026. `codex/a2-visual-snapshot` on Apple M3 Max, Rust 1.99.1, 10k W1 entities with 1.5k moving and the existing four-pass Metal V1 host. The first C02 consumer-specific page copies stable IDs, f64 positions and visual selectors into three preallocated SoA buffers. `Arc` leases prevent a held page from being rewritten; a full page pool skips only optional publication. The serial host now uses this page and releases it after render-column extraction. This is a lifetime step toward worker extraction, not worker execution.

| Check | Result |
| --- | ---: |
| Visual-page capture, Divan release, 100 samples | 32.52 µs median; 29.95–58.99 µs |
| Serial W1 tick, same benchmark process | 44.41 µs median; 42.66–48.33 µs |
| Four full-record snapshots, same process | 187.2 µs median for four copies |
| Assembled Metal frame with timestamps, 100 samples | 1.373 ms median; 1.23–34.46 ms |

The page has 48 logical payload bytes per retained row, so three 10k pages reserve about 1.44 MB plus container/allocator overhead. A 600-tick 10k-entity test with 1,000 spawn/despawn pairs and four existing full-record copies measured **zero test-thread heap allocations** after representative warmup while also publishing and releasing the visual page each tick. A separate test holds two pages across world ticks, observes explicit view shedding on a third publication, then reuses only the released slot; one-row capacity reports one dropped visual sample. The opt-in real Metal `host_gpu` test passes the integrated world-to-frame path. `cargo check` passes for `wasm32-unknown-unknown` and `aarch64-apple-ios-sim` libraries; these are compile checks, not browser or phone runs.

The previous recorded assembled frame median was 1.284 ms with timestamps, but these separate runs have wide tails and concurrent system variation; no frame-time delta is claimed. The current benchmark still reports 141 median benchmark-thread Rust allocator calls and 26.37 KB for the whole GPU frame, including wgpu calls; the authoritative tick plus visual-page publication retains its narrower zero-allocation assertion. Worker overlap, a complete C02 read-model bundle, owned GPU packets, real surface pacing and Chrome/iPhone runtime remain open.
