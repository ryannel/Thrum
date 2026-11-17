# Water

`./dev scene examples/water` cooks this example's grey calibration float and opens the sea under
the engine's sky. The game selects its own 60 Hz world, trade wind, longer swell and sea declaration.
The panel controls the Sun, camera height and bearing, and the sea's stage. The float is a lighting
reference; physical floating bodies land in WATERS M5.

`./dev scene examples/water --web` serves the same scene and cooked float in Chrome with WebGPU.
The page pauses its world while hidden. If the graphics device is lost, **Recover graphics**
rebuilds its GPU resources while retaining the scene, controls and clock.
`node scripts/check_water_browser.mjs --case first-frame` compares the submitted sea fields
with the f64 model; `--case frames` also covers interpolation, repeats, visibility, a hitch
and device replacement. Run either under the orchestrate script's Chrome lock.
The `fixed` and `realtime` cases save submitted 2560×1440 images and source/time records;
their page callback intervals describe the page clock, not physical display cadence.
`THRUM_WATER_OUTPUT` selects a new scratch folder under `tmp/waters-M3/`.

`thrum-water` owns one contribution containing the ambient wave passes and the after-opaque surface.
It takes the level from the shared CPU map at each frame boundary and borrows all lighting and exposure
from the scene. It holds no example presets or cooked content. This first surface supports a sea;
other bodies, shore masks, interaction waves and full optics follow in their planned milestones.

The source for the calibration float and white texel is `make_float.py`; running it regenerates the
committed assets. No external asset or Broadsides code was copied.

For submitted Metal evidence, build the proof driver before taking the GPU lock:

```sh
cargo test --release --locked --offline -p water-scene --test proof_gpu --no-run
```

Under the orchestrate script's `lock gpu`, run `python3 scripts/capture_water.py --capture-only --mode fixed --output
tmp/waters-M2/fixed` or its `--mode realtime` counterpart. Set `THRUM_ASSET_STORE` to this example's
cooked store to include the float. Fixed mode submits six declared views at an explicit synthetic
time. Then run the same command with `--package-only` outside the lock. Realtime mode pans the camera
using actual elapsed time and retains every admitted submitted image in a hashed, lossless spool,
with time records and an encoded review movie. Its bounded three-second capture has a 3 GiB pixel
reserve and a 6 GiB raw-spool allowance; these are observer resources, absent from ordinary frames.
Neither route
captures the desktop or proves physical display cadence.

For a portable Metal comparison, set `THRUM_RECIPE=portable THRUM_WATER_RECIPE=portable`
on the fixed capture.

`python3 scripts/water_cost.py record --scene first-frame`, also under the GPU lock, records the
paired capture-off run. `check` takes a fresh measurement and validates the committed record;
timings themselves are investigation tripwires rather than pass thresholds. The cost tool cooks
the calibration float and keeps it in both arms at identical world times, so only water differs.
