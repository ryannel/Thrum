# A0 world-storage spike

Selected source and measurement inputs for the [A1 storage decision](../A1-STORAGE-SPIKE-2026-11-03.md). Owner: Thrum V1 A1. Versions are pinned in `Cargo.lock`: hecs 0.11.2, bevy_ecs 0.20.1, Divan 0.1.21, Rust 1.99.1. This isolated package is outside the engine workspace. Its 0.25 s Divan output and three serial p50/p99 runs are retained here; local build output is reconstructible and stays under `tmp/`.

Reproduce from the repository root with `CARGO_TARGET_DIR=tmp/world-storage-spike/target RUSTC_WRAPPER= cargo test --manifest-path docs/evidence/v1-workload/world-storage-spike/Cargo.toml --locked --release --offline` and similarly `cargo bench --bench storage` or `cargo run --bin measure`. For Wasm use `RUSTC=/Users/ryannel/.rustup/toolchains/1.99.1-aarch64-apple-darwin/bin/rustc` with `cargo check --target wasm32-unknown-unknown`; adjust only the toolchain path on another machine. No public engine API is implied by these adapters.
