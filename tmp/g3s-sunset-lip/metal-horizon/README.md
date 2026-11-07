# Isolated Metal horizon diagnostic

This WIP is based on 9d440836. It holds diagnostic source and raw outputs
outside the shipping tree. The WGSL uses a Depth32Float fixture written from
terrain-only reference depths. It does not execute in the showcase frame
graph and does not include dynamic casters, the shadow raster draw, forward
shading or presentation.

From this branch's checkout root, regenerate the inputs and run the proof:

```sh
cd tmp/g3s-sunset-lip
python3 west-spur-source/generate.py --evidence-dir "$PWD/metal-horizon/source-evidence"
THRUM_HORIZON_MODEL=flat python3 metal-horizon/inputs.py west-spur-source metal-horizon
shasum -a 256 -c metal-horizon/INPUTS.sha256
metal-horizon/run.sh > metal-horizon/metal.log 2>&1
python3 metal-horizon/summarize.py
```

Run with access to the real Mac GPU. The runner injects a temporary test
into the existing Metal depth fixture module and restores the files on exit.
The original run used Python 3.15, Rust 1.99.1 and M3 Max Metal under shared
load. One 32-pair recording lost the two final query pairs. Its raw queries
are preserved. The subsequent 16-pair run has complete data; the summary
discards the first two warm-up pairs. The exact triangle reference and the
flat-raster reference are separate oracles.

The next experiment should reduce per-fragment traversal divergence and
reject skipped intersections against these same rays. A real graph route
must then include moving and empty casters, self-shadow, camera snaps,
dawn/sunset sweeps and paired whole-frame measurements before promotion.
