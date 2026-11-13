# SEA design proofs

Two reproductions run while [the design](../../../plans/SEA/DESIGN.md#check-that-it-can-work) was written, on
13 November 2026. Each is a short CPU script of the core equations with its output beside it. They are evidence
for the design's choices and the source of the bounds in M2, M5 and M6 of [the done-when file](../../../plans/SEA/DONE.json).
They are not engine code and no engine test depends on them.

| Script | Output | What it shows |
|---|---|---|
| `wake_proof.py` | `wake-proof.txt` | An FFT eWave on body-following windows gives the Kelvin wake's transverse wavelength at two speeds, a crest that travels on after the source is removed, and two banded levels that sum to one fine level; a constant-speed wave equation and two unbanded levels fail |
| `long_wave_precision.py` | `long-wave-precision.txt` | f32 evaluation of 192 long-wave components stays within 0.05 mm of f64 when the phase is reduced in f64 first, and is 18 mm off when it is not |

Run either with `uv run --with numpy python <script>` from the repository root. The wake script takes a few
minutes.
