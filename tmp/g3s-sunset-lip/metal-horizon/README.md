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
mkdir -p metal-horizon/dda-repro
cp metal-horizon/expected.bin metal-horizon/dda-repro/
THRUM_HORIZON_OUTPUT="$PWD/metal-horizon/dda-repro" \
    metal-horizon/run.sh > metal-horizon/dda-repro/metal.log 2>&1
python3 metal-horizon/compare.py dda-repro

python3 metal-horizon/sparse.py
THRUM_HORIZON_INPUT="$PWD/metal-horizon/sparse-run" \
    THRUM_HORIZON_OUTPUT="$PWD/metal-horizon/sparse-run" \
    metal-horizon/run.sh > metal-horizon/sparse-run/metal.log 2>&1
python3 metal-horizon/compare.py sparse-run
```

Run with access to the real Mac GPU. The runner injects a temporary test
into the existing Metal depth fixture module and restores the files on exit.
The original run used Python 3.15, Rust 1.99.1 and M3 Max Metal under shared
load. One 32-pair recording lost the two final query pairs. Its raw queries
are preserved. The subsequent 16-pair run has complete data; the summary
discards the first two warm-up pairs. The exact triangle reference and the
flat-raster reference are separate oracles.

The second experiment compares the original stackless depth-first traversal
with a hierarchical digital differential analyser. The latter tracks the
first cell in each remaining ray interval and steps across integer tile
boundaries. It retains the signed slope bounds and skips the same subtrees.
Run `compare.py dda-run` and `compare.py sparse-run` against the retained
results. The terrain and sparse outputs agree with DFS and the CPU oracles.
In the six measured permutations under shared load, the DDA traversal
was faster than DFS in every pair. Reduction plus DDA still cost more
than legacy taps at median. A first run had a missing final timestamp
pair; the witness adds an untimed trailing dispatch to separate the
final timed pass from the resolve. No conclusion about production
timing follows from these short compute runs.

Next use the game graph. It must cover the actual caster raster, moving
casters, self-shadow, camera snaps, dawn/sunset sweeps and paired whole
frames before promotion.
