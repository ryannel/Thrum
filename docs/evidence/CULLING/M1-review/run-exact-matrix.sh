#!/bin/sh
set -e
cargo build --locked --offline --release -p v1-game --features draw-probe,thrum-render/creation-counters && cargo build --locked --offline --release -p thrum-cook && rm -rf tmp/exact-m0 && mkdir -p tmp/exact-m0 && target/release/thrum-cook cook examples/v1-game/assets/showcase tmp/exact-m0/valley && python3 scripts/apple_tile_m1.py --store tmp/exact-m0/valley --output tmp/exact-m0/m1 && python3 scripts/apple_tile_m2.py --store tmp/exact-m0/valley --output tmp/exact-m0/m2
