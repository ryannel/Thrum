# Cloud-factor volume fixture

Two 2×2×2 density volumes, `volumes/clear` (every texel 0) and `volumes/dense` (every texel 255),
cooked on macOS with the default profiles by
`cargo run --offline -p thrum-cook -- cook <source> <store>` from `TVOL` sources of those sizes
and densities. Only `HEAD` and `cas/` are kept. The cooked march test in `src/draw/tests.rs`
draws them through `CookedAssets` with the scene's Sun optical depth on, which selects the
cooked shader's cloud-factor variant.
