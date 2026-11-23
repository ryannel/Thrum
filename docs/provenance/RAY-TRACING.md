# RAY-TRACING fixture provenance

The M3 Maps fallback guard uses the Thrum lighthouse rock and a generated solid-white 1 × 4096 RGBA
texture. The rock product matches the existing cooked-view fixture. The texture's narrow width makes
Metal's 256-byte row alignment cross one upload budget while keeping the portable KTX2 file small.

To regenerate the store, copy `examples/lighthouse/assets/meshes/rock.gltf` and `rock.bin` into
`meshes/` under an empty source directory and put an opaque-white 1 × 4096 RGBA PNG at
`textures/white.png`. Run the workspace's locked `thrum-cook cook SOURCE STORE`. Retain `HEAD` and
the three CAS products in `crates/thrum-asset-gpu/tests/data/maps-fallback/`.
