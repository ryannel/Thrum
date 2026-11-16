# Mapped index fixture

The 7,504-byte CAS product is the showcase's `rock`, cooked on macOS at `a3dd15da8`
with `cargo run --offline -p thrum-cook -- cook examples/v1-game/assets/showcase tmp/review-engine-m3/valley-before`.
The filename is its BLAKE3 content hash; the test verifies it before opening the mapping.
It contains 672 indices across three meshlets. The cooker and cooked format are unchanged by M3.
