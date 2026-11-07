# Thrum starter

One cooked rock on lit ground under a physically based sky, a camera, two Sun sliders and a reloadable material
hook, in 42 lines of game code ([src/main.rs](src/main.rs), counting non-blank, non-comment lines). The game names no
engine game types: it uses the sky kit in `thrum_app::sky`, which brings the typed controls, cooked meshes, the
ground, the material hook and the window.

## Run it

On a Mac, from the repository root:

```sh
./dev starter
```

If this is a fresh checkout, first run `rustup run 1.99.1 cargo fetch --locked` once (see
[CONTRIBUTING](../../CONTRIBUTING.md)). `./dev starter` is `./dev scene examples/starter`, which:

1. builds `thrum-starter` and the `thrum-cook` cooker in release mode;
2. cooks [assets/](assets) into a store at `target/scenes/thrum-starter/store`;
3. starts `thrum-cook watch` on the same folders, so saved asset edits are cooked again;
4. opens the window with `THRUM_ASSET_STORE` set to that store, `THRUM_PARAMS` set to
   [starter.params.json](starter.params.json) and `THRUM_MATERIAL` set to [material.wgsl](material.wgsl);
5. stops the watcher when you close the window, or if the scene fails.

The panel beside the window holds the Sun's elevation and azimuth. Set `THRUM_REPORT=1` to print a frames report
(frame times and counters) when the window closes.

The first build links the cooker and prints a linker warning, `ld: no platform load command found in
'.../libintel_tex_2...', assuming: macOS`. It is harmless: the cooker's prebuilt texture-compression kernels carry no
platform tag, and macOS is the right assumption.

## Change it while it runs

- **Assets.** Save a change to `assets/meshes/rock.gltf` or `assets/textures/white.png` (any PNG of
  the same name). The watcher cooks a new generation, and the window logs
  `content generation N drawn` when it appears. To experiment on a copy instead, set
  `THRUM_SCENE_ASSETS=/path/to/copy` before `./dev starter`.
- **Parameters.** Edit [starter.params.json](starter.params.json) (display values, in degrees) and
  press `L` in the window. Unknown keys and out-of-range values are refused by name.
- **Material.** Edit [material.wgsl](material.wgsl), the rock's material hook, and press `L`. Its
  `cooked_surface` function returns the base colour, roughness, metalness and emission to shade
  with, and can use the fragment's `uv`, world position and normal; the file's comments list the
  fields and a moss example. The new material draws once it compiles, which takes about a second,
  while the old one keeps drawing. A mistake is printed with its line and column, such as
  `material.wgsl:17:21: no definition in scope for identifier`, and the previous material keeps
  drawing.
- **Code.** Edit `src/main.rs`, close the window and run `./dev starter` again.

## Make your own scene

1. **Copy** this folder: `cp -R examples/starter examples/<name>`.
2. **Name** the package in `examples/<name>/Cargo.toml`: `name = "thrum-<name>"` (the binary has the same name).
   Rename `starter.params.json` to `<name>.params.json`; `./dev scene` passes the file named after the folder.
3. **Join the workspace:** add `"examples/<name>"` to `members` in the root `Cargo.toml`.
4. **Register it** in `.config/architecture.toml`, next to `[packages.thrum-starter]`:

   ```toml
   [packages.thrum-<name>]
   manifest = "examples/<name>/Cargo.toml"
   allowed_workspace_dependencies = ["thrum-app"]
   ```

5. **Run** `./dev scene examples/<name>`. The first run adds the new package to `Cargo.lock` (offline, nothing is
   downloaded) and says so; commit that change with the scene. Every later run cooks, watches and hot-reloads your
   `assets/` exactly as the starter does, and reuses the starter's build.

Keep the shape of [src/main.rs](src/main.rs): one `main` behind `#[cfg(target_os = "macos")]`, an empty `main`
for other targets, and everything else in one module behind one `#[cfg(target_os = "macos")]`. The window runs
only on the Mac, and one `cfg` on the module is cheaper than one on every item.

## Write your scene

`thrum_app::sky::run(Config::new(name, title), build)` opens the window. `build` returns
`Result<SkyGame, Box<dyn Error>>`, so `?` works on control declarations and the kit's own errors alike. A
`SkyGame` is built from:

- `Controls`: the sliders and toggles shown in the panel and set by parameter files. `.slider(key, label, range)`
  takes `.default`, `.step` and `.display(Display::plain(decimals, unit))`, then `.add()?`.
- `Camera::look_at(position, target, vertical_fov_degrees)`.
- An update closure, run before every frame, that maps controls onto the `Sky`: `sun_elevation_degrees`,
  `sun_azimuth_degrees` and `camera`. A value the renderer cannot draw (for example a camera below the ground) is
  explained on standard error and the previous sky is kept.
- `.with_meshes(placements)?`: cooked meshes from your `assets/`, drawn and hot-reloaded from the store
  `./dev scene` sets.
- `.with_ground(Some(colour))` or `.with_ground(None)`; see [The ground](#the-ground).

World units are metres with +Y up and the ground at Y = 0. North is -Z and east is +X; the Sun's azimuth runs
clockwise from north. Exposure follows a camera meter, so twilight reads dim rather than grey.

`Config::new(name, title)` gives a 1600×900 window titled `title`, diagnostics prefixed with `name`, and
`THRUM_FRAMES=<n>` to close the window after `n` frames in scripted runs. Its fields are public: for example
`Config { window_size: [1280, 720], ..Config::new("thrum-x", "X") }`.

## Place meshes

A `MeshPlacement` names a cooked mesh and its base-colour texture by their paths under `assets/` without the
extension, and says where the mesh stands:

```rust
MeshPlacement::new("meshes/rock", "textures/white")
    .rotate_degrees([0.0, 0.0, 1.0], 90.0) // turn about a world axis (right-hand rule)
    .yaw_degrees(30.0)                     // turn clockwise seen from above, like the Sun's azimuth
    .scale(1.5)                            // uniform only
    .at([4.0, 0.0, -2.0])                  // where its origin goes, in metres
    .on_ground()                           // rest its lowest point at that height instead
    .material(CookedMaterial { base_colour: [0.4, 0.38, 0.34, 1.0], roughness: 0.9, metalness: 0.0 })
```

Turns apply in call order; `scale` and `at` can come anywhere. `on_ground` uses the cooked mesh's own vertices, so
you need not know its bounds, and it follows edits to the asset. The default material is the texture's own colour, roughness
0.55 and metalness 0. `base_colour` is linear RGB and alpha, each 0–1.

**Placements are rigid with one uniform scale.** Normals and culling and shadow bounds are transformed by the
placement, so a stretched, sheared or mirrored transform would draw wrongly. The builder cannot make one. If you
write `transform` by hand, a bad one is refused when the scene starts, naming the placement, the cause and the fix,
for example `mesh placement 2 ("meshes/rock") scales its axes unevenly (X 1, Y 3, Z 1); ...`. To stretch a mesh,
reshape it in its source file. The scale must lie within 0.0001–10000 and the origin within 1,000 km of the world
origin on each axis. That is the kit's conservative supported range, not where `f32` overflows; a transform without a
finite inverse is refused as well.

The starter's rock spans X −1.54 to 1.36 m, Y −0.83 to 1.00 m and Z −1.06 to 0.94 m around its origin, so it is
widest along X. `on_ground` rests it whichever way you turn it.

## The ground

The kit draws a flat ground at Y = 0, about 8 km across: a rough, dry-earth surface lit by the Sun and the sky and
receiving the Sun's shadows from your meshes. `.with_ground(Some([r, g, b]))` changes its linear base colour (each
0–1; the default is `thrum_app::sky::DEFAULT_GROUND`). `.with_ground(None)` removes it, leaving only the sky's own
distant planet surface below the horizon, which takes no shadows.

## Review a frame without a window

`THRUM_CAPTURE_PNG=<file>` renders one frame offscreen at the window size and writes it as a PNG,
after applying `THRUM_PARAMS`, waiting for the cooked meshes to draw and drawing `THRUM_MATERIAL` if it is set
(a hook that does not compile refuses the capture):

```sh
THRUM_ASSET_STORE=target/scenes/thrum-starter/store THRUM_PARAMS=examples/starter/starter.params.json \
  THRUM_CAPTURE_PNG=tmp/starter.png target/release/thrum-starter
```

Run `./dev starter` (or `./dev scene` for your scene) once first, so the binary and the store exist.

## When something is missing

The kit names the problem and the next step. Without `THRUM_ASSET_STORE`, or with a store that was never cooked,
it asks you to run `./dev scene`. A refused control declaration or mesh placement names the control or placement
and the fix. Cook failures are printed by `thrum-cook`, and the previous generation keeps drawing.

## Asset provenance

`assets/meshes/rock.gltf`, `rock.bin` and `assets/textures/white.png` are byte-for-byte copies of
`examples/v1-game/assets/showcase/`. They were generated in this repository by its stdlib Python
recipe `examples/v1-game/assets/showcase/generate.py` (commit `a5f29b7`), with no third-party
content. They are covered by the repository's own terms; Thrum has no distribution licence yet.
