# Thrum starter

One cooked rock under a physically based sky, a camera, and two Sun sliders, in about 80 lines
of game code ([src/main.rs](src/main.rs)). The game names no engine game types: it uses the sky
kit in `thrum_app::sky`, cooked meshes from `thrum-asset-gpu` and typed controls from
`thrum-controls`.

## Run it

On a Mac, from the repository root:

```sh
./dev starter
```

If this is a fresh checkout, first run `rustup run 1.99.1 cargo fetch --locked` once (see
[CONTRIBUTING](../../CONTRIBUTING.md)). The command:

1. builds `thrum-starter` and the `thrum-cook` cooker in release mode;
2. cooks [assets/](assets) into a store at `target/starter/store`;
3. starts `thrum-cook watch` on the same folders, so saved asset edits are cooked again;
4. opens the window with `THRUM_ASSET_STORE` and `THRUM_PARAMS=examples/starter/starter.params.json`;
5. stops the watcher when you close the window, or if the starter fails.

The panel beside the window holds the Sun's elevation and azimuth. Closing the window ends the run
and prints a frames report.

## Change it while it runs

- **Assets.** Save a change to `assets/meshes/rock.gltf` or `assets/textures/white.png` (any PNG of
  the same name). The watcher cooks a new generation, and the starter logs
  `content generation N drawn` when it appears. To experiment on a copy instead, set
  `THRUM_STARTER_ASSETS=/path/to/copy` before `./dev starter`.
- **Parameters.** Edit [starter.params.json](starter.params.json) (display values, in degrees) and
  press `L` in the window. Unknown keys and out-of-range values are refused by name.
- **Code.** Edit `src/main.rs`, close the window and run `./dev starter` again.

## Review a frame without a window

`THRUM_CAPTURE_PNG=<file>` renders one frame offscreen at the window size and writes it as a PNG,
after applying `THRUM_PARAMS` and waiting for the cooked rock to draw:

```sh
THRUM_ASSET_STORE=target/starter/store THRUM_PARAMS=examples/starter/starter.params.json \
  THRUM_CAPTURE_PNG=tmp/starter.png target/release/thrum-starter
```

`THRUM_STARTER_FRAMES=<n>` closes the window by itself after `n` presented frames, for scripted
checks.

## Write your own scene

`thrum_app::sky::run(config, || ...)` takes a `SkyGame` built from:

- `Controls`: the sliders and toggles shown in the panel and set by parameter files;
- `Camera::look_at(position, target, vertical_fov_degrees)`;
- an update closure, run before every frame, that maps controls onto the `Sky`
  (`sun_elevation_degrees`, `sun_azimuth_degrees`, `camera`);
- optionally `.with_contribution(Box::new(cooked_assets), status)`, where `status` reports
  `ContentStatus::Loading`, `Drawn(generation)` or `Failed(reason)`.

World units are metres with +Y up and sea level at Y = 0. North is -Z and east is +X; the Sun's
azimuth runs clockwise from north. Exposure follows a camera meter, so twilight reads dim rather than
grey. A value the renderer cannot draw (for example a camera below sea level) is explained on
standard error and the previous sky is kept.

## When something is missing

The starter names the problem and the next step. Without `THRUM_ASSET_STORE`, or with a store that
was never cooked, it asks you to run `./dev starter`. Cook failures are printed by `thrum-cook`,
and the previous generation keeps drawing.

## Asset provenance

`assets/meshes/rock.gltf`, `rock.bin` and `assets/textures/white.png` are byte-for-byte copies of
`examples/v1-game/assets/showcase/`. They were generated in this repository by its stdlib Python
recipe `examples/v1-game/assets/showcase/generate.py` (commit `a5f29b7`), with no third-party
content. They are covered by the repository's own terms; Thrum has no distribution licence yet.
