# Mirror pool

`./dev scene examples/mirror` cooks the assets and opens the scene on macOS. The controls set eye height,
look-down angle, pan, orbit, and the Sun's elevation and azimuth. The moving beacon follows the scene's
presentation clock. The pool asks for one full-size secondary view, then combines the reflected scenery
with the air from the pool to the real camera.

`./dev scene examples/mirror --web` serves the same authored scene and material on hardware Chrome.
The [secondary view author guide](../../docs/EFFECTS.md#secondary-views) describes the runtime
controls, including their ranges and what part of the draw each changes.

The tower, rock and white texel are copied from the [lighthouse example](../lighthouse/README.md).
Run `python3 examples/mirror/generate_meshes.py` to regenerate the post, bank, ground and hill.
The water is 250 m above sea level. The far bank rises from the waterline so its surface, including
the light around the lamp and the shadows of nearby objects, is visible in both views.
The [scene material hook](material.wgsl) paints the tower and lamp.

## Submitted Metal inspection

From the repository root, build the scene and cooker. Use a new output folder under `tmp/`.

```sh
cargo build --locked --offline -p mirror-scene -p thrum-cook
mkdir -p tmp/scene-views-m2
target/debug/thrum-cook cook examples/mirror/assets tmp/scene-views-m2/store
python3 "$ORCHESTRATE_LOOP" lock gpu --worktree . -- env \
  THRUM_ASSET_STORE=tmp/scene-views-m2/store THRUM_MATERIAL=examples/mirror/material.wgsl \
  THRUM_MIRROR_INSPECT=tmp/scene-views-m2/run target/debug/mirror-scene
python3 examples/mirror/package_inspection.py tmp/scene-views-m2/run tmp/scene-views-m2/review
```

The driver reads 2560×1440 ordinary T2 submissions. Its fixed checkpoints set named poses, and the
motion arms evaluate the controls at monotonic elapsed time. It waits for each submitted image and records
the opportunity, skipped opportunities, submission, world tick and capture time. The packet contains
lossless PNGs, lossless RGB movies with submitted frame spacing, and their source ledger and hashes.
