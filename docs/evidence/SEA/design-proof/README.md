# SEA design proofs

Four reproductions run while [the design](../../../plans/SEA/DESIGN.md#check-that-it-can-work) was written and
revised, on 13 and 14 November 2026. Each is a short CPU script of the core equations with its output beside it. They are evidence
for the design's choices and the source of the bounds in M2, M5, M6 and M8 of [the done-when file](../../../plans/SEA/DONE.json).
They are not engine code and no engine test depends on them.

| Script | Output | What it shows |
|---|---|---|
| `wake_proof.py` | `wake-proof.txt` | An FFT eWave on body-following windows gives the Kelvin wake's transverse wavelength at two speeds, a crest that travels on after the source is removed, and two banded levels that sum to one fine level; a constant-speed wave equation and two unbanded levels fail |
| `long_wave_precision.py` | `long-wave-precision.txt` | f32 evaluation of 192 long-wave components stays within 0.05 mm of f64 when the phase is reduced in f64 first, and is 18 mm off when it is not |
| `revision_proof.py` | `revision-proof.txt` | Five parts, for the mechanisms changed after [the design review](../design-review.md). A: a wake step of several ticks against single ticks. B: the display blend across a span of several ticks. C: the receiving rectangles against rays reflected and refracted on the CPU. D: the foam transfer chain with no birth. E: copied scene colour turned back into radiance |
| `first_principles_proof.py` | `first-principles-proof.txt` | Two parts, for the mechanisms changed when the design was judged again from first principles. F: cascade phase after 24 hours, from f32 seconds and from a whole-cycle loop with integer phase. G: a wave evaluated at the display's time against a blend of two tick endpoints, and a height carried to a later tick by its rate of change |

Run any of them with `uv run --with numpy python <script>` from the repository root. The wake script takes a few
minutes and the revision script about one.
